#!/bin/sh
set -eu

arch=${1:-aarch64}
test_dir=$(mktemp -d -t infinityos-mouse-menu.XXXXXX)
monitor="$test_dir/monitor.sock"
qmp="$test_dir/qmp.sock"
log="$test_dir/serial.log"
qemu_pid=""

# ------------------------=
# FUNC: cleanup
# DESC: Releases temporary resources created by this script.
# ------------------=
cleanup() {
    if test -n "$qemu_pid" && kill -0 "$qemu_pid" 2>/dev/null; then kill "$qemu_pid" 2>/dev/null || true; fi
    rm -rf "$test_dir"
}
trap cleanup EXIT INT TERM

case "$arch" in
    x86_64)
        firmware=${OVMF_CODE:-/opt/homebrew/share/qemu/edk2-x86_64-code.fd}
        qemu-system-x86_64 -machine q35 -m 1024M \
            -drive if=pflash,format=raw,readonly=on,file="$firmware" \
            -cdrom builds/InfinityOS-x86_64.iso -serial file:"$log" -display none -no-reboot \
            -monitor unix:"$monitor",server=on,wait=off &
        ;;
    aarch64)
        firmware=${AAVMF_CODE:-/opt/homebrew/share/qemu/edk2-aarch64-code.fd}
        qemu-system-aarch64 -machine virt -cpu cortex-a72 -m 1024M -bios "$firmware" \
            -device ramfb -device qemu-xhci -device usb-kbd -device usb-mouse -device virtio-scsi-pci \
            -drive if=none,id=cd,format=raw,media=cdrom,file=build/test-media/InfinityOS-aarch64-qemu.bootmedia \
            -device scsi-cd,drive=cd,bootindex=0 -serial file:"$log" -display vnc=127.0.0.1:97 -no-reboot \
            -monitor unix:"$monitor",server=on,wait=off -qmp unix:"$qmp",server=on,wait=off &
        ;;
    *) echo "ERROR: unsupported mouse-menu architecture: $arch" >&2; exit 2 ;;
esac
qemu_pid=$!

attempt=0
while ! grep -Fq '[ui] startup selection' "$log" 2>/dev/null; do
    attempt=$((attempt + 1))
    test "$attempt" -lt 300 || { cat "$log"; echo 'FAIL: startup selector did not appear' >&2; exit 1; }
    sleep 0.1
done

if test "$arch" = aarch64; then
    # Exercise the same raw USB HID fallback used when VirtualBox advertises a
    # stale Absolute Pointer protocol after ExitBootServices.
    {
        echo 'sendkey down'; sleep 0.3
        echo 'mouse_button 1'; sleep 0.3; echo 'mouse_button 0'; sleep 0.8
        echo 'quit'
    } | nc -U "$monitor" >/dev/null 2>&1 || true
    wait "$qemu_pid" 2>/dev/null || true
    qemu_pid=""
    grep -Fq '[BOOT] firmware pointer set ready' "$log"
    grep -Fq '[device] mouse0 ready' "$log"
    grep -Fq '[mouse] Repair selected' "$log"
    grep -Fq '[repair] recovery console started' "$log"
    cat "$log"
    echo 'PASS: aarch64 raw USB mouse activated the initial menu'
    exit 0
else
    {
        # Keyboard focus deliberately updates the same pointer coordinates as
        # hover navigation. This makes button-edge testing deterministic across
        # QEMU versions whose HMP mouse_move packet scale differs.
        echo 'sendkey down'; sleep 0.3
        echo 'mouse_button 1'; sleep 0.3; echo 'mouse_button 0'; sleep 0.5
        echo 'sendkey e-x-i-t'; sleep 0.2; echo 'sendkey ret'; sleep 1
        echo 'sendkey down'; echo 'sendkey down'; sleep 0.3
        echo 'mouse_button 1'; sleep 0.3; echo 'mouse_button 0'; sleep 0.5
        echo 'sendkey e-x-i-t'; sleep 0.2; echo 'sendkey ret'; sleep 1
        echo 'sendkey down'; echo 'sendkey down'; echo 'sendkey down'; sleep 0.3
        echo 'mouse_button 1'; sleep 0.3; echo 'mouse_button 0'; sleep 0.5
        echo 'mouse_move 60 -33'; sleep 0.3
        echo 'mouse_button 1'; sleep 0.3; echo 'mouse_button 0'; sleep 0.8
        echo 'sendkey ret'; sleep 0.8
        echo 'quit'
    } | nc -U "$monitor" >/dev/null 2>&1 || true
fi

wait "$qemu_pid" 2>/dev/null || true
qemu_pid=""

grep -Fq '[mouse] Console selected' "$log" || {
    cat "$log"; echo 'FAIL: Recovery Console row did not respond to mouse click' >&2; exit 1;
}
grep -Fq '[mouse] Installer selected' "$log" || {
    cat "$log"; echo 'FAIL: Installer row did not respond to mouse click' >&2; exit 1;
}
grep -Fq '[mouse] Repair selected' "$log" || {
    cat "$log"; echo 'FAIL: Repair row did not respond to mouse click' >&2; exit 1;
}
grep -Fq '[installer] wizard started' "$log" || {
    cat "$log"; echo 'FAIL: mouse-selected installer did not reach wizard' >&2; exit 1;
}
grep -Fq 'ONE DISK BECOMES ONE INFINITY POOL.' "$log" || {
    cat "$log"; echo 'FAIL: mouse activation did not advance the installer' >&2; exit 1;
}

cat "$log"
echo "PASS: $arch graphical menu mouse navigation"
