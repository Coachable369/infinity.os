#!/bin/sh
set -eu
cd "$(dirname "$0")/.."

loader=boot/common/uefi_loader.c
driver=kernel/drivers/input/uefi.rs
ps2_driver=kernel/drivers/input/ps2.rs
pointer_codec=kernel/drivers/input/pointer.rs
virtualbox_setup=tools/configure-virtualbox-arm64.sh
virtualbox_guard=tools/guard-virtualbox-arm64-input.sh
virtualbox_start=tools/start-virtualbox-arm64.sh

# VirtualBox exposes its absolute tablet as HID protocol zero. The loader must
# retain that interface without forcing boot protocol onto it.
rg -q 'interface\.interface_protocol != 0 && interface\.interface_protocol != 1' "$loader"
rg -q 'usb_mouse_absolute\[slot\] = interface\.interface_protocol == 0' "$loader"

# An asynchronous relative mouse may coexist with the synchronous absolute
# tablet. Confirm that the async path cannot starve protocol-zero endpoints.
rg -Uq 'if pointers\.usb_mouse_async != 0 \{[[:space:][:print:]]*poll_async_usb_mouse\(pointers\);[[:space:][:print:]]*usb_mouse_absolute\[index\] != 0[[:space:][:print:]]*USE_ABSOLUTE_MOVEMENT && ABSOLUTE_UPDATED_THIS_POLL' "$driver"

# Bootstrap animation must never run ahead of input polling. Absolute tablet
# queues are collapsed to the newest sample and duplicate firmware/raw axes are
# suppressed, preventing the animated first screen from replaying stale motion.
rg -Uq 'pub fn run\(\) -> ! \{[[:space:][:print:]]*loop \{[[:space:][:print:]]*let input[[:space:][:print:]]*crate::bootstrap::animation_tick\(\);' "$driver"
rg -q 'let report_budget = if absolute \{ 16 \} else \{ 4 \}' "$driver"
rg -q 'let mut latest_absolute = None' "$driver"
rg -q 'USE_ABSOLUTE_MOVEMENT && ABSOLUTE_UPDATED_THIS_POLL' "$driver"
rg -q 'note_pointer_activity' kernel/core/console.rs
rg -q 'POINTER_ACTIVITY_GRACE_TICKS = 30' kernel/core/bootstrap.rs
rg -Uq 'if POINTER_ACTIVITY_GRACE_TICKS != 0 \{[[:space:][:print:]]*POINTER_ACTIVITY_GRACE_TICKS -= 1;[[:space:][:print:]]*true' kernel/core/bootstrap.rs
rg -q 'delta_x != 0 || delta_y != 0 || button_changed' kernel/core/console.rs
rg -Uq 'VirtualBox ARM.*Absolute Pointer[[:space:][:print:]]*stationary firmware state notification[[:space:][:print:]]*self\.pointer_interaction\(true\);[[:space:][:print:]]*self\.pointer_interaction\(false\);' kernel/core/console.rs
rg -q 'installer_stationary_pointer_target' kernel/core/console.rs
rg -Uq 'let defer_for_pointer = unsafe \{[[:space:][:print:]]*POINTER_ACTIVITY_PENDING = false;[[:space:][:print:]]*if POINTER_ACTIVITY_GRACE_TICKS != 0[[:space:][:print:]]*\};[[:space:][:print:]]*if defer_for_pointer \{[[:space:][:print:]]*return;' kernel/core/bootstrap.rs

# Oracle VirtualBox USBHIDT_REPORT is packed as buttons, dz, dw, padding,
# X(u16 LE), Y(u16 LE), with both axes declared in the 0..0x7fff range.
rg -q 'decode_usb_absolute_pointer' "$driver"
rg -q 'u16::from_le_bytes\(\[report\[4\], report\[5\]\]\)' "$pointer_codec"
rg -q 'u16::from_le_bytes\(\[report\[6\], report\[7\]\]\)' "$pointer_codec"
rg -q 'value\.min\(0x7fff\).*0x7fff' "$pointer_codec"

# Initial discovery must preserve every common pointer class used by physical
# and virtual machines: legacy PS/2, wheel/five-button extensions, relative
# USB HID mice, absolute USB HID tablets/touchpads, and UEFI pointer protocols.
rg -q 'negotiate_mouse_protocol' "$ps2_driver"
rg -q 'mouse_set_sample_rate\(200\).*' "$ps2_driver"
rg -q 'TRANSPORT_UEFI_RELATIVE' "$driver"
rg -q 'TRANSPORT_UEFI_ABSOLUTE' "$driver"
rg -q 'TRANSPORT_USB_HID_RELATIVE' "$driver"
rg -q 'TRANSPORT_USB_HID_ABSOLUTE' "$driver"
rg -q 'FEATURE_BUTTONS_EXTENDED' "$pointer_codec"

# VirtualBox ARM defaults new Other/Unknown guests to PS/2 input even though
# that virtual machine has no usable PS/2 pointer path. The shipped preflight
# must require the USB tablet, USB keyboard, and xHCI controller together.
rg -q -- '--usb off --usb-xhci on --mouse usbtablet --keyboard usb' "$virtualbox_setup"
rg -q 'Pointing Device:.*USB Tablet' "$virtualbox_setup"
rg -q 'xHCI USB:.*enabled' "$virtualbox_setup"
rg -q 'usb="off"' "$virtualbox_setup"
rg -q 'InfinityOS-aarch64\\\.iso.*infinity-aarch64\\\.iso' "$virtualbox_guard"
rg -q 'infinityos|infinitybox|infinity apple arm' "$virtualbox_guard"
rg -q 'configure-virtualbox-arm64\.sh' "$virtualbox_start"
rg -q 'guard-virtualbox-arm64-input\.sh' build.sh
rg -q 'start-virtualbox-arm64\.sh' build.sh

echo 'PASS input regression: PS/2, UEFI, USB HID, VirtualBox ARM preflight, latency-safe bootstrap scheduling, packet layout, and capability discovery'
