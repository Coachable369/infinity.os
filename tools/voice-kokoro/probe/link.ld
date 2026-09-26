ENTRY(_start)
SECTIONS {
  . = 0x40080000;
  .text : { *(.text.entry) *(.text .text.*) }
  .rodata : { *(.rodata .rodata.*) }
  .kokoro_init_array : { KEEP(*(.kokoro_init_array)) }
  .data : { *(.data .data.*) }
  .bss : { *(.bss .bss.*) *(COMMON) }
  /DISCARD/ : { *(.eh_frame*) }
}
