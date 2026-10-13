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
    .data : ALIGN(0x1000) { *(.data .data.*) } :data
    .kokoro_init_array : ALIGN(8) { KEEP(*(.kokoro_init_array)) } :data
    .browser_init_array : ALIGN(8) { KEEP(*(.browser_init_array)) } :data
    .init_array : ALIGN(8) { KEEP(*(.init_array .init_array.*)) } :data
    .fini_array : ALIGN(8) { KEEP(*(.fini_array .fini_array.*)) } :data
    .got : ALIGN(0x1000) { *(.got .got.*) } :data
    /* Consume the private speech arena before the ordinary BSS wildcard.
       VirtualBox reserves 0xe0000000..0xffffffff for devices, not RAM. */
    __low_bss = ALIGN(0x1000);
    .speech_heap 0x100000000 (NOLOAD) : ALIGN(0x1000) {
        *private-native.o(.bss.heap)
    } :speech
    . = __low_bss;
    .bss : ALIGN(0x1000) { *(.bss .bss.*) *(COMMON) } :data
    ASSERT(. <= 0xe0000000, "ARM kernel overlaps VirtualBox device aperture")
    ASSERT(SIZEOF(.speech_heap) == 0x60000000, "ARM speech arena must reside in high RAM")
    /DISCARD/ : { *(.eh_frame*) *(.comment*) }
}
