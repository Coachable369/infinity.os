#if defined(__aarch64__)
#define INFINITY_AARCH64 1
#endif
#include "../boot/common/uefi_loader.c"
#include <assert.h>
#include <stdlib.h>

static EFI_FILE_PROTOCOL test_root, test_file;
static EFI_LOADED_IMAGE_PROTOCOL test_loaded;
static EFI_SIMPLE_FILE_SYSTEM_PROTOCOL test_fs;
static unsigned calls, fail_at, roots_closed, files_closed;
static uint8_t present;
static unsigned range_calls, range_frees, range_mode;
static unsigned image_calls, image_frees, image_fail, image_high_map;
static uint64_t image_ceiling = UINT64_MAX;
// ------------------------=
// FUNC: test_image_map
// DESC: Exposes high conventional RAM beside reserved RAM to verify explicit safe staging placement.
// ------------------=
static EFI_STATUS EFIAPI test_image_map(size_t *bytes, void *buffer, uint64_t *key, size_t *stride, uint32_t *version) {
    assert(*bytes >= 80); *bytes = 80; *stride = 40; *key = 1; *version = 1;
    uint64_t entries[10] = {7, UINT64_C(0x200000000), 0, 2, 0,
                            0, UINT64_C(0x300000000), 0, 100, 0};
    memcpy(buffer, entries, sizeof(entries)); return EFI_SUCCESS;
}
// ------------------------=
// FUNC: test_image_allocate
// DESC: Requires unrestricted high-memory staging and models allocation failure without returning a bogus image.
// ------------------=
static EFI_STATUS EFIAPI test_image_allocate(uint32_t kind, uint32_t type, size_t pages, uint64_t *address) {
    assert(type == EFI_LOADER_DATA && pages == 2);
    assert(kind == (image_high_map ? EFI_ALLOCATE_ADDRESS : EFI_ALLOCATE_MAX_ADDRESS));
    assert(*address == (image_high_map ? UINT64_C(0x200000000) : image_ceiling));
    ++image_calls;
    *address = image_ceiling == UINT32_MAX ? UINT64_C(0xffffe000) : UINT64_C(0x200000000);
    return image_fail ? 1 : EFI_SUCCESS;
}
// ------------------------=
// FUNC: test_image_free
// DESC: Checks staging pages above 4 GiB are released with the exact rounded allocation size.
// ------------------=
static EFI_STATUS EFIAPI test_image_free(uint64_t address, size_t pages) {
    assert(address == UINT64_C(0x200000000) && pages == 2);
    ++image_frees; return EFI_SUCCESS;
}
// ------------------------=
// FUNC: test_range_allocate
// DESC: Models firmware refusing one allocation across adjacent conventional-memory descriptors.
// ------------------=
static EFI_STATUS EFIAPI test_range_allocate(uint32_t kind, uint32_t type, size_t pages, uint64_t *address) {
    assert(kind == EFI_ALLOCATE_ADDRESS && type == EFI_LOADER_DATA);
    ++range_calls;
    if (pages == 4 || (range_mode == 1 && *address == UINT64_C(0x100000000))) return 1;
    assert(pages == 2);
    return EFI_SUCCESS;
}
// ------------------------=
// FUNC: test_range_free
// DESC: Verifies rollback frees exactly the first successful reservation.
// ------------------=
static EFI_STATUS EFIAPI test_range_free(uint64_t address, size_t pages) {
    assert(address == UINT64_C(0xffffe000) && pages == 2);
    ++range_frees; return EFI_SUCCESS;
}
// ------------------------=
// FUNC: test_range_map
// DESC: Supplies unordered descriptors with an optional reserved-memory hole.
// ------------------=
static EFI_STATUS EFIAPI test_range_map(size_t *bytes, void *buffer, uint64_t *key, size_t *stride, uint32_t *version) {
    assert(*bytes >= 80); *bytes = 80; *stride = 40; *key = 1; *version = 1;
    uint64_t entries[10] = {7, UINT64_C(0x100000000), 0, 2, 0,
                            7, UINT64_C(0xffffe000), 0, 2, 0};
    if (range_mode == 2) entries[0] = 0;
    memcpy(buffer, entries, sizeof(entries)); return EFI_SUCCESS;
}

// ------------------------=
// FUNC: test_close
// DESC: Tracks release of firmware file handles.
// ------------------=
static EFI_STATUS EFIAPI test_close(EFI_FILE_PROTOCOL *file) {
    if (file == &test_root) ++roots_closed; else ++files_closed;
    return EFI_SUCCESS;
}
// ------------------------=
// FUNC: test_open
// DESC: Supplies a local media kernel or an installed ESP without one.
// ------------------=
static EFI_STATUS EFIAPI test_open(EFI_FILE_PROTOCOL *root, EFI_FILE_PROTOCOL **file, CHAR16 *path, uint64_t mode, uint64_t attributes) {
    (void)attributes;
    assert(root == &test_root && mode == EFI_FILE_MODE_READ);
    const CHAR16 expected[] = L"\\EFI\\INFINITY\\KERNEL.ELF";
    for (size_t i = 0; i < sizeof(expected)/sizeof(expected[0]); ++i) assert(path[i] == expected[i]);
    if (!present) return 1;
    *file = &test_file;
    return EFI_SUCCESS;
}
// ------------------------=
// FUNC: test_volume
// DESC: Returns the source image filesystem root.
// ------------------=
static EFI_STATUS EFIAPI test_volume(EFI_SIMPLE_FILE_SYSTEM_PROTOCOL *fs, EFI_FILE_PROTOCOL **root) {
    assert(fs == &test_fs);
    if (fail_at == 3) return 1;
    *root = &test_root;
    return EFI_SUCCESS;
}
// ------------------------=
// FUNC: test_protocol
// DESC: Models firmware protocol discovery failures independently.
// ------------------=
static EFI_STATUS EFIAPI test_protocol(EFI_HANDLE handle, EFI_GUID *guid, void **out) {
    (void)handle; (void)guid;
    ++calls;
    if (calls == fail_at) return 1;
    *out = calls == 1 ? (void *)&test_loaded : (void *)&test_fs;
    return EFI_SUCCESS;
}
// ------------------------=
// FUNC: infinity_handoff
// DESC: Prevents privileged guest handoff in the host fixture.
// ------------------=
void EFIAPI infinity_handoff(InfinityBootInfo *info, uint64_t a, uint64_t b, uint64_t c) {
    (void)info; (void)a; (void)b; (void)c; abort();
}
// ------------------------=
// FUNC: infinity_ap_entry
// DESC: Prevents guest CPU entry in the host fixture.
// ------------------=
void infinity_ap_entry(void) { abort(); }
// ------------------------=
// FUNC: main
// DESC: Verifies media detection, installed fallback and balanced firmware handle cleanup.
// ------------------=
int main(void) {
    assert(!installed_kernel_size_valid(sizeof(Elf64Header)-1));
    assert(installed_kernel_size_valid(sizeof(Elf64Header)));
    assert(installed_kernel_size_valid(UINT64_C(802)*1024*1024));
    assert(installed_kernel_size_valid(UINT64_C(1024)*1024*1024));
    assert(!installed_kernel_size_valid(UINT64_C(1024)*1024*1024+1));
    EFI_BOOT_SERVICES boot = {0};
    boot.allocate_pages = test_image_allocate; boot.free_pages = test_image_free;
    assert(!allocate_kernel_image(&boot, 0));
    assert(!allocate_kernel_image(&boot, SIZE_MAX));
    assert(image_calls == 0);
    void *staging = allocate_kernel_image(&boot, PAGE_SIZE + 1);
    assert((uintptr_t)staging == UINT64_C(0x200000000));
    free_kernel_image(&boot, staging, PAGE_SIZE + 1);
    assert(image_calls == 1 && image_frees == 1);
    image_fail = 1;
    assert(!allocate_kernel_image(&boot, PAGE_SIZE + 1));
    assert(image_calls == 2 && image_frees == 1);
    image_fail = 0; image_high_map = 1; boot.get_memory_map = test_image_map;
    staging = allocate_kernel_image(&boot, PAGE_SIZE + 1);
    assert((uintptr_t)staging == UINT64_C(0x200000000));
    free_kernel_image(&boot, staging, PAGE_SIZE + 1);
    assert(image_calls == 3 && image_frees == 2);
    image_high_map = 0; image_ceiling = UINT32_MAX;
    assert((uintptr_t)allocate_native_buffer(&boot, PAGE_SIZE + 1, image_ceiling) == UINT64_C(0xffffe000));
    assert(image_calls == 4);
    boot.allocate_pages = test_range_allocate; boot.free_pages = test_range_free;
    boot.get_memory_map = test_range_map;
    for (range_mode = 0; range_mode < 3; ++range_mode) {
        range_calls = range_frees = 0;
        assert(reserve_kernel_range(&boot, UINT64_C(0xffffe000), UINT64_C(0x100002000)) == (range_mode == 0));
        assert(range_calls == (range_mode == 2 ? 2u : 3u));
        assert(range_frees == (range_mode != 0));
    }
    EFI_SYSTEM_TABLE system = {0};
    boot.handle_protocol = test_protocol;
    system.boot_services = &boot;
    test_fs.open_volume = test_volume;
    test_root.open = test_open;
    test_root.close = test_close;
    test_file.close = test_close;
    for (unsigned failure = 0; failure <= 3; ++failure) {
        for (unsigned found = 0; found <= 1; ++found) {
            calls = roots_closed = files_closed = 0;
            fail_at = failure; present = found;
            assert(boot_media_has_kernel(NULL, &system) == (failure == 0 && found));
            assert(roots_closed == (failure == 0));
            assert(files_closed == (failure == 0 && found));
        }
    }
    return 0;
}
