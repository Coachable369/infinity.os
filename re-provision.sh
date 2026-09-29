#!/bin/sh
set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
vm_name=${1:-infinityos-4}
. "$project_root/tools/select-install-iso.sh"
iso_input=$(select_install_iso "$project_root" "${2:-}")
memory_mb=${INFINITY_VM_MEMORY_MB:-20480}
cpu_count=${INFINITY_VM_CPU_COUNT:-6}
disk_size_mb=${INFINITY_VM_DISK_SIZE_MB:-16384}
vboxmanage=${INFINITY_VBOXMANAGE:-VBoxManage}
trace_root=${INFINITY_VM_TRACE_ROOT:-$project_root/build/vm-logs}

# ------------------------=
# FUNC: die
# DESC: Print a provisioning error and stop without continuing destructive work.
# ------------------=
die() {
    printf 'ERROR: %s\n' "$1" >&2
    exit 1
}

# ------------------------=
# FUNC: prepare_trace_log
# DESC: Preserves the prior VM serial trace and prepares a repository-local sink for the replacement VM.
# ------------------=
prepare_trace_log() {
    case "$trace_root" in
        "$project_root"/build/*) ;;
        *) die "The VM trace root must remain under $project_root/build: $trace_root" ;;
    esac
    trace_directory=$trace_root/$vm_name
    trace_log=$trace_directory/serial.log
    mkdir -p "$trace_directory"
    if test -s "$trace_log"; then
        trace_archive=$trace_directory/serial-$(date -u +%Y%m%dT%H%M%SZ)-$$.log
        mv "$trace_log" "$trace_archive"
        printf 'Archived prior VM trace: %s\n' "$trace_archive"
    fi
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
# FUNC: default_machine_folder
# DESC: Resolve VirtualBox's configured machine root without assuming a user home path.
# ------------------=
default_machine_folder() {
    "$vboxmanage" list systemproperties 2>/dev/null \
        | sed -n 's/^Default machine folder:[[:space:]]*//p' \
        | sed -n '1p'
}

# ------------------------=
# FUNC: recover_unregistered_vm
# DESC: Re-register and delete only the exact stale VM directory that blocks idempotent recreation.
# ------------------=
recover_unregistered_vm() {
    machine_root=${INFINITY_VM_BASE_FOLDER:-$(default_machine_folder)}
    test -n "$machine_root" || die "VirtualBox did not report its default machine folder."
    case "$machine_root" in
        /*) ;;
        *) die "VirtualBox reported a non-absolute machine folder: $machine_root" ;;
    esac

    stale_directory=$machine_root/$vm_name
    stale_config=$stale_directory/$vm_name.vbox
    test -e "$stale_directory" || return 0
    test ! -L "$stale_directory" || die "Refusing to recover a symbolic-link VM directory: $stale_directory"

    if test ! -e "$stale_config"; then
        rmdir "$stale_directory" 2>/dev/null || true
        return 0
    fi
    test ! -L "$stale_config" || die "Refusing to recover a symbolic-link VM settings file: $stale_config"
    test -f "$stale_config" || die "The stale VM settings path is not a regular file: $stale_config"

    printf 'Recovering unregistered stale VM for deletion: %s\n' "$stale_config"
    if ! register_output=$("$vboxmanage" registervm "$stale_config" 2>&1); then
        printf '%s\n' "$register_output" >&2
        die "VirtualBox could not register the stale VM for safe deletion."
    fi
    delete_existing_vm
    if test -d "$stale_directory"; then rmdir "$stale_directory" 2>/dev/null || true; fi
    test ! -e "$stale_config" || die "VirtualBox retained the stale VM settings file: $stale_config"
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
        --tpm-type 2.0 \
        --chipset armv8virtual \
        --vram 128 \
        --graphicscontroller vmsvga \
        --boot1 disk \
        --boot2 dvd \
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
        --uart1 0x3F8 4 \
        --uart-mode1 file "$trace_log" \
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
    printf '%s\n' "$final_info" | grep -Fq 'uart1="0x03f8,4"' || die "The VM serial trace UART is not enabled."
    printf '%s\n' "$final_info" | grep -Fq "uartmode1=\"file,$trace_log\"" || die "The VM serial trace file is not configured."
    printf '%s\n' "$final_human_info" | grep -Fq 'Pointing Device:             USB Mouse' || die "Generic USB HID mouse input is not configured."
    printf '%s\n' "$final_human_info" | grep -Fq 'Keyboard Device:             USB Keyboard' || die "USB keyboard input is not configured."
    printf '%s\n' "$final_human_info" | grep -Fq 'xHCI USB:                    enabled' || die "The xHCI controller is not enabled."

    printf '\nPASS: VirtualBox VM reprovisioned and left powered off.\n'
    printf '  Name:      %s\n' "$vm_name"
    printf '  Platform:  ARM64 host CPU (%s virtual CPUs)\n' "$cpu_count"
    printf '  Memory:    %s MB\n' "$memory_mb"
    printf '  Disk:      %s MB, blank VDI\n' "$disk_size_mb"
    printf '  ISO:       %s\n' "$iso_path"
    printf '  Trace:     %s\n' "$trace_log"
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

prepare_trace_log
delete_existing_vm
recover_unregistered_vm
create_arm64_vm
verify_vm
