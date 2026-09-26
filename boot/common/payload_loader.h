/* Large files are split into 512 MiB shards,
 * independent of total payload length and FAT's individual-file size limit. */
static EFI_FILE_PROTOCOL *payload_root;
static EFI_FILE_PROTOCOL *payload_file;
static uint32_t payload_kind = UINT32_MAX, payload_part = UINT32_MAX;

// ------------------------=
// FUNC: payload_read
// DESC: Reads a bounded logical payload range across independently seekable shards.
// ------------------=
static uint64_t EFIAPI payload_read(uint32_t kind, uint64_t offset, size_t length, void *destination) {
    if (!payload_root || kind > 4 || length > 1024 * 1024) return 1;
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
#if !defined(INFINITY_AARCH64)
#include "payload_cache.h"

// ------------------------=
// FUNC: cache_install_payloads
// DESC: Reserves bounded installer shards before firmware exit; runtime reads only immutable RAM.
// ------------------=
static void cache_install_payloads(EFI_SYSTEM_TABLE *system, InfinityBootInfo *info) {
    uint64_t total = 0;
    for (uint32_t kind = 0; kind < 2; ++kind) {
        for (uint32_t part = 0; part < PAYLOAD_PARTS; ++part) {
            uint8_t probe;
            if (payload_read(kind, (uint64_t)part * PAYLOAD_PART_BYTES, 1, &probe)) break;
            uint8_t metadata[512];
            size_t size = sizeof(metadata);
            EFI_GUID guid = file_info_guid;
            if (payload_file->get_info(payload_file, &guid, &size, metadata) != EFI_SUCCESS || size < 16)
                fail(system, L"Invalid payload metadata\r\n", "Invalid payload metadata\n");
            uint64_t bytes = read_u64(metadata + 8);
            if (!bytes || bytes > PAYLOAD_PART_BYTES || total + bytes > UINT64_C(8) * 1024 * 1024 * 1024)
                fail(system, L"Installer payload exceeds bounds\r\n", "Installer payload exceeds bounds\n");
            uint64_t address = (uint64_t)IDENTITY_MAP_GIB * 1024 * 1024 * 1024 - 1;
            if (system->boot_services->allocate_pages(EFI_ALLOCATE_MAX_ADDRESS, EFI_LOADER_DATA,
                    (size_t)((bytes + PAGE_MASK) / PAGE_SIZE), &address) != EFI_SUCCESS)
                fail(system, L"Installer payload requires more RAM\r\n", "Installer payload requires more RAM\n");
            for (uint64_t offset = 0; offset < bytes;) {
                size_t count = bytes - offset > 1024 * 1024 ? 1024 * 1024 : (size_t)(bytes - offset);
                if (payload_read(kind, (uint64_t)part * PAYLOAD_PART_BYTES + offset, count,
                        (void *)(uintptr_t)(address + offset)))
                    fail(system, L"Incomplete installer payload\r\n", "Incomplete installer payload\n");
                offset += count;
            }
            payload_cache[kind][part] = (PayloadPart){(const uint8_t *)(uintptr_t)address, bytes};
            total += bytes;
            if (bytes < PAYLOAD_PART_BYTES) break;
        }
    }
    if (total) info->payload_bridge = (uint64_t)(uintptr_t)payload_cached_read;
}
#endif

// ------------------------=
// FUNC: prepare_payloads
// DESC: Exposes streaming installation reads and reserves the installed model arena before handoff.
// ------------------=
static void prepare_payloads(EFI_HANDLE image, EFI_SYSTEM_TABLE *system, InfinityBootInfo *info, uint8_t installed) {
    EFI_LOADED_IMAGE_PROTOCOL *loaded = NULL;
    EFI_SIMPLE_FILE_SYSTEM_PROTOCOL *filesystem = NULL;
    EFI_GUID guid = loaded_image_guid;
    if (system->boot_services->handle_protocol(image, &guid, (void **)&loaded) != EFI_SUCCESS) return;
    guid = simple_fs_guid;
    if (system->boot_services->handle_protocol(loaded->device_handle, &guid, (void **)&filesystem) != EFI_SUCCESS ||
        filesystem->open_volume(filesystem, &payload_root) != EFI_SUCCESS) return;
#if defined(INFINITY_AARCH64)
    info->payload_bridge = (uint64_t)(uintptr_t)payload_read;
#else
    if (!installed) cache_install_payloads(system, info);
#endif
    uint8_t probe[4];
    if (!installed || payload_read(4, 0, sizeof(probe), probe) != 0) return;
    if (!equal_bytes(probe, (const uint8_t *)"GGUF", 4))
        fail(system, L"Invalid native model payload\r\n", "Invalid native model payload\n");
    const uint8_t second_model = payload_read(3, 0, sizeof(probe), probe) == 0;
    if (second_model && !equal_bytes(probe, (const uint8_t *)"GGUF", 4))
        fail(system, L"Invalid Ministral model payload\r\n", "Invalid Ministral model payload\n");
    const uint64_t primary_bytes = UINT64_C(2019373888);
    const uint64_t model_bytes = primary_bytes + (second_model ? UINT64_C(2147023008) : 0);
    const uint64_t work_bytes = UINT64_C(1280) * 1024 * 1024 * (1 + second_model);
    uint64_t model = 0, work = 0;
    uint32_t allocation_type = EFI_ALLOCATE_ANY_PAGES;
#if !defined(INFINITY_AARCH64)
    allocation_type = EFI_ALLOCATE_MAX_ADDRESS;
    model = work = (uint64_t)IDENTITY_MAP_GIB * 1024 * 1024 * 1024 - 1;
#endif
    if (system->boot_services->allocate_pages(allocation_type, EFI_LOADER_DATA,
            (size_t)((model_bytes + PAGE_MASK) / PAGE_SIZE), &model) != EFI_SUCCESS ||
        system->boot_services->allocate_pages(allocation_type, EFI_LOADER_DATA,
            (size_t)(work_bytes / PAGE_SIZE), &work) != EFI_SUCCESS)
        fail(system, L"Native models require more available RAM\r\n", "Native model arena allocation failed\n");
    serial_write("[BOOT] loading native Hermes and optional Ministral payloads\n");
    for (uint64_t offset = 0; offset < model_bytes;) {
        size_t count = 1024 * 1024;
        uint32_t kind = offset < primary_bytes ? 4 : 3;
        uint64_t source_offset = kind == 4 ? offset : offset - primary_bytes;
        uint64_t remaining = kind == 4 ? primary_bytes - offset : model_bytes - offset;
        if (count > remaining) count = (size_t)remaining;
        if (payload_read(kind, source_offset, count, (void *)(uintptr_t)(model + offset)) != 0)
            fail(system, L"Native model payload is incomplete\r\n", "Native model payload read failed\n");
        offset += count;
    }
    info->model_address = model; info->model_bytes = model_bytes;
    info->model_work_address = work; info->model_work_bytes = work_bytes;
}
