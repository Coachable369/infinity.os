/* Executes the production AP bootstrap after ExitBootServices, not a host mock. */
#define efi_main unused_loader_main
#include "../../../boot/common/uefi_loader.c"
#undef efi_main
int _fltused;
static volatile uint64_t observed[8];
static uint8_t memory_map[131072];

// ------------------------=
// FUNC: finish
// DESC: Publishes the binary guest test result through QEMU's dedicated exit device.
// ------------------=
static void finish(uint32_t code) {
    __asm__ volatile("outl %0, %1" :: "a"(code), "Nd"((uint16_t)0xf4));
    for (;;) __asm__ volatile("cli; hlt");
}
// ------------------------=
// FUNC: worker
// DESC: Verifies independent argument/stack delivery and floating-point execution on each AP.
// ------------------=
static void EFIAPI worker(void *argument) {
    size_t id = (size_t)argument;
    volatile double value = 1.25;
    for (size_t i = 0; i < 32; ++i) value += 0.25;
    if (id && id <= 8 && value == 9.25) observed[id - 1] = id * 17;
}
// ------------------------=
// FUNC: efi_main
// DESC: Exits firmware, switches to kernel paging, and verifies three native secondary CPUs.
// ------------------=
EFI_STATUS EFIAPI efi_main(EFI_HANDLE image, EFI_SYSTEM_TABLE *system) {
    __asm__ volatile("outb %0, %1" :: "a"((uint8_t)1), "Nd"((uint16_t)0xe9));
    uint64_t cr3 = prepare_identity_map(system);
    InfinityBootInfo info = {0};
    prepare_payloads(image, system, &info, 0);
    if (info.payload_bridge != (uint64_t)(uintptr_t)payload_cached_read) finish(7);
    X86Workers *bridge = (X86Workers *)(uintptr_t)x86_worker_bridge(system, cr3);
    if (!bridge || bridge->version != 2 || !bridge->hz) finish(1);
    __asm__ volatile("outb %0, %1" :: "a"((uint8_t)2), "Nd"((uint16_t)0xe9));
    size_t size = sizeof(memory_map), descriptor_size = 0;
    uint64_t key = 0;
    uint32_t version = 0;
    if (system->boot_services->get_memory_map(&size, memory_map, &key, &descriptor_size, &version)) finish(2);
    if (system->boot_services->exit_boot_services(image, key)) finish(3);
    __asm__ volatile("outb %0, %1" :: "a"((uint8_t)3), "Nd"((uint16_t)0xe9));
    __asm__ volatile("cli; mov %0, %%cr3" :: "r"(cr3) : "memory");
    /* Poison firmware handles: the capability must now use only reserved RAM. */
    payload_root = NULL; payload_file = NULL;
    uint8_t copied[32];
    if (payload_cached_read(0, 4075, 24, copied)) finish(8);
    for (size_t i = 0; i < 24; ++i) if (copied[i] != (4075 + i) % 251) finish(8);
    memset(copied, 0xa5, sizeof(copied));
    if (!payload_cached_read(0, 4090, 32, copied) ||
        !payload_cached_read(2, 0, 1, copied) ||
        !payload_cached_read(0, UINT64_MAX, 1, copied)) finish(9);
    for (size_t i = 0; i < sizeof(copied); ++i) if (copied[i] != 0xa5) finish(9);
    if (bridge->start(worker, 3) != 3) finish(4);
    uint64_t begin = x86_ticks();
    while ((observed[0] != 17 || observed[1] != 34 || observed[2] != 51) &&
           x86_ticks() - begin < bridge->hz) __asm__ volatile("pause");
    if (observed[0] != 17 || observed[1] != 34 || observed[2] != 51) finish(5);
    if (bridge->start(worker, 3) != 0) finish(6);
    if (payload_cached_read(1, 0, 32, copied)) finish(10);
    for (size_t i = 0; i < 32; ++i) if (copied[i] != (uint8_t)(255 - i)) finish(10);
    finish(16);
    return 0;
}
