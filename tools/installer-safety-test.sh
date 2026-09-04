#!/bin/sh
set -eu

rg -Fq 'StorageProvisioningPlan' kernel/storage/mod.rs
rg -Fq 'StorageManager::provision' kernel/core/console.rs
rg -Uq 'self\s*\.storage_plan\s*\.as_ref\(\)' kernel/core/console.rs
rg -Fq 'confirmation popup cancelled' kernel/core/console.rs
rg -Fq 'destructive confirmation accepted' kernel/core/console.rs
rg -Uq 'installer_step == InstallerStep::Confirm[[:space:]]*\{[^}]*[[:space:]]0' kernel/core/console.rs
rg -Fq 'EVENT_INSTALLER_PLAN_CONFIRMED' kernel/core/console.rs
rg -Fq 'state=complete' kernel/storage/format.rs
rg -Fq 'provision_with_progress' kernel/storage/mod.rs
rg -Fq 'installer_reboot_countdown' kernel/core/console.rs
rg -Fq 'firmware_runtime_services' kernel/core/boot_info.rs
rg -Fq 'reset_system' kernel/core/console.rs
rg -Fq 'include_bytes!("../../assets/boot/infinity-installer-activation-v2.bmp")' kernel/core/bootstrap.rs
rg -Fq 'include_bytes!("../../assets/boot/infinity-storage-hierarchy-v3.bmp")' kernel/core/bootstrap.rs
rg -Fq 'fn installer_pool_panel(' kernel/core/bootstrap.rs
rg -Fq 'include_bytes!("../../assets/boot/infinity-disk-discovery-vision-v1.bmp")' kernel/core/bootstrap.rs
rg -Fq 'include_bytes!("../../assets/boot/infinity-storage-device-v1.bmp")' kernel/core/bootstrap.rs
rg -Fq 'fn installer_disk_discovery_panel(' kernel/core/bootstrap.rs
rg -Fq 'include_bytes!("../../assets/boot/infinity-date-time-world-v1.bmp")' kernel/core/bootstrap.rs
rg -Fq 'fn installer_date_time_panel(' kernel/core/bootstrap.rs
rg -Fq 'install_date_time_configuration(plan.date_time)' kernel/storage/format.rs
rg -Fq 'b"/system/settings/date-time"' kernel/storage/object.rs
rg -Fq 'StorageProfile::SharedDynamic' kernel/core/console.rs
if rg -Fq 'CHOOSE SPACE PRIORITY' kernel/core/bootstrap.rs; then
    echo 'FAIL: obsolete storage-priority screen remains in the installer' >&2
    exit 1
fi
rg -Fq 'b"DISKS DISCOVERED"' kernel/core/bootstrap.rs
rg -Fq 'b"MORE THAN STORAGE"' kernel/core/bootstrap.rs
rg -Fq 'storage_device: Option<crate::storage::StorageDevice>' kernel/core/bootstrap.rs
rg -Fq 'b"ONE DISK BECOMES"' kernel/core/bootstrap.rs
rg -Fq 'b"PART OF THE "' kernel/core/bootstrap.rs
rg -Fq 'b"INFINITY POOL."' kernel/core/bootstrap.rs
rg -Fq 'b"SYSTEM | PERSONAL | APPLICATIONS | RECOVERY"' kernel/core/bootstrap.rs
rg -Fq '2 => (135, 350)' kernel/core/bootstrap.rs
rg -Fq '2 => (510, 350)' kernel/core/bootstrap.rs
rg -Fq 'self.installer_step == InstallerStep::Hierarchy' kernel/core/console.rs
rg -Fq 'installer_screen == 7 && (content_redraw || focus_changed || pressed_changed)' kernel/core/bootstrap.rs
if rg -Fq 'installer_screen == 7 && (content_redraw || focus_changed || pointer_changed)' kernel/core/bootstrap.rs; then
    echo 'FAIL: destructive confirmation still repaints on every pointer motion report' >&2
    exit 1
fi
rg -Fq 'mov ecx, 28' boot/x86/bootstrap.asm
rg -Fq 'mov dword [BOOT_INFO + 8], 4' boot/x86/bootstrap.asm
if rg -n 'outb\(|outw\(' kernel/core/console.rs; then
    echo 'FAIL: installer UI contains raw block I/O' >&2
    exit 1
fi
echo 'PASS: destructive writes remain behind plan, modal motion is damage-limited, step-two topology and controls are versioned, and provisioner boundaries hold'
