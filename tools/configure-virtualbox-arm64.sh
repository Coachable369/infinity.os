#!/bin/sh
set -eu

vm_name=${1:-}

if test -z "$vm_name"; then
    echo "Usage: $0 <VirtualBox-VM-name>" >&2
    echo "Configures the ARM64 VM for InfinityOS USB keyboard and tablet input." >&2
    exit 2
fi

command -v VBoxManage >/dev/null 2>&1 || {
    echo "ERROR: VBoxManage was not found. Install or open VirtualBox first." >&2
    exit 1
}

machine_info=$(VBoxManage showvminfo "$vm_name" --machinereadable 2>/dev/null) || {
    echo "ERROR: VirtualBox VM '$vm_name' was not found." >&2
    exit 1
}

machine_state=$(printf '%s\n' "$machine_info" | sed -n 's/^VMState="\(.*\)"/\1/p')
configured_info=$(VBoxManage showvminfo "$vm_name")
if printf '%s\n' "$configured_info" | grep -Fq 'Pointing Device:             USB Tablet' &&
   printf '%s\n' "$configured_info" | grep -Fq 'Keyboard Device:             USB Keyboard' &&
   printf '%s\n' "$configured_info" | grep -Fq 'xHCI USB:                    enabled' &&
   printf '%s\n' "$machine_info" | grep -Fq 'usb="off"'; then
    echo "PASS: '$vm_name' already has the InfinityOS ARM64 input profile."
    exit 0
fi

if test "$machine_state" != "poweroff"; then
    echo "ERROR: '$vm_name' must be fully powered off before its input hardware can be changed." >&2
    echo "Shut the VM down, then run this command again." >&2
    exit 1
fi

# VirtualBox ARM routes its virtual tablet to the legacy OHCI root hub when
# both OHCI and xHCI are enabled. InfinityOS discovers the tablet through the
# xHCI-backed UEFI HID path, so leave OHCI off and xHCI on explicitly.
VBoxManage modifyvm "$vm_name" --usb off --usb-xhci on --mouse usbtablet --keyboard usb

configured_info=$(VBoxManage showvminfo "$vm_name")
machine_info=$(VBoxManage showvminfo "$vm_name" --machinereadable)
printf '%s\n' "$configured_info" | grep -Fq 'Pointing Device:             USB Tablet' || {
    echo "ERROR: VirtualBox did not retain the USB Tablet setting." >&2
    exit 1
}
printf '%s\n' "$configured_info" | grep -Fq 'Keyboard Device:             USB Keyboard' || {
    echo "ERROR: VirtualBox did not retain the USB Keyboard setting." >&2
    exit 1
}
printf '%s\n' "$configured_info" | grep -Fq 'xHCI USB:                    enabled' || {
    echo "ERROR: VirtualBox did not enable the xHCI controller." >&2
    exit 1
}
printf '%s\n' "$machine_info" | grep -Fq 'usb="off"' || {
    echo "ERROR: VirtualBox left OHCI enabled, so the tablet can bypass xHCI." >&2
    exit 1
}

echo "PASS: '$vm_name' is ready for InfinityOS ARM64 input."
echo "  Pointing device: USB Tablet"
echo "  Keyboard:        USB Keyboard"
echo "  USB controller:  xHCI"
echo "  Legacy OHCI:     disabled"
