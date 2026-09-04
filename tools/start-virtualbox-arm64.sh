#!/bin/sh
set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
vm_name=${1:-}

if test -z "$vm_name"; then
    echo "Usage: $0 <VirtualBox-VM-name>" >&2
    exit 2
fi

"$project_root/tools/configure-virtualbox-arm64.sh" "$vm_name"

machine_state=$(VBoxManage showvminfo "$vm_name" --machinereadable |
    sed -n 's/^VMState="\(.*\)"/\1/p')
case "$machine_state" in
    running)
        echo "PASS: '$vm_name' is already running with verified input hardware."
        ;;
    poweroff)
        VBoxManage startvm "$vm_name" --type gui
        ;;
    *)
        echo "ERROR: '$vm_name' cannot be started from state '$machine_state'." >&2
        exit 1
        ;;
esac
