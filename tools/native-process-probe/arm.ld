ENTRY(_start)
SECTIONS {
    . = 0x40080000;
    text_start = .;
    .text : { *(.text.entry) *(.text .text.*) }
    . = ALIGN(4096); text_end = .;
    rodata_start = .;
    .rodata : { *(.rodata .rodata.*) }
    . = ALIGN(4096); rodata_end = .;
    data_start = .;
    .data : { *(.data .data.*) *(.got .got.*) }
    .bss : { *(.bss .bss.*) *(COMMON) }
    . = ALIGN(4096); data_end = .;
    /DISCARD/ : { *(.eh_frame*) }
}
