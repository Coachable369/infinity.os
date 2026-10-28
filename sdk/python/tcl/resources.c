#include "tclInt.h"

typedef struct { const char *path; const unsigned char *bytes; size_t size; } BundledResource;
#include "tcl-resources.inc"
typedef struct { const BundledResource *entry; size_t offset; Tcl_Channel channel; } ResourceReader;
static unsigned channel_serial;
static int registered;

// ------------------------=
// FUNC: lookup
// DESC: Finds an immutable upstream library resource by exact absolute path.
// ------------------=
static const BundledResource *lookup(const char *path) {
    size_t lo = 0, hi = sizeof(resources) / sizeof(resources[0]);
    while (lo < hi) {
        size_t middle = lo + (hi - lo) / 2;
        int order = strcmp(path, resources[middle].path);
        if (!order) return &resources[middle];
        if (order < 0) hi = middle; else lo = middle + 1;
    }
    return NULL;
}
// ------------------------=
// FUNC: directory_exists
// DESC: Derives a bundled directory from actual resource paths, not a fabricated filesystem entry.
// ------------------=
static int directory_exists(const char *path) {
    size_t length = strlen(path);
    for (size_t i = 0; i < sizeof(resources) / sizeof(resources[0]); ++i) {
        if (!strncmp(path, resources[i].path, length) && resources[i].path[length] == '/') return 1;
    }
    return 0;
}
// ------------------------=
// FUNC: path_in_filesystem
// DESC: Claims only the two bundled Tcl/Tk library trees; user files remain in the native capability namespace.
// ------------------=
static int path_in_filesystem(Tcl_Obj *path, ClientData *internal) {
    (void)internal;
    const char *name = Tcl_GetString(path);
    const char *roots[] = { "/system/python/tcl8.6", "/system/python/tk8.6" };
    for (size_t i = 0; i < sizeof(roots) / sizeof(roots[0]); ++i) {
        size_t length = strlen(roots[i]);
        if (!strncmp(name, roots[i], length) && (!name[length] || name[length] == '/')) return TCL_OK;
    }
    return -1;
}
// ------------------------=
// FUNC: separator
// DESC: Provides the same slash separator as the native object namespace.
// ------------------=
static Tcl_Obj *separator(Tcl_Obj *path) { (void)path; return Tcl_NewStringObj("/", 1); }
// ------------------------=
// FUNC: resource_stat
// DESC: Reports actual bundled lengths and read-only modes, with ENOENT for absent resources.
// ------------------=
static int resource_stat(Tcl_Obj *path, Tcl_StatBuf *info) {
    const char *name = Tcl_GetString(path);
    const BundledResource *entry = lookup(name);
    int directory = !entry && directory_exists(name);
    if (!entry && !directory) { errno = ENOENT; return -1; }
    memset(info, 0, sizeof(*info));
    info->st_mode = directory ? S_IFDIR | 0555 : S_IFREG | 0444;
    info->st_nlink = 1;
    info->st_size = entry ? (off_t)entry->size : 0;
    return 0;
}
// ------------------------=
// FUNC: resource_access
// DESC: Enforces immutable library access before a channel is opened.
// ------------------=
static int resource_access(Tcl_Obj *path, int mode) {
    Tcl_StatBuf info;
    if (resource_stat(path, &info) < 0) return -1;
    if ((mode & W_OK) || ((mode & X_OK) && !S_ISDIR(info.st_mode))) { errno = EACCES; return -1; }
    return 0;
}
// ------------------------=
// FUNC: reader_close
// DESC: Releases the per-channel cursor while leaving immutable resources untouched.
// ------------------=
static int reader_close(ClientData data, Tcl_Interp *interp) { (void)interp; ckfree(data); return 0; }
// ------------------------=
// FUNC: reader_input
// DESC: Copies a bounded slice from the exact bundled Tcl/Tk resource.
// ------------------=
static int reader_input(ClientData data, char *buffer, int count, int *error) {
    ResourceReader *reader = data;
    if (count < 0) { *error = EINVAL; return -1; }
    size_t length = (size_t)count;
    if (length > reader->entry->size - reader->offset) length = reader->entry->size - reader->offset;
    memcpy(buffer, reader->entry->bytes + reader->offset, length); reader->offset += length;
    return (int)length;
}
// ------------------------=
// FUNC: reader_seek
// DESC: Updates the resource cursor only for a valid in-bounds seek target.
// ------------------=
static int reader_seek(ClientData data, long offset, int mode, int *error) {
    ResourceReader *reader = data;
    int64_t origin = mode == SEEK_SET ? 0 : mode == SEEK_CUR ? (int64_t)reader->offset :
        mode == SEEK_END ? (int64_t)reader->entry->size : -1;
    if (origin < 0 || offset < -origin || offset > (int64_t)reader->entry->size - origin) {
        *error = EINVAL; return -1;
    }
    reader->offset = (size_t)(origin + offset); return (int)reader->offset;
}
// ------------------------=
// FUNC: reader_watch
// DESC: Notifies Tcl of actual immediately readable immutable content without native descriptor polling.
// ------------------=
static void reader_watch(ClientData data, int mask) {
    ResourceReader *reader = data;
    if ((mask & TCL_READABLE) && reader->channel) Tcl_NotifyChannel(reader->channel, TCL_READABLE);
}
static const Tcl_ChannelType resource_channel = {
    .typeName = "infinityBundledLibrary", .version = TCL_CHANNEL_VERSION_1,
    .closeProc = reader_close, .inputProc = reader_input, .seekProc = reader_seek,
    .watchProc = reader_watch,
};
// ------------------------=
// FUNC: resource_open
// DESC: Opens a read-only channel over verified bundled data; no temporary extraction is involved.
// ------------------=
static Tcl_Channel resource_open(Tcl_Interp *interp, Tcl_Obj *path, int mode, int permissions) {
    (void)permissions;
    const BundledResource *entry = lookup(Tcl_GetString(path));
    if (!entry || (mode & (O_WRONLY | O_RDWR | O_CREAT | O_TRUNC | O_APPEND))) {
        errno = entry ? EROFS : ENOENT;
        if (interp) Tcl_SetObjResult(interp, Tcl_NewStringObj(Tcl_PosixError(interp), -1));
        return NULL;
    }
    ResourceReader *reader = ckalloc(sizeof(*reader));
    reader->entry = entry; reader->offset = 0; reader->channel = NULL;
    char name[48]; snprintf(name, sizeof(name), "infinityLibrary%u", ++channel_serial);
    reader->channel = Tcl_CreateChannel(&resource_channel, name, reader, TCL_READABLE);
    return reader->channel;
}
// ------------------------=
// FUNC: resource_match
// DESC: Enumerates actual direct library children using Tcl's own glob pattern matching.
// ------------------=
static int resource_match(Tcl_Interp *interp, Tcl_Obj *result, Tcl_Obj *directory,
    const char *pattern, Tcl_GlobTypeData *types) {
    const char *parent = Tcl_GetString(directory);
    size_t length = strlen(parent);
    if (!pattern) {
        Tcl_StatBuf info;
        if (resource_stat(directory, &info) < 0) return TCL_OK;
        if (types && types->type && !(types->type &
            (S_ISDIR(info.st_mode) ? TCL_GLOB_TYPE_DIR : TCL_GLOB_TYPE_FILE))) return TCL_OK;
        return Tcl_ListObjAppendElement(interp, result, directory);
    }
    while (length > 1 && parent[length - 1] == '/') --length;
    char previous[512] = {0};
    for (size_t i = 0; i < sizeof(resources) / sizeof(resources[0]); ++i) {
        const char *path = resources[i].path;
        if (strncmp(parent, path, length) || path[length] != '/') continue;
        const char *child = path + length + 1, *slash = strchr(child, '/');
        size_t part_length = slash ? (size_t)(slash - child) : strlen(child);
        if (!part_length || length + 1 + part_length >= sizeof(previous)) continue;
        char name[512]; memcpy(name, child, part_length); name[part_length] = 0;
        if (pattern && !Tcl_StringMatch(name, pattern)) continue;
        if (types && types->type && !(types->type & (slash ? TCL_GLOB_TYPE_DIR : TCL_GLOB_TYPE_FILE))) continue;
        char full[512]; memcpy(full, path, length + 1 + part_length); full[length + 1 + part_length] = 0;
        if (!strcmp(full, previous)) continue;
        strcpy(previous, full);
        if (Tcl_ListObjAppendElement(interp, result, Tcl_NewStringObj(full, -1)) != TCL_OK) return TCL_ERROR;
    }
    return TCL_OK;
}
static const Tcl_Filesystem resource_filesystem = {
    .typeName = "infinityBundledLibrary", .structureLength = sizeof(Tcl_Filesystem),
    .version = TCL_FILESYSTEM_VERSION_1, .pathInFilesystemProc = path_in_filesystem,
    .filesystemSeparatorProc = separator, .statProc = resource_stat,
    .accessProc = resource_access, .openFileChannelProc = resource_open,
    .matchInDirectoryProc = resource_match,
};
// ------------------------=
// FUNC: InfinityTclRegisterResources
// DESC: Registers the immutable library once per native interpreter process before Tcl_Init reads init.tcl.
// ------------------=
int InfinityTclRegisterResources(void) {
    if (registered) return TCL_OK;
    int status = Tcl_FSRegister(NULL, &resource_filesystem);
    if (status == TCL_OK) registered = 1;
    return status;
}
