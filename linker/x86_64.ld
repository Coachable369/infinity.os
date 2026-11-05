ENTRY(infinity_kernel_entry)

PHDRS
{
    text PT_LOAD FLAGS(5);
    rodata PT_LOAD FLAGS(4);
    data PT_LOAD FLAGS(6);
    speech PT_LOAD FLAGS(6);
    browser PT_LOAD FLAGS(6);
    backdrop PT_LOAD FLAGS(6);
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
    .got : ALIGN(0x1000) { *(.got .got.*) } :data
    .data : ALIGN(0x1000) { *(.data .data.* .ldata .ldata.*)
        . = ALIGN(4096);
        infinity_browser_private_browser_data_start = .;
        *(.browser_state_data.*)
        infinity_browser_private_browser_data_end = .;
    } :data
    .bss : ALIGN(0x1000) { *(.bss .bss.* .lbss .lbss.*) *(COMMON)
        . = ALIGN(4096);
        infinity_browser_private_browser_bss_start = .;
        *(.browser_state_bss.*)
        infinity_browser_private_browser_bss_end = .;
    } :data
    ASSERT(. < 0x80000000, "x86 ordinary kernel data exceeds relative-addressing range")
    .speech_heap 0x100000000 (NOLOAD) : ALIGN(0x1000) { *(.speech_heap) } :speech
    .browser_heap 0x160000000 (NOLOAD) : ALIGN(0x1000) { *(.browser_heap) } :browser
    .login_backdrop 0x180000000 (NOLOAD) : ALIGN(0x1000) { *(.login_backdrop) *(.launcher_stage) } :backdrop
    ASSERT(SIZEOF(.login_backdrop) == 0 || SIZEOF(.login_backdrop) == 0x3f49000,
           "x86 display arena must retain both complete 4K caches")
    ASSERT(SIZEOF(.browser_heap) == 0 || SIZEOF(.browser_heap) == 0x20000000,
           "x86 browser arena must retain its complete 512 MiB allocation")
    ASSERT(!DEFINED(infinity_browser_private_infinity_browser_run) ||
           (infinity_browser_private_browser_data_end > infinity_browser_private_browser_data_start &&
            infinity_browser_private_browser_bss_end > infinity_browser_private_browser_bss_start),
           "native browser component lacks restartable state; rebuild it")
    ASSERT(infinity_browser_private_browser_data_end - infinity_browser_private_browser_data_start <= 0x800000, "browser restart checkpoint capacity exceeded")
    /DISCARD/ : { *(.eh_frame*) *(.comment*) }
}
