#!/bin/sh
set -eu

disk=build/infinity-test-disk.raw
firmware=${OVMF_CODE:-/opt/homebrew/share/qemu/edk2-x86_64-code.fd}
log=build/installed-console-test.log
test_dir=$(mktemp -d -t infinityos-console.XXXXXX)
monitor="$test_dir/monitor.sock"
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

: > "$log"
qemu-system-x86_64 -machine pc -m 256M \
    -drive if=pflash,format=raw,readonly=on,file="$firmware" \
    -drive if=ide,index=0,format=raw,file="$disk" -boot order=c \
    -serial file:"$log" -display none -no-reboot \
    -monitor unix:"$monitor",server=on,wait=off &
qemu_pid=$!

attempt=0
while ! grep -Fq '[ui] infinity console' "$log" 2>/dev/null; do
    attempt=$((attempt + 1))
    test "$attempt" -lt 400 || { cat "$log"; echo 'FAIL: installed console did not start' >&2; exit 1; }
    sleep 0.1
done

# ------------------------=
# FUNC: type_command
# DESC: Types type command input into the virtual machine.
# ------------------=
type_command() {
    command=$1
    while test -n "$command"; do
        character=${command%"${command#?}"}
        command=${command#?}
        if test "$character" = ' '; then
            key=spc
        elif test "$character" = '-'; then
            key=minus
        else
            key=$character
        fi
        echo "sendkey $key"
        sleep 0.04
    done
    echo 'sendkey ret'
    sleep 0.4
}

{
    type_command 'system generation'
    type_command 'system boot'
    type_command 'ai status'
    type_command 'storage'
    type_command 'project list'
    type_command 'collection list'
    type_command 'model inspect local-intent-v1'
    type_command 'what is going on with this machine'
    echo quit
} | nc -U "$monitor" >/dev/null 2>&1 || true
wait "$qemu_pid" 2>/dev/null || true
qemu_pid=""

for evidence in '[operation] System.GenerationInspect' 'Active generation: 1' \
    'State: ACTIVE' 'Integrity: valid (verified by Infinity EFI)' \
    '[operation] System.BootStatus' 'Boot mode: Installed' 'Boot device: storage0'; do
    grep -Fq "$evidence" "$log" || { cat "$log"; echo "FAIL: missing installed console evidence: $evidence" >&2; exit 1; }
done
for evidence in 'storage - Infinity Pool state and usage' 'ProjectSet result' \
    'CollectionSet result' '[console] typed operation graph validated'; do
    grep -Fq "$evidence" "$log" || { cat "$log"; echo "FAIL: missing installed Console language evidence: $evidence" >&2; exit 1; }
done
for evidence in '[operation] AI.Status' 'Infinity AI Service: ready' \
    '[operation] Model.Inspect' 'Object binding: native System object' 'Processing: LOCAL' \
    'Interpreted locally by local-intent-v1.'; do
    grep -Fq "$evidence" "$log" || { cat "$log"; echo "FAIL: missing installed AI evidence: $evidence" >&2; exit 1; }
done
echo 'PASS: installed generation and boot status operations'
