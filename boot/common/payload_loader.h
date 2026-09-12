/* ARM64 retains firmware services. Large files are split into 512 MiB shards,
 * independent of total payload length and FAT's individual-file size limit. */
#if defined(INFINITY_AARCH64)
static EFI_FILE_PROTOCOL *payload_root;
static EFI_FILE_PROTOCOL *payload_file;
static uint32_t payload_kind = UINT32_MAX, payload_part = UINT32_MAX;

// ------------------------=
// FUNC: payload_read
// DESC: Reads a bounded logical payload range across independently seekable shards.
// ------------------=
static uint64_t EFIAPI payload_read(uint32_t kind, uint64_t offset, size_t length, void *destination) {
    if (!payload_root || kind > 3 || length > 1024 * 1024) return 1;
    uint8_t *out = destination;
    while (length) {
        uint64_t part64 = offset / UINT64_C(536870912);
        if (part64 > 999) return 1;
        uint32_t part = (uint32_t)part64;
        if (!payload_file || kind != payload_kind || part != payload_part) {
            if (payload_file) payload_file->close(payload_file);
            payload_file = NULL;
            CHAR16 path[] = L"\\EFI\\INFINITY\\PAYLOAD\\P0-000.BIN";
            path[23] = (CHAR16)('0' + kind);
            path[25] = (CHAR16)('0' + part / 100);
            path[26] = (CHAR16)('0' + part / 10 % 10);
            path[27] = (CHAR16)('0' + part % 10);
            if (payload_root->open(payload_root, &payload_file, path, EFI_FILE_MODE_READ, 0) != EFI_SUCCESS) return 1;
            payload_kind = kind; payload_part = part;
        }
        uint64_t within = offset % UINT64_C(536870912);
        size_t count = length;
        if (count > UINT64_C(536870912) - within) count = (size_t)(UINT64_C(536870912) - within);
        size_t actual = count;
        if (payload_file->set_position(payload_file, within) != EFI_SUCCESS ||
            payload_file->read(payload_file, &actual, out) != EFI_SUCCESS || actual != count) return 1;
        offset += count; length -= count; out += count;
    }
    return 0;
}
#endif

// ------------------------=
// FUNC: prepare_payloads
// DESC: Exposes streaming installation reads and reserves the installed model arena before handoff.
// ------------------=
static void prepare_payloads(EFI_HANDLE image, EFI_SYSTEM_TABLE *system, InfinityBootInfo *info, uint8_t installed) {
#if defined(INFINITY_AARCH64)
    EFI_LOADED_IMAGE_PROTOCOL *loaded = NULL;
    EFI_SIMPLE_FILE_SYSTEM_PROTOCOL *filesystem = NULL;
    EFI_GUID guid = loaded_image_guid;
    if (system->boot_services->handle_protocol(image, &guid, (void **)&loaded) != EFI_SUCCESS) return;
    guid = simple_fs_guid;
    if (system->boot_services->handle_protocol(loaded->device_handle, &guid, (void **)&filesystem) != EFI_SUCCESS ||
        filesystem->open_volume(filesystem, &payload_root) != EFI_SUCCESS) return;
    info->payload_bridge = (uint64_t)(uintptr_t)payload_read;
    uint8_t probe[4];
    if (!installed || payload_read(2, 0, sizeof(probe), probe) != 0) return;
    if (!equal_bytes(probe, (const uint8_t *)"GGUF", 4))
        fail(system, L"Invalid native model payload\r\n", "Invalid native model payload\n");
    const uint8_t second_model = payload_read(3, 0, sizeof(probe), probe) == 0;
    if (second_model && !equal_bytes(probe, (const uint8_t *)"GGUF", 4))
        fail(system, L"Invalid Ministral model payload\r\n", "Invalid Ministral model payload\n");
    const uint64_t model_bytes = UINT64_C(5027783488) + (second_model ? UINT64_C(2147023008) : 0);
    const uint64_t work_bytes = UINT64_C(1280) * 1024 * 1024 * (second_model ? 2 : 1);
    uint64_t model = 0, work = 0;
    if (system->boot_services->allocate_pages(EFI_ALLOCATE_ANY_PAGES, EFI_LOADER_DATA,
            (size_t)((model_bytes + PAGE_MASK) / PAGE_SIZE), &model) != EFI_SUCCESS ||
        system->boot_services->allocate_pages(EFI_ALLOCATE_ANY_PAGES, EFI_LOADER_DATA,
            (size_t)(work_bytes / PAGE_SIZE), &work) != EFI_SUCCESS)
        fail(system, L"Native Qwen requires more available RAM\r\n", "Native Qwen model arena allocation failed\n");
    serial_write("[BOOT] loading native Qwen3-8B payload\n");
    for (uint64_t offset = 0; offset < model_bytes;) {
        size_t count = 1024 * 1024;
        uint32_t kind = offset < UINT64_C(5027783488) ? 2 : 3;
        uint64_t source_offset = kind == 2 ? offset : offset - UINT64_C(5027783488);
        uint64_t remaining = kind == 2 ? UINT64_C(5027783488) - offset : model_bytes - offset;
        if (count > remaining) count = (size_t)remaining;
        if (payload_read(kind, source_offset, count, (void *)(uintptr_t)(model + offset)) != 0)
            fail(system, L"Native model payload is incomplete\r\n", "Native model payload read failed\n");
        offset += count;
    }
    info->model_address = model; info->model_bytes = model_bytes;
    info->model_work_address = work; info->model_work_bytes = work_bytes;
#else
    (void)image; (void)system; (void)info; (void)installed;
#endif
}
