ENTRY(_start)
SECTIONS {
    . = 0x40080000;
    .text : { KEEP(*(.text.entry)) *(.text*) }
    .rodata : { *(.rodata*) }
    .data : { *(.data*) }
    .init_array : { __init_start = .; KEEP(*(SORT_BY_INIT_PRIORITY(.init_array.*))) KEEP(*(.init_array)) __init_end = .; }
    .bss : { *(.bss*) *(COMMON) }
    ASSERT(. < 0x70000000, "engine fixture overlaps stack reservation")
    /DISCARD/ : { *(.eh_frame*) }
}
