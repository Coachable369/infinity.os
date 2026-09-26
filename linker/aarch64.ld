ENTRY(infinity_kernel_entry)

SECTIONS
{
    . = 0x10000000;
    .text : ALIGN(0x1000) { *(.text .text.*) }
    .rodata : ALIGN(0x1000) { *(.rodata .rodata.*) }
    .data : ALIGN(0x1000) { *(.data .data.*) }
    .kokoro_init_array : ALIGN(8) { KEEP(*(.kokoro_init_array)) }
    .bss : ALIGN(0x1000) { *(.bss .bss.*) *(COMMON) }
    /DISCARD/ : { *(.eh_frame*) *(.comment*) }
}
