SECTIONS {
    .browser_init_array : {
        browser_init_start = .;
        KEEP(*(SORT_BY_INIT_PRIORITY(.init_array.*)))
        KEEP(*(.init_array))
        browser_init_end = .;
    }
}
