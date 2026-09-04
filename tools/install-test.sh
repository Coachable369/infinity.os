#!/bin/sh
set -eu

mode=${1:-full}
disk=build/infinity-test-disk.raw
firmware=${OVMF_CODE:-/opt/homebrew/share/qemu/edk2-x86_64-code.fd}
install_log=build/install-test.log
boot_log=build/installed-boot.log

# ------------------------=
# FUNC: boot_installed
# DESC: Starts and validates boot installed.
# ------------------=
boot_installed() {
    : > "$boot_log"
    set +e
    timeout 35 qemu-system-x86_64 -machine pc -m 256M \
        -drive if=pflash,format=raw,readonly=on,file="$firmware" \
        -drive if=ide,index=0,format=raw,file="$disk" -boot order=c \
        -serial file:"$boot_log" -display none -no-reboot
    qemu_result=$?
    set -e
    if ! grep -Fq '[BOOT] Infinity container located' "$boot_log" ||
       ! grep -Fq '[BOOT] Infinity pool located' "$boot_log" ||
       ! grep -Fq 'Boot source: installed system' "$boot_log" ||
       ! grep -Fq '[BOOT] System Space online' "$boot_log" ||
       ! grep -Fq '[BOOT] Generation 1 ACTIVE' "$boot_log" ||
       ! grep -Fq '[BOOT] System manifest valid' "$boot_log" ||
       ! grep -Fq '[BOOT] Kernel valid' "$boot_log" ||
       ! grep -Fq 'Kernel online.' "$boot_log" ||
       ! grep -Fq '[runtime] execution manager online' "$boot_log" ||
       ! grep -Fq '[runtime] capability manager online' "$boot_log" ||
       ! grep -Fq '[iop] router online' "$boot_log" ||
       ! grep -Fq '[event] fabric online' "$boot_log" ||
       ! grep -Fq '[service] storage ready' "$boot_log" ||
       ! grep -Fq '[service] object ready' "$boot_log" ||
       ! grep -Fq '[service] namespace ready' "$boot_log" ||
       ! grep -Fq '[service] console ready' "$boot_log" ||
       ! grep -Fq '[service] local-ml ready' "$boot_log" ||
       ! grep -Fq '[service] infinity-ai ready' "$boot_log" ||
       ! grep -Fq '[service] voice ready' "$boot_log" ||
       ! grep -Fq '[service] agent ready' "$boot_log" ||
       ! grep -Fq '[service] identity ready' "$boot_log" ||
       ! grep -Fq '[service] authentication ready' "$boot_log" ||
       ! grep -Fq '[service] session ready' "$boot_log" ||
       ! grep -Fq '[service] settings ready' "$boot_log" ||
       ! grep -Fq '[service] onboarding ready' "$boot_log" ||
       ! grep -Fq '[service] shell ready' "$boot_log" ||
       ! grep -Fq '[service] skin-registry ready' "$boot_log" ||
       ! grep -Fq '[service] window-server ready' "$boot_log" ||
       ! grep -Fq '[service] infinity-ui ready' "$boot_log" ||
       ! grep -Fq '[service] clipboard ready' "$boot_log" ||
       ! grep -Fq '[ai] local CPU inference online' "$boot_log" ||
       ! grep -Fq '[ai] model registry verified' "$boot_log" ||
       ! grep -Fq 'Infinity Runtime online.' "$boot_log" ||
       ! grep -Fq '[onboarding] first-boot experience started' "$boot_log" ||
       grep -Fq '[ui] startup selection' "$boot_log"; then
        cat "$boot_log"
        echo "FAIL: installed disk did not boot independently (QEMU status $qemu_result)" >&2
        exit 1
    fi
    cat "$boot_log"
    echo 'PASS: installed disk booted without installation media'
}

if test "$mode" = --boot-only; then
    test -s "$disk" || { echo "ERROR: test disk does not exist: $disk" >&2; exit 1; }
    boot_installed
    exit 0
fi

dd if=/dev/zero of="$disk" bs=1M count=512 status=none
: > "$install_log"
test_dir=$(mktemp -d -t infinityos-install.XXXXXX)
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

qemu-system-x86_64 -machine pc -m 512M \
    -drive if=pflash,format=raw,readonly=on,file="$firmware" \
    -drive if=ide,index=0,format=raw,file="$disk" \
    -drive if=ide,index=2,media=cdrom,readonly=on,file=build/infinity-x86_64.iso \
    -boot order=d -serial file:"$install_log" -display none -no-reboot \
    -monitor unix:"$monitor",server=on,wait=off &
qemu_pid=$!

attempt=0
while ! grep -Fq '[ui] startup selection' "$install_log" 2>/dev/null; do
    attempt=$((attempt + 1))
    test "$attempt" -lt 400 || { cat "$install_log"; echo 'FAIL: installer did not start' >&2; exit 1; }
    sleep 0.1
done

{
    echo 'sendkey 1'; sleep 0.2; echo 'sendkey ret'; sleep 1
    echo 'sendkey f1'; sleep 1; echo 'sendkey f1'; sleep 1
    echo 'sendkey shift-tab'; sleep 0.7; echo 'sendkey tab'; sleep 0.7; echo 'sendkey ret'; sleep 1
    echo 'sendkey ret'; sleep 1
    echo 'sendkey ret'; sleep 1
    echo 'sendkey ret'; sleep 1
    echo 'sendkey right'; sleep 0.7
    echo 'sendkey tab'; sleep 0.7; echo 'sendkey tab'; sleep 0.7; echo 'sendkey tab'; sleep 0.7; echo 'sendkey ret'; sleep 1
    echo 'sendkey ret'; sleep 1
} | nc -U "$monitor" >/dev/null 2>&1 || true

grep -Fq '[installer] confirmation popup opened' "$install_log" || {
    cat "$install_log"; echo 'FAIL: destructive confirmation popup did not open' >&2; exit 1;
}
{ echo 'sendkey ret'; } | nc -U "$monitor" >/dev/null 2>&1 || true
test "$(grep -Fc '[installer] help toggled' "$install_log")" -eq 2 || {
    cat "$install_log"; echo 'FAIL: F1 did not open and close installer help' >&2; exit 1;
}
attempt=0
while ! grep -Fq '[installer] confirmation popup cancelled' "$install_log" 2>/dev/null; do
    attempt=$((attempt + 1))
    test "$attempt" -lt 80 || {
        cat "$install_log"; echo 'FAIL: confirmation popup cancel action did not return to review' >&2; exit 1;
    }
    sleep 0.1
done
grep -Fq '[installer] date-time configuration changed' "$install_log" || {
    cat "$install_log"; echo 'FAIL: date/time controls did not accept keyboard input' >&2; exit 1;
}
grep -Fq '[installer] focus=back' "$install_log" || {
    cat "$install_log"; echo 'FAIL: Tab did not move installer focus' >&2; exit 1;
}
grep -Fq '[installer] focus=back reverse' "$install_log" || {
    cat "$install_log"; echo 'FAIL: Shift-Tab did not reverse installer focus' >&2; exit 1;
}
if grep -Fq '[provision] state=provisioning' "$install_log"; then
    cat "$install_log"; echo 'FAIL: provisioning began after popup cancellation' >&2; exit 1
fi

{
    echo 'sendkey ret'; sleep 1
    echo 'sendkey tab'; sleep 0.7
    echo 'sendkey ret'; sleep 1
} | nc -U "$monitor" >/dev/null 2>&1 || true

grep -Fq '[installer] destructive confirmation accepted' "$install_log" || {
    cat "$install_log"; echo 'FAIL: confirmation popup did not authorize installation' >&2; exit 1;
}

attempt=0
while ! grep -Fq '[install] complete' "$install_log" 2>/dev/null; do
    attempt=$((attempt + 1))
    if ! kill -0 "$qemu_pid" 2>/dev/null || test "$attempt" -ge 1800; then
        cat "$install_log"; echo 'FAIL: provisioning did not complete' >&2; exit 1
    fi
    sleep 0.1
done

{ echo 'sendkey ret'; } | nc -U "$monitor" >/dev/null 2>&1 || true
attempt=0
while kill -0 "$qemu_pid" 2>/dev/null; do
    attempt=$((attempt + 1)); test "$attempt" -lt 180 || {
        cat "$install_log"; echo 'FAIL: firmware reboot mechanism did not reset QEMU' >&2; exit 1;
    }
    sleep 0.1
done
wait "$qemu_pid" 2>/dev/null || true
qemu_pid=""

for evidence in '[provision] state=provisioning' '[verify] Partition table' \
    '[verify] EFI boot environment' '[verify] Infinity container' '[verify] Infinity pool' \
    '[verify] System space' '[verify] Personal space' '[verify] Applications space' \
    '[verify] Recovery space' '[verify] InfinityOS kernel' '[verify] Bootloader' \
    '[write] Runtime, service registry, capability policy, native AI, and InfinityUI objects' '[verify] Runtime core' \
    '[verify] Service manifests' '[verify] Capability policy' '[verify] Service registry' \
    '[verify] Native AI model registry and policy' \
    '[verify] InfinityUI runtime and skin packages' '[verify] Window Server and trusted UI policy' \
    '[generation] Generation 1 state=INSTALLING' '[write] System component manifest (CORE)' \
    '[generation] Generation 1 state=READY' '[verify] System manifest' \
    '[generation] Generation 1 state=ACTIVE' '[verify] Generation activated' \
    '[provision] state=complete' '[install] complete' \
    '[install] reboot countdown started' '[install] firmware reboot requested'; do
    grep -Fq "$evidence" "$install_log" || { cat "$install_log"; echo "FAIL: missing evidence: $evidence" >&2; exit 1; }
done
echo 'PASS: blank virtual disk provisioned and verified'
boot_installed
