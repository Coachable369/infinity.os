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
    if (bridge->start(worker, 3) != 3) finish(4);
    uint64_t begin = x86_ticks();
    while ((observed[0] != 17 || observed[1] != 34 || observed[2] != 51) &&
           x86_ticks() - begin < bridge->hz) __asm__ volatile("pause");
    if (observed[0] != 17 || observed[1] != 34 || observed[2] != 51) finish(5);
    if (bridge->start(worker, 3) != 0) finish(6);
    finish(16);
    return 0;
}
