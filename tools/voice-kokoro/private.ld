SECTIONS {
    .kokoro_init_array : {
        native_init_start = .;
        KEEP(*(SORT_BY_INIT_PRIORITY(.init_array.*)))
        KEEP(*(.init_array))
        native_init_end = .;
    }
}
