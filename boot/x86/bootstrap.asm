bits 16
org 0x8000

%define BOOT_INFO 0x7000
%define BOOT_MAGIC_LOW  0x4f4f5430
%define BOOT_MAGIC_HIGH 0x494e4642

; ------------------------=
; FUNC: start
; DESC: Enters protected mode and transfers control to the 32-bit bootstrap routine.
; ------------------=
start:
    cli
    mov al, 2
    out 0x92, al               ; fast A20 gate
    lgdt [gdt_descriptor]
    mov eax, cr0
    or eax, 1
    mov cr0, eax
    jmp 0x08:protected_entry

bits 32
; ------------------------=
; FUNC: protected_entry
; DESC: Loads the ELF kernel, prepares BootInfo, and calls the kernel entry point.
; ------------------=
protected_entry:
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax
    mov esp, 0x00090000
    cld

    mov esi, kernel_image
    cmp dword [esi], 0x464c457f
    jne fatal
    cmp byte [esi + 4], 1      ; ELFCLASS32
    jne fatal
    cmp byte [esi + 5], 1      ; little endian
    jne fatal
    cmp word [esi + 18], 3     ; EM_386
    jne fatal

    mov ebx, [esi + 28]
    add ebx, esi
    movzx ecx, word [esi + 44]
.next_segment:
    test ecx, ecx
    jz .segments_loaded
    cmp dword [ebx], 1         ; PT_LOAD
    jne .advance
    push ecx
    push ebx
    mov ecx, [ebx + 16]
    mov edi, [ebx + 12]
    mov esi, [ebx + 4]
    add esi, kernel_image
    rep movsb
    mov ecx, [ebx + 20]
    sub ecx, [ebx + 16]
    xor eax, eax
    rep stosb
    pop ebx
    pop ecx
.advance:
    movzx edx, word [kernel_image + 42]
    add ebx, edx
    dec ecx
    jmp .next_segment

.segments_loaded:
    mov edi, BOOT_INFO
    xor eax, eax
    mov ecx, 28                ; 112-byte BootInfo v4
    rep stosd
    mov dword [BOOT_INFO], BOOT_MAGIC_LOW
    mov dword [BOOT_INFO + 4], BOOT_MAGIC_HIGH
    mov dword [BOOT_INFO + 8], 4
    mov dword [BOOT_INFO + 12], 1
    mov dword [BOOT_INFO + 32], 1
    push dword BOOT_INFO
    mov eax, [kernel_image + 24]
    call eax

; ------------------------=
; FUNC: fatal
; DESC: Reports an unrecoverable bootstrap error and halts the processor.
; ------------------=
fatal:
    mov dx, 0x3f8
    mov al, '!'
    out dx, al
.halt:
    cli
    hlt
    jmp .halt

align 8
gdt:
    dq 0
    dq 0x00cf9a000000ffff
    dq 0x00cf92000000ffff
gdt_end:
gdt_descriptor:
    dw gdt_end - gdt - 1
    dd gdt

align 16
kernel_image:
    incbin "build/x86/kernel.elf"
kernel_image_end:

times (512 - (($-$$) % 512)) % 512 db 0
