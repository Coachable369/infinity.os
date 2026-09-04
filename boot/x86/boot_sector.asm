bits 16
org 0x7c00

%ifndef PAYLOAD_SECTORS
%error "PAYLOAD_SECTORS must be supplied by the build"
%endif

; ------------------------=
; FUNC: start
; DESC: Loads the bootstrap payload from BIOS sectors and transfers control to it.
; ------------------=
start:
    cli
    xor ax, ax
    mov ds, ax
    mov ss, ax
    mov sp, 0x7c00
    mov ax, 0x0800
    mov es, ax
    sti
    mov [boot_drive], dl

    mov si, 1                  ; first payload sector (LBA)
    mov di, PAYLOAD_SECTORS
    xor bx, bx                 ; payload destination starts at 0800:0000
.read_sector:
    mov ax, si
    xor dx, dx
    mov cx, 18
    div cx                     ; AX = track, DX = sector index
    mov cl, dl
    inc cl                     ; BIOS sectors begin at one
    xor dx, dx
    mov bx, 2
    div bx                     ; AX = cylinder, DX = head
    mov ch, al
    mov dh, dl
    mov dl, [boot_drive]
    xor bx, bx
    mov ax, 0x0201
    int 0x13
    jc disk_error
    mov ax, es
    add ax, 0x20               ; advance destination by one 512-byte sector
    mov es, ax
    inc si
    dec di
    jnz .read_sector
    jmp 0x0800:0x0000

; ------------------------=
; FUNC: disk_error
; DESC: Prints a BIOS disk-read failure and halts the processor.
; ------------------=
disk_error:
    mov si, error_message
.print:
    lodsb
    test al, al
    jz .halt
    mov ah, 0x0e
    mov bx, 0x0007
    int 0x10
    jmp .print
.halt:
    cli
    hlt
    jmp .halt

boot_drive db 0
error_message db "InfinityOS bootstrap: disk read error", 13, 10, 0

times 510-($-$$) db 0
dw 0xaa55
