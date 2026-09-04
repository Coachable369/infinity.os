bits 64
default rel

section .text
global infinity_handoff

; Microsoft x64 input from the UEFI loader:
;   rcx = BootInfo*, rdx = ELF entry, r8 = stack top, r9 = PML4 address
; Rust kernel entry uses the SysV ABI, so BootInfo moves to rdi.
; ------------------------=
; FUNC: infinity_handoff
; DESC: Installs kernel segments, paging, and stack state before entering Rust.
; ------------------=
infinity_handoff:
    cli
    lgdt [gdt_descriptor]
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov fs, ax
    mov gs, ax
    push qword 0x08
    lea rax, [rel .code_segment_ready]
    push rax
    retfq
.code_segment_ready:
    mov cr3, r9
    mov rsp, r8
    and rsp, -16
    xor rbp, rbp
    mov rdi, rcx
    call rdx
.halt:
    hlt
    jmp .halt

section .rdata
align 8
gdt:
    dq 0
    dq 0x00af9a000000ffff
    dq 0x00af92000000ffff
gdt_end:
gdt_descriptor:
    dw gdt_end - gdt - 1
    dq gdt
