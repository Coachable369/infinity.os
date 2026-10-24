SECTIONS {
    .speech_heap (NOLOAD) : { *(.speech_heap) }
    .bss.speech_control (NOLOAD) : { *(.speech_control) }
    .data.native_reset : {
        native_reset_data_start = .;
        *(.data .data.*)
        native_reset_data_end = .;
    }
    .bss.native_reset (NOLOAD) : {
        native_reset_bss_start = .;
        *(.bss .bss.* COMMON)
        native_reset_bss_end = .;
    }
    .kokoro_init_array : {
        native_init_start = .;
        KEEP(*(SORT_BY_INIT_PRIORITY(.init_array.*)))
        KEEP(*(.init_array))
        native_init_end = .;
    }
}
