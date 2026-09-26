ENTRY(_start)
SECTIONS {
  . = 0x40080000;
  .text : { *(.text.entry) *(.text .text.*) }
  .rodata : { *(.rodata .rodata.*) }
  .init_array : {
    native_init_start = .;
    KEEP(*(.init_array .init_array.*))
    native_init_end = .;
  }
  .data : { *(.data .data.*) }
  .bss : { *(.bss .bss.*) *(COMMON) }
  /DISCARD/ : { *(.eh_frame*) }
}
