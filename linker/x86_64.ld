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
    . = 0x04000000;
    .text : ALIGN(0x1000) { *(.text .text.* .ltext .ltext.*) } :text
    .rodata : ALIGN(0x1000) { *(.rodata .rodata.* .lrodata .lrodata.*) } :rodata
    .kokoro_init_array : ALIGN(0x1000) { KEEP(*(.kokoro_init_array)) } :data
    .browser_init_array : ALIGN(8) { KEEP(*(.browser_init_array)) } :data
    .init_array : ALIGN(8) { KEEP(*(.init_array .init_array.*)) } :data
    .fini_array : ALIGN(8) { KEEP(*(.fini_array .fini_array.*)) } :data
    .got : ALIGN(8) { *(.got .got.*) } :data
    .data : ALIGN(0x1000) { *(.data .data.* .ldata .ldata.*) } :data
    .bss : ALIGN(0x1000) { *(.bss .bss.* .lbss .lbss.*) *(COMMON) } :data
    .speech_heap 0x100000000 (NOLOAD) : ALIGN(0x1000) { *(.speech_heap) } :speech
    /DISCARD/ : { *(.eh_frame*) *(.comment*) }
}
