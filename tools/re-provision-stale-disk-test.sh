#!/bin/sh
set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
temp_base=${TMPDIR:-$project_root/build/tmp}
mkdir -p "$temp_base"
test_root=$(mktemp -d "${temp_base%/}/infinity-reprovision-test.XXXXXX")
test_root=$(CDPATH= cd -- "$test_root" && pwd -P)
trap 'rm -rf "$test_root"' EXIT HUP INT TERM

# ------------------------=
# FUNC: fail
# DESC: Stop the behavioral test when an expected state transition is absent.
# ------------------=
fail() {
    printf 'FAIL: %s\n' "$1" >&2
    exit 1
}

# ------------------------=
# FUNC: create_fake_host_commands
# DESC: Provide deterministic ARM64 host and ISO inspection behavior to the test.
# ------------------=
create_fake_host_commands() {
    fake_bin=$test_root/bin
    mkdir -p "$fake_bin"

    cp "$project_root/tools/test-fixtures/re-provision/VBoxManage" "$fake_bin/VBoxManage"
    cp "$project_root/tools/test-fixtures/re-provision/file" "$fake_bin/file"
    cp "$project_root/tools/test-fixtures/re-provision/uname" "$fake_bin/uname"
    chmod +x "$fake_bin/VBoxManage" "$fake_bin/file" "$fake_bin/uname"
    mkdir -p "$test_root/tools" "$test_root/builds"
    cp "$project_root/re-provision.sh" "$test_root/re-provision.sh"
    cp "$project_root/tools/select-install-iso.sh" "$test_root/tools/"
}

# ------------------------=
# FUNC: run_orphan_replacement_case
# DESC: Verify reprovisioning replaces an unregistered stale VDI at the target path.
# ------------------=
run_orphan_replacement_case() {
    case_root=$test_root/orphan
    vm_directory=$case_root/VirtualBox\ VMs/infinityos-4
    iso_path=$test_root/builds/InfinityOS-aarch64.iso
    disk_path=$vm_directory/infinityos-4.vdi
    state_path=$case_root/state
    mkdir -p "$vm_directory" "$state_path"
    printf 'stale\n' > "$disk_path"
    printf 'test-media\n' > "$iso_path"
    printf 'old\n' > "$state_path/vm-state"

    PATH="$fake_bin:$PATH" \
        TEST_VM_DIRECTORY="$vm_directory" \
        TEST_ISO_PATH="$iso_path" \
        TEST_STATE_PATH="$state_path" \
        TEST_MEDIUM_MODE=orphan \
        INFINITY_VM_MEMORY_MB=22480 \
        INFINITY_VBOXMANAGE="$fake_bin/VBoxManage" \
        sh "$test_root/re-provision.sh" infinityos-4 "$iso_path" >/dev/null

    test "$(sed -n '1p' "$disk_path")" = fresh || fail "The stale VDI was not replaced."
    test -f "$state_path/unregistered" || fail "The old VM was not unregistered."
    test -f "$state_path/medium-inspected" || fail "The stale VDI registration was not checked."
    test "$(sed -n '1p' "$state_path/memory")" = 22480 || fail "The configured memory default was not applied."
    test "$(sed -n '1p' "$state_path/graphics")" = vmsvga || fail "VMSVGA graphics were not applied."
    test "$(sed -n '1p' "$state_path/mouse")" = usb || fail "Generic USB HID mouse input was not applied."
}

# ------------------------=
# FUNC: run_attached_medium_case
# DESC: Verify reprovisioning preserves a VDI that VirtualBox reports as attached.
# ------------------=
run_attached_medium_case() {
    case_root=$test_root/attached
    vm_directory=$case_root/VirtualBox\ VMs/infinityos-4
    iso_path=$test_root/builds/InfinityOS-aarch64.iso
    disk_path=$vm_directory/infinityos-4.vdi
    state_path=$case_root/state
    mkdir -p "$vm_directory" "$state_path"
    printf 'attached\n' > "$disk_path"
    printf 'test-media\n' > "$iso_path"
    printf 'old\n' > "$state_path/vm-state"

    if PATH="$fake_bin:$PATH" \
        TEST_VM_DIRECTORY="$vm_directory" \
        TEST_ISO_PATH="$iso_path" \
        TEST_STATE_PATH="$state_path" \
        TEST_MEDIUM_MODE=attached \
        INFINITY_VBOXMANAGE="$fake_bin/VBoxManage" \
        sh "$test_root/re-provision.sh" infinityos-4 "$iso_path" >/dev/null 2>&1; then
        fail "An attached VDI was replaced."
    fi

    test "$(sed -n '1p' "$disk_path")" = attached || fail "The attached VDI was modified."
    test -f "$state_path/close-attempted" || fail "VirtualBox was not asked to safely close the registered VDI."
}

# ------------------------=
# FUNC: run_unregistered_machine_case
# DESC: Verify a stale unregistered settings file is safely registered, deleted, and replaced.
# ------------------=
run_unregistered_machine_case() {
    case_root=$test_root/unregistered
    vm_directory=$case_root/VirtualBox\ VMs/infinityos-4
    iso_path=$test_root/builds/InfinityOS-aarch64.iso
    disk_path=$vm_directory/infinityos-4.vdi
    config_path=$vm_directory/infinityos-4.vbox
    state_path=$case_root/state
    mkdir -p "$vm_directory" "$state_path"
    printf 'stale-settings\n' > "$config_path"
    printf 'stale-disk\n' > "$disk_path"
    printf 'test-media\n' > "$iso_path"

    PATH="$fake_bin:$PATH" \
        TEST_VM_DIRECTORY="$vm_directory" \
        TEST_ISO_PATH="$iso_path" \
        TEST_STATE_PATH="$state_path" \
        TEST_MEDIUM_MODE=orphan \
        INFINITY_VBOXMANAGE="$fake_bin/VBoxManage" \
        sh "$test_root/re-provision.sh" infinityos-4 "$iso_path" >/dev/null

    test -f "$state_path/unregistered" || fail "The recovered VM was not deleted through VirtualBox."
    test -f "$config_path" || fail "The replacement VM settings were not created."
    test "$(sed -n '1p' "$disk_path")" = fresh || fail "The recovered VM disk was not replaced."
    test "$(sed -n '1p' "$state_path/vm-state")" = new || fail "The replacement VM was not registered."
}

create_fake_host_commands
run_orphan_replacement_case
run_attached_medium_case
run_unregistered_machine_case
printf 'PASS: stale VM recovery and VDI replacement are idempotent; attached media is preserved.\n'
