#!/bin/sh
set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
vm_name=${1:-infinityos-4}
iso_input=${2:-$project_root/builds/InfinityOS-aarch64.iso}
memory_mb=${INFINITY_VM_MEMORY_MB:-12288}
cpu_count=${INFINITY_VM_CPU_COUNT:-6}
disk_size_mb=${INFINITY_VM_DISK_SIZE_MB:-16384}
vboxmanage=${INFINITY_VBOXMANAGE:-VBoxManage}

# ------------------------=
# FUNC: die
# DESC: Print a provisioning error and stop without continuing destructive work.
# ------------------=
die() {
    printf 'ERROR: %s\n' "$1" >&2
    exit 1
}

# ------------------------=
# FUNC: machine_value
# DESC: Read one value from VirtualBox machine-readable configuration output.
# ------------------=
machine_value() {
    key=$1
    printf '%s\n' "$2" | sed -n "s/^${key}=\"\(.*\)\"/\1/p"
}

# ------------------------=
# FUNC: wait_for_poweroff
# DESC: Wait for VirtualBox to report that the target VM has fully powered off.
# ------------------=
wait_for_poweroff() {
    attempts=0
    while test "$attempts" -lt 30; do
        current_info=$("$vboxmanage" showvminfo "$vm_name" --machinereadable 2>/dev/null || true)
        current_state=$(machine_value VMState "$current_info")
        if test "$current_state" = "poweroff"; then
            return 0
        fi
        attempts=$((attempts + 1))
        sleep 1
    done
    die "VirtualBox did not power off '$vm_name' within 30 seconds."
}

# ------------------------=
# FUNC: delete_existing_vm
# DESC: Power off and permanently remove only the exact VM selected for reprovisioning.
# ------------------=
delete_existing_vm() {
    existing_info=$("$vboxmanage" showvminfo "$vm_name" --machinereadable 2>/dev/null || true)
    test -n "$existing_info" || return 0

    existing_state=$(machine_value VMState "$existing_info")
    if test "$existing_state" != "poweroff"; then
        printf 'Powering off existing VM: %s\n' "$vm_name"
        "$vboxmanage" controlvm "$vm_name" poweroff >/dev/null
        wait_for_poweroff
    fi

    printf 'Deleting existing VM and its virtual disks: %s\n' "$vm_name"
    attempts=0
    while test "$attempts" -lt 30; do
        if delete_output=$("$vboxmanage" unregistervm "$vm_name" --delete 2>&1); then
            return 0
        fi
        if ! printf '%s\n' "$delete_output" | grep -Fq 'while it is locked'; then
            printf '%s\n' "$delete_output" >&2
            die "VirtualBox could not delete '$vm_name'."
        fi
        attempts=$((attempts + 1))
        sleep 1
    done
    die "VirtualBox retained a session lock on '$vm_name' for 30 seconds."
}

# ------------------------=
# FUNC: remove_stale_disk
# DESC: Remove only the replacement VM's exact leftover VDI when it is safe to do so.
# ------------------=
remove_stale_disk() {
    test -L "$disk_path" && die "Refusing to replace a symbolic-link disk path: $disk_path"
    test -e "$disk_path" || return 0
    test -f "$disk_path" || die "The replacement disk path is not a regular file: $disk_path"

    if "$vboxmanage" showmediuminfo disk "$disk_path" >/dev/null 2>&1; then
        printf 'Deleting registered leftover virtual disk: %s\n' "$disk_path"
        if ! "$vboxmanage" closemedium disk "$disk_path" --delete; then
            die "The existing disk is still registered or attached; detach it in VirtualBox before retrying: $disk_path"
        fi
    else
        printf 'Deleting unregistered leftover virtual disk: %s\n' "$disk_path"
        unlink "$disk_path" || die "Could not remove the stale virtual disk: $disk_path"
    fi

    test ! -e "$disk_path" || die "VirtualBox retained the stale virtual disk: $disk_path"
}

# ------------------------=
# FUNC: create_arm64_vm
# DESC: Create a fresh EFI ARM64 VM, blank system disk, and mounted InfinityOS ISO.
# ------------------=
create_arm64_vm() {
    "$vboxmanage" createvm \
        --name "$vm_name" \
        --platform-architecture arm \
        --ostype Other_arm64 \
        --register

    "$vboxmanage" modifyvm "$vm_name" \
        --memory "$memory_mb" \
        --cpus "$cpu_count" \
        --cpu-profile host \
        --firmware efi \
        --chipset armv8virtual \
        --vram 128 \
        --graphicscontroller vmsvga \
        --boot1 dvd \
        --boot2 disk \
        --boot3 none \
        --boot4 none \
        --nic1 nat \
        --nictype1 82540EM \
        --cable-connected1 on \
        --usb off \
        --usb-xhci on \
        --mouse usb \
        --keyboard usb \
        --audio-driver default \
        --audio-controller hda \
        --audio-enabled on

    "$vboxmanage" setextradata "$vm_name" VBoxInternal2/EfiGraphicsResolution 2560x1440
    "$vboxmanage" storagectl "$vm_name" \
        --name VirtioSCSI \
        --add virtio-scsi \
        --portcount 2 \
        --bootable on

    created_info=$("$vboxmanage" showvminfo "$vm_name" --machinereadable)
    config_file=$(machine_value CfgFile "$created_info")
    test -n "$config_file" || die "VirtualBox did not report the new VM configuration path."
    vm_directory=${config_file%/*}
    disk_path=$vm_directory/$vm_name.vdi
    remove_stale_disk

    "$vboxmanage" createmedium disk \
        --filename "$disk_path" \
        --size "$disk_size_mb" \
        --format VDI
    "$vboxmanage" storageattach "$vm_name" \
        --storagectl VirtioSCSI \
        --port 0 \
        --device 0 \
        --type hdd \
        --medium "$disk_path"
    "$vboxmanage" storageattach "$vm_name" \
        --storagectl VirtioSCSI \
        --port 1 \
        --device 0 \
        --type dvddrive \
        --medium "$iso_path"
}

# ------------------------=
# FUNC: verify_vm
# DESC: Confirm the new VM is ARM64, correctly configured, ISO-backed, and powered off.
# ------------------=
verify_vm() {
    final_info=$("$vboxmanage" showvminfo "$vm_name" --machinereadable)
    final_human_info=$("$vboxmanage" showvminfo "$vm_name")

    printf '%s\n' "$final_info" | grep -Fq 'platformArchitecture="ARM"' || die "The replacement VM is not ARM64."
    printf '%s\n' "$final_info" | grep -Fq 'ostype="Other/Unknown (ARM 64-bit)"' || die "The replacement VM has the wrong guest type."
    printf '%s\n' "$final_info" | grep -Fq "memory=$memory_mb" || die "The replacement VM has the wrong memory allocation."
    printf '%s\n' "$final_info" | grep -Fq "cpus=$cpu_count" || die "The replacement VM has the wrong CPU count."
    printf '%s\n' "$final_info" | grep -Fq 'graphicscontroller="vmsvga"' || die "The replacement VM is not using VMSVGA graphics."
    printf '%s\n' "$final_info" | grep -Fq 'VMState="poweroff"' || die "The replacement VM is not powered off."
    printf '%s\n' "$final_info" | grep -Fq "$iso_path" || die "The ARM64 ISO is not mounted."
    printf '%s\n' "$final_info" | grep -Fq 'usb="off"' || die "OHCI must remain disabled while USB keyboard input is routed through xHCI."
    printf '%s\n' "$final_info" | grep -Fq 'xhci="on"' || die "The xHCI controller is not enabled."
    printf '%s\n' "$final_human_info" | grep -Fq 'Pointing Device:             USB Mouse' || die "Generic USB HID mouse input is not configured."
    printf '%s\n' "$final_human_info" | grep -Fq 'Keyboard Device:             USB Keyboard' || die "USB keyboard input is not configured."
    printf '%s\n' "$final_human_info" | grep -Fq 'xHCI USB:                    enabled' || die "The xHCI controller is not enabled."

    printf '\nPASS: VirtualBox VM reprovisioned and left powered off.\n'
    printf '  Name:      %s\n' "$vm_name"
    printf '  Platform:  ARM64 host CPU (%s virtual CPUs)\n' "$cpu_count"
    printf '  Memory:    %s MB\n' "$memory_mb"
    printf '  Disk:      %s MB, blank VDI\n' "$disk_size_mb"
    printf '  ISO:       %s\n' "$iso_path"
    printf '  State:     powered off\n'
}

command -v "$vboxmanage" >/dev/null 2>&1 || die "VBoxManage was not found. Install VirtualBox first."
test "$(uname -m)" = "arm64" || die "This reprovisioner is restricted to an ARM64 Mac host."

case "$vm_name" in
    ''|.|..|*/*|*\\*) die "The VM name must be a simple VirtualBox name without path separators." ;;
esac

test -f "$iso_input" || die "ARM64 installer ISO not found: $iso_input"
iso_directory=$(CDPATH= cd -- "$(dirname -- "$iso_input")" && pwd)
iso_path=$iso_directory/$(basename -- "$iso_input")
file "$iso_path" | grep -Fq "ISO 9660" || die "Installer media is not an ISO 9660 image: $iso_path"
file "$iso_path" | grep -Fq "bootable" || die "Installer ISO is not marked bootable: $iso_path"

printf 'InfinityOS ARM64 VirtualBox reprovisioning\n'
printf '  VM:  %s\n' "$vm_name"
printf '  ISO: %s\n' "$iso_path"

delete_existing_vm
create_arm64_vm
verify_vm
