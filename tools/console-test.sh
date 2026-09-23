#!/bin/sh
set -eu

arch=${1:-x86_64}
test_dir=$(mktemp -d -t infinityos-console.XXXXXX)
monitor="$test_dir/monitor.sock"
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
    x86)
        qemu-system-i386 -machine pc -m 128M -cdrom builds/InfinityOS-x86.iso -boot d \
            -serial file:"$log" -display none -no-reboot \
            -monitor unix:"$monitor",server=on,wait=off &
        ;;
    x86_64)
        firmware=${OVMF_CODE:-/opt/homebrew/share/qemu/edk2-x86_64-code.fd}
        qemu-system-x86_64 -machine q35 -m 256M \
            -drive if=pflash,format=raw,readonly=on,file="$firmware" \
            -cdrom builds/InfinityOS-x86_64.iso -serial file:"$log" -display none -no-reboot \
            -monitor unix:"$monitor",server=on,wait=off &
        ;;
    aarch64)
        firmware=${AAVMF_CODE:-/opt/homebrew/share/qemu/edk2-aarch64-code.fd}
        qemu-system-aarch64 -machine virt -cpu cortex-a72 -m 512M -bios "$firmware" \
            -device ramfb -device qemu-xhci -device usb-kbd -device usb-tablet -device virtio-scsi-pci \
            -drive if=none,id=cd,format=raw,media=cdrom,file=builds/InfinityOS-aarch64-qemu-test.iso \
            -device scsi-cd,drive=cd,bootindex=0 -serial file:"$log" -display none -no-reboot \
            -monitor unix:"$monitor",server=on,wait=off &
        ;;
    *) echo "ERROR: unsupported console-test architecture: $arch" >&2; exit 2 ;;
esac
qemu_pid=$!

attempt=0
while ! grep -Fq '[ui] startup selection' "$log" 2>/dev/null; do
    attempt=$((attempt + 1))
    test "$attempt" -lt 300 || { cat "$log"; echo 'FAIL: startup selector did not appear' >&2; exit 1; }
    sleep 0.1
done

if test "$arch" = aarch64; then key_delay=0.10; else key_delay=0.03; fi
# ------------------------=
# FUNC: type_text
# DESC: Types type text input into the virtual machine.
# ------------------=
type_text() {
    printf '%s\n' "$1" | fold -w 1 | while IFS= read -r character; do
        if test "$character" = ' '; then key=spc; else key=$character; fi
        echo "sendkey $key"
        sleep "$key_delay"
    done
}
# ------------------------=
# FUNC: press_enter
# DESC: Implements the press enter script operation.
# ------------------=
press_enter() { echo 'sendkey ret'; sleep 0.4; }

{
    type_text '3'; press_enter; sleep 0.6
    type_text 'help'; press_enter; sleep 0.6
    type_text 'storage'; press_enter; sleep 0.6
    type_text 'storage inspect'; press_enter; sleep 0.6
    type_text 'help object find'; press_enter; sleep 0.6
    type_text 'project list'; press_enter; sleep 0.6
    type_text 'collection list'; press_enter; sleep 0.6
    type_text 'device list'; press_enter; sleep 0.6
    type_text 'show connected devices'; press_enter; sleep 0.6
    type_text 'system status'; press_enter; sleep 0.6
    type_text 'system info'; press_enter; sleep 0.6
    type_text 'memory status'; press_enter; sleep 0.6
    type_text 'ai status'; press_enter; sleep 0.6
    type_text 'model list'; press_enter; sleep 0.6
    type_text 'provider list'; press_enter; sleep 0.6
    type_text 'voice status'; press_enter; sleep 0.6
    type_text 'agent list'; press_enter; sleep 0.6
    type_text 'what is going on with this machine'; press_enter; sleep 0.6
    type_text 'clear'; press_enter; sleep 0.6
    type_text 'exit'; press_enter; sleep 0.6
    echo 'sendkey tab'; sleep 0.3; press_enter; sleep 0.6
    echo 'sendkey esc'; sleep 0.8
    echo 'quit'
} | nc -U "$monitor" >/dev/null 2>&1 || true

wait "$qemu_pid" 2>/dev/null || true
qemu_pid=""

# ------------------------=
# FUNC: require
# DESC: Checks the required require condition.
# ------------------=
require() {
    grep -Fq "$1" "$log" || { cat "$log"; echo "FAIL: missing console evidence: $1" >&2; exit 1; }
}

require '[ui] infinity console'
require ' --[ NODE 01 ]-- SYSTEM ONLINE -- SELECT OPERATION -->'
require '[operation] help'
require 'storage - Infinity Pool state and usage'
require 'More information is required. Example:'
require 'Result type: ObjectSet'
require '[console] typed operation graph validated'
require 'Interpreted as: device list'
require '[operation] system.status'
require '[operation] system.info'
require '[operation] memory.status'
require '[operation] AI.Status'
require '[operation] Model.List'
require '[operation] AI.ProviderList'
require '[operation] Voice.Status'
require '[operation] Agent.List'
require 'Interpreted locally by local-intent-v1.'
require '[operation] console.clear'
require '[startup] focus=repair'
require '[repair] recovery console started'
test "$(grep -Fc '[operation] device.list' "$log")" -ge 2 || {
    cat "$log"; echo 'FAIL: exact and natural-language device requests did not share device.list' >&2; exit 1;
}
test "$(grep -Fc '[ui] startup selection' "$log")" -ge 3 || {
    cat "$log"; echo 'FAIL: exit/installer return did not reach startup selection' >&2; exit 1;
}

cat "$log"
echo "PASS: $arch console and intent runtime interaction"
