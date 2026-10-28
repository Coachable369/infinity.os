#include <infinity/app.h>
#include <infinity/syscall.h>

extern int infinity_app_entry(const InfinityAppApi *);

// ------------------------=
// FUNC: stream_write
// DESC: Writes a checked stdout or stderr descriptor through the existing native console syscall.
// ------------------=
static ptrdiff_t stream_write(void *context, int handle, const unsigned char *bytes, size_t length) {
    (void)context;
    if (handle != 1 && handle != 2) return -9;
    size_t written = 0;
    while (written < length) {
        size_t count = length - written;
        if (count > 4096) count = 4096;
        int64_t result = infinity_system_call(INFINITY_SYS_CONSOLE_WRITE,
            (uintptr_t)(bytes + written), count, (uint64_t)handle, 0, 0, 0);
        if (result <= 0 || (uint64_t)result > count) return written ? (ptrdiff_t)written : -1;
        written += (size_t)result;
    }
    return (ptrdiff_t)written;
}

// ------------------------=
// FUNC: console_write
// DESC: Sends bounded output through a native syscall; this callback and its table live entirely in user memory.
// ------------------=
static int console_write(void *context, const unsigned char *bytes, size_t length) {
    return length > 0x7fffffffU ? -1 : (int)stream_write(context, 1, bytes, length);
}

// ------------------------=
// FUNC: stream_read
// DESC: Reads native stdin and allows the supervisor to park this process until input, EOF or cancellation.
// ------------------=
static ptrdiff_t stream_read(void *context, int handle, unsigned char *bytes, size_t length) {
    (void)context;
    if (handle != 0) return -9;
    if (length > 1024) length = 1024;
    for (;;) {
        int64_t result = infinity_system_call(INFINITY_SYS_CONSOLE_READ,
            (uintptr_t)bytes, length, 0, 0, 0, 0);
        if (result != -11) return (ptrdiff_t)result;
    }
}

// ------------------------=
// FUNC: stream_open
// DESC: Rejects ambient file opening because this launch grants only standard terminal streams.
// ------------------=
static int stream_open(void *context, const unsigned char *path, size_t length, uint32_t flags) {
    (void)context; (void)path; (void)length; (void)flags;
    return -13;
}

// ------------------------=
// FUNC: stream_seek
// DESC: Reports that native terminal streams do not support seeking.
// ------------------=
static int64_t stream_seek(void *context, int handle, int64_t offset, uint32_t origin) {
    (void)context; (void)handle; (void)offset; (void)origin;
    return -29;
}

// ------------------------=
// FUNC: stream_flush
// DESC: Confirms only unbuffered standard descriptors; unknown handles cannot be used as objects.
// ------------------=
static int stream_flush(void *context, int handle) {
    (void)context;
    return handle >= 0 && handle <= 2 ? 0 : -9;
}

// ------------------------=
// FUNC: stream_close
// DESC: Closes the caller's standard FILE bookkeeping without revoking another invocation's terminal grant.
// ------------------=
static int stream_close(void *context, int handle) {
    return stream_flush(context, handle);
}

// ------------------------=
// FUNC: infinity_app_start
// DESC: Starts ordinary main inside hardware-isolated user mode and exits via the supervisor, never through a kernel callback.
// ------------------=
__attribute__((noreturn)) void infinity_app_start(void) {
    const InfinityObjectApi streams = {stream_open, stream_read, stream_write, stream_seek, stream_flush, stream_close};
    const InfinityAppApi api = {INFINITY_APP_ABI, sizeof(InfinityAppApi), 0, console_write, &streams};
    int status = infinity_app_entry(&api);
    infinity_system_call(INFINITY_SYS_EXIT, (uint64_t)(int64_t)status, 0, 0, 0, 0, 0);
    for (;;) infinity_system_call(INFINITY_SYS_YIELD, 0, 0, 0, 0, 0, 0);
}
