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
    EFI_BOOT_SERVICES boot = {0};
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
