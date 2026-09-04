#!/bin/sh
set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
configured=0

if ! command -v VBoxManage >/dev/null 2>&1; then
    echo "VirtualBox not installed; ARM64 VM input guard skipped."
    exit 0
fi

vm_ids=$(VBoxManage list vms | sed -n 's/.*{\([^}]*\)}$/\1/p')
for vm_id in $vm_ids; do
    machine_info=$(VBoxManage showvminfo "$vm_id" --machinereadable 2>/dev/null || true)
    printf '%s\n' "$machine_info" | grep -Fqi 'ARM 64-bit' || continue
    # Validate both attached InfinityOS media and recognizable InfinityOS VM
    # names. A newly created VM can exist before the ISO is mounted; limiting
    # the guard to its current optical medium allowed that PS/2/USB-off profile
    # to survive the build and fail on the next launch.
    vm_name=$(printf '%s\n' "$machine_info" | sed -n 's/^name="\(.*\)"/\1/p')
    if ! printf '%s\n' "$machine_info" | grep -Eqi 'InfinityOS-aarch64\.iso|infinity-aarch64\.iso' &&
       ! printf '%s\n' "$vm_name" | grep -Eqi 'infinityos|infinitybox|infinity apple arm'; then
        continue
    fi
    "$project_root/tools/configure-virtualbox-arm64.sh" "$vm_id"
    configured=$((configured + 1))
done

if test "$configured" -eq 0; then
    echo "PASS: no registered ARM64 VM attached to InfinityOS media requires configuration."
else
    echo "PASS: verified the InfinityOS input profile for $configured attached ARM64 VM(s)."
fi
