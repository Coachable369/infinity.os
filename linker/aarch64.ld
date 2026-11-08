ENTRY(infinity_kernel_entry)

PHDRS
{
    text PT_LOAD FLAGS(5);
    rodata PT_LOAD FLAGS(4);
    data PT_LOAD FLAGS(6);
    speech PT_LOAD FLAGS(6);
    stack PT_GNU_STACK FLAGS(6);
}

SECTIONS
{
    . = 0x10000000;
    .text : ALIGN(0x1000) { *(.text .text.*) } :text
    .rodata : ALIGN(0x1000) { *(.rodata .rodata.*) } :rodata
    .data : ALIGN(0x1000) { *(.data .data.*)
        . = ALIGN(4096);
        infinity_browser_private_browser_data_start = .;
        *(.browser_state_data.*)
        infinity_browser_private_browser_data_end = .;
    } :data
    .kokoro_init_array : ALIGN(8) { KEEP(*(.kokoro_init_array)) } :data
    .browser_init_array : ALIGN(8) { KEEP(*(.browser_init_array)) } :data
    .init_array : ALIGN(8) { KEEP(*(.init_array .init_array.*)) } :data
    .fini_array : ALIGN(8) { KEEP(*(.fini_array .fini_array.*)) } :data
    .got : ALIGN(0x1000) { *(.got .got.*) } :data
    /* Consume the private speech arena before the ordinary BSS wildcard.
       VirtualBox reserves 0xe0000000..0xffffffff for devices, not RAM. */
    __low_bss = ALIGN(0x1000);
    .speech_heap 0x100000000 (NOLOAD) : ALIGN(0x1000) {
        *(.speech_heap)
    } :speech
    . = __low_bss;
    .bss (NOLOAD) : ALIGN(0x1000) { *(.bss .bss.*) *(COMMON)
        /* Canonical ARM retains the browser arena in ordinary low RAM. */
        *(.browser_heap)
        . = ALIGN(4096);
        infinity_browser_private_browser_bss_start = .;
        *(.browser_state_bss.*)
        infinity_browser_private_browser_bss_end = .;
    } :data
    ASSERT(. <= 0xe0000000, "ARM kernel overlaps VirtualBox device aperture")
    ASSERT(SIZEOF(.speech_heap) == 0x60000000, "ARM speech arena must reside in high RAM")
    ASSERT(!DEFINED(infinity_browser_private_infinity_browser_run) ||
           (infinity_browser_private_browser_data_end > infinity_browser_private_browser_data_start &&
            infinity_browser_private_browser_bss_end > infinity_browser_private_browser_bss_start),
           "native browser component lacks restartable state; rebuild it")
    ASSERT(infinity_browser_private_browser_data_end - infinity_browser_private_browser_data_start <= 0x200000, "browser restart checkpoint capacity exceeded")
    /DISCARD/ : { *(.eh_frame*) *(.comment*) }
}
