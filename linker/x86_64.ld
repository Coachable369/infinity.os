ENTRY(infinity_kernel_entry)

SECTIONS
{
    . = 0x04000000;
    .text : ALIGN(0x1000) { *(.text .text.*) }
    .rodata : ALIGN(0x1000) { *(.rodata .rodata.*) }
    .kokoro_init_array : ALIGN(8) { KEEP(*(.kokoro_init_array)) }
    .data : ALIGN(0x1000) { *(.data .data.*) }
    .bss : ALIGN(0x1000) { *(.bss .bss.*) *(COMMON) }
    .speech_heap (NOLOAD) : ALIGN(0x1000) { *(.speech_heap) }
    /DISCARD/ : { *(.eh_frame*) *(.comment*) }
}
