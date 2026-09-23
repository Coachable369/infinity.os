#!/bin/sh
set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
firmware=${OVMF_CODE:-/opt/homebrew/share/qemu/edk2-x86_64-code.fd}
qa_dir=$(mktemp -d -t infinity-installer-ui.XXXXXX)
disk="$qa_dir/disk.raw"
monitor="$qa_dir/monitor.sock"
log="$qa_dir/install.log"
qemu_pid=""

# ------------------------=
# FUNC: cleanup
# DESC: Stops the private installer QA machine and removes its temporary disk.
# ------------------=
cleanup() {
    if test -n "$qemu_pid" && kill -0 "$qemu_pid" 2>/dev/null; then kill "$qemu_pid" 2>/dev/null || true; fi
    rm -rf "$qa_dir"
}
trap cleanup EXIT INT TERM

# ------------------------=
# FUNC: monitor_command
# DESC: Sends one HMP command and allows the guest a bounded interval to render it.
# ------------------=
monitor_command() {
    { echo "$1"; sleep "${2:-0.25}"; } | nc -U "$monitor" >/dev/null 2>&1 || true
}

# ------------------------=
# FUNC: capture_frame
# DESC: Captures and converts one installer framebuffer proof image.
# ------------------=
capture_frame() {
    name=$1
    ppm="$project_root/builds/InfinityOS-$name-proof.ppm"
    png="$project_root/builds/InfinityOS-$name-proof.png"
    monitor_command "screendump $ppm" 0.6
    test -s "$ppm" || { cat "$log"; echo "framebuffer capture failed: $name" >&2; exit 1; }
    sips -s format png "$ppm" --out "$png" >/dev/null
    test -s "$png"
    echo "Installer UI VM capture: $png"
}

cd "$project_root"
test -s builds/InfinityOS-x86_64.iso
mkdir -p builds
dd if=/dev/zero of="$disk" bs=1M count=0 seek=16384 status=none
: > "$log"

qemu-system-x86_64 -machine pc -m 4096M \
    -drive if=pflash,format=raw,readonly=on,file="$firmware" \
    -drive if=ide,index=0,format=raw,file="$disk" \
    -drive if=ide,index=2,media=cdrom,readonly=on,file=builds/InfinityOS-x86_64.iso \
    -boot order=d -serial file:"$log" -display none -no-reboot \
    -monitor unix:"$monitor",server=on,wait=off &
qemu_pid=$!

attempt=0
while ! grep -Fq '[ui] startup selection' "$log" 2>/dev/null; do
    attempt=$((attempt + 1))
    test "$attempt" -lt 400 || { cat "$log"; echo "installer did not start" >&2; exit 1; }
    sleep 0.1
done

monitor_command 'sendkey 1' 0.4
monitor_command 'sendkey ret' 2
monitor_command 'sendkey ret' 2
monitor_command 'sendkey ret' 2
monitor_command 'sendkey ret' 2
monitor_command 'sendkey ret' 2
monitor_command 'sendkey right' 0.6
monitor_command 'sendkey tab' 0.6
monitor_command 'sendkey tab' 0.6
monitor_command 'sendkey tab' 0.6
monitor_command 'sendkey ret' 2
monitor_command 'sendkey ret' 2

grep -Fq '[installer] confirmation popup opened' "$log" || { cat "$log"; exit 1; }
capture_frame installer-confirmation
monitor_command 'mouse_move 48 28' 0.5
capture_frame installer-confirmation-motion

monitor_command 'sendkey ret' 0.25
sleep 1.2
capture_frame installer-progress
