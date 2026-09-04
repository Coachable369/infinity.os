#!/bin/sh
set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
disk="$project_root/build/infinity-test-disk.raw"
firmware=${OVMF_CODE:-/opt/homebrew/share/qemu/edk2-x86_64-code.fd}
qa_dir=$(mktemp -d -t infinityui-qa.XXXXXX)
qa_disk="$qa_dir/installed-system.raw"
monitor="$qa_dir/monitor.sock"
log="$project_root/build/infinity-ui-vm-qa.log"
qemu_pid=""

test -s "$disk" || { echo "missing installed test disk" >&2; exit 1; }
cp "$disk" "$qa_disk"
mkdir -p "$project_root/builds"
: > "$log"

# ------------------------=
# FUNC: cleanup
# DESC: Stops the visual QA machine and removes its private monitor socket.
# ------------------=
cleanup() {
    if test -n "$qemu_pid" && kill -0 "$qemu_pid" 2>/dev/null; then kill "$qemu_pid" 2>/dev/null || true; fi
    rm -rf "$qa_dir"
}
trap cleanup EXIT INT TERM

# ------------------------=
# FUNC: monitor_command
# DESC: Sends one bounded keyboard or capture command to the QEMU control monitor.
# ------------------=
monitor_command() {
    { echo "$1"; sleep "${2:-0.16}"; } | nc -U "$monitor" >/dev/null 2>&1 || true
}

# ------------------------=
# FUNC: type_word
# DESC: Types an ASCII onboarding value using discrete firmware keyboard events.
# ------------------=
type_word() {
    value=$1
    while test -n "$value"; do
        character=$(printf '%s' "$value" | cut -c1)
        monitor_command "sendkey $character" 0.09
        value=$(printf '%s' "$value" | cut -c2-)
    done
}

# ------------------------=
# FUNC: capture_frame
# DESC: Captures and converts one named VM framebuffer state for visual review.
# ------------------=
capture_frame() {
    capture_name=$1
    capture_ppm="$project_root/builds/InfinityOS-$capture_name-proof.ppm"
    capture_png="$project_root/builds/InfinityOS-$capture_name-proof.png"
    monitor_command "screendump $capture_ppm" 0.5
    test -s "$capture_ppm" || { cat "$log"; echo "framebuffer capture failed: $capture_name" >&2; exit 1; }
    sips -s format png "$capture_ppm" --out "$capture_png" >/dev/null
    test -s "$capture_png"
    echo "InfinityUI VM capture: $capture_png"
}

qemu-system-x86_64 -machine pc -m 512M \
    -drive if=pflash,format=raw,readonly=on,file="$firmware" \
    -drive if=ide,index=0,format=raw,file="$qa_disk" -boot order=c \
    -serial file:"$log" -display none -no-reboot \
    -monitor unix:"$monitor",server=on,wait=off &
qemu_pid=$!

attempt=0
while ! grep -Fq '[onboarding] first-boot experience started' "$log" 2>/dev/null; do
    attempt=$((attempt + 1))
    test "$attempt" -lt 500 || { cat "$log"; echo "onboarding did not appear" >&2; exit 1; }
    sleep 0.1
done

sleep 1
capture_frame onboarding-polish

monitor_command 'sendkey ret' 0.5
monitor_command 'sendkey ret' 0.4
capture_frame onboarding-validation
type_word infinitynode
monitor_command 'sendkey tab' 0.2
monitor_command 'sendkey tab' 0.2
monitor_command 'sendkey ret' 0.4
type_word aurelius
monitor_command 'sendkey ret' 0.4
monitor_command 'sendkey shift-a' 0.1
type_word urelius
monitor_command 'sendkey ret' 0.4
type_word password
monitor_command 'sendkey ret' 0.4
monitor_command 'sendkey ret' 0.4
monitor_command 'sendkey ret' 1.0

attempt=0
while ! grep -Fq '[shell] top bar ready' "$log" 2>/dev/null; do
    attempt=$((attempt + 1))
    test "$attempt" -lt 300 || { cat "$log"; echo "desktop did not appear" >&2; exit 1; }
    sleep 0.1
done

sleep 1
capture_frame desktop-polish
monitor_command 'sendkey ret' 0.8
capture_frame system-menu-polish
monitor_command 'sendkey down' 0.3
monitor_command 'sendkey ret' 3.0
capture_frame settings-polish
monitor_command 'sendkey esc' 0.5
monitor_command 'sendkey ret' 0.5
for _ in 1 2 3 4 5 6 7; do monitor_command 'sendkey down' 0.4; done
monitor_command 'sendkey ret' 1.0
attempt=0
while ! grep -Fq '[session] signed out; authentication surface ready' "$log" 2>/dev/null; do
    attempt=$((attempt + 1))
    test "$attempt" -lt 100 || { cat "$log"; echo "authentication surface did not appear" >&2; exit 1; }
    sleep 0.1
done
sleep 1
capture_frame login-gold-standard
