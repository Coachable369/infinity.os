ENTRY(_start)
SECTIONS {
    . = 0x40080000;
    .text : { KEEP(*(.text.entry)) *(.text*) }
    .rodata : { *(.rodata*) }
    .data : { *(.data*)
        . = ALIGN(4096);
        infinity_browser_private_browser_data_start = .;
        *(.browser_state_data.*)
        infinity_browser_private_browser_data_end = .;
    }
    .init_array : { __init_start = .; KEEP(*(SORT_BY_INIT_PRIORITY(.init_array.*))) KEEP(*(.init_array)) __init_end = .; }
    .bss : { *(.bss*) *(COMMON)
        . = ALIGN(4096);
        infinity_browser_private_browser_bss_start = .;
        *(.browser_state_bss.*)
        infinity_browser_private_browser_bss_end = .;
    }
    ASSERT(. < 0x70000000, "engine fixture overlaps stack reservation")
    ASSERT(!DEFINED(infinity_browser_private_infinity_browser_run) ||
           (infinity_browser_private_browser_data_end > infinity_browser_private_browser_data_start &&
            infinity_browser_private_browser_bss_end > infinity_browser_private_browser_bss_start),
           "native browser component lacks restartable state; rebuild it")
    ASSERT(infinity_browser_private_browser_data_end - infinity_browser_private_browser_data_start <= 0x200000, "browser restart checkpoint capacity exceeded")
    /DISCARD/ : { *(.eh_frame*) }
}
