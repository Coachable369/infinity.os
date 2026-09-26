; Native INIT/SIPI entry. The loader copies this single page below 1 MiB.
bits 16
section .rdata
align 16
global infinity_ap_page
infinity_ap_page:
; ------------------------=
; FUNC: infinity_ap_entry
; DESC: Enters long mode with kernel paging and a dedicated stack, without firmware.
; ------------------=
    cli
    cld
    mov ax, cs
    mov ds, ax
    movzx ebx, ax
    shl ebx, 4
    lgdt [0x828]
    mov eax, cr0
    or eax, 1
    mov cr0, eax
    jmp dword far [0x830]
bits 32
times 0x100-($-infinity_ap_page) db 0
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov eax, cr4
    or eax, 0x620
    mov cr4, eax
    mov eax, [ebx+0x800]
    mov cr3, eax
    mov ecx, 0xc0000080
    rdmsr
    or eax, 0x100
    wrmsr
    mov eax, cr0
    and eax, 0x9ffffff3
    or eax, 0x80000003
    mov cr0, eax
    jmp far [ebx+0x838]
bits 64
times 0x200-($-infinity_ap_page) db 0
    mov rsp, [rbx+0x808]
    and rsp, -16
    sub rsp, 32
    fninit
    ldmxcsr [rbx+0x870]
    ; Enable optional AVX state only on processors advertising XSAVE and AVX.
    ; Workers remain non-preemptible; no interrupted task shares this FP state.
    push rbx
    mov eax, 1
    cpuid
    and ecx, 0x14000000
    cmp ecx, 0x14000000
    jne .no_avx
    mov rax, cr4
    or eax, 0x40000
    mov cr4, rax
    xor ecx, ecx
    mov eax, 7
    xor edx, edx
    xsetbv
.no_avx:
    pop rbx
    mov rcx, [rbx+0x818]
    mov rax, [rbx+0x810]
    mov qword [rbx+0x820], 1
    call rax
.park:
    cli
    hlt
    jmp .park
times 0x800-($-infinity_ap_page) db 0
    times 5 dq 0
    dw 31
    dd 0
    dw 0
    dd 0
    dw 8
    dw 0
    dd 0
    dw 24
times 0x850-($-infinity_ap_page) db 0
    dq 0, 0x00cf9a000000ffff, 0x00cf92000000ffff, 0x00af9a000000ffff
    dd 0x1f80
times 4096-($-infinity_ap_page) db 0
