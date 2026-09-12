# Installed UEFI boot handoff

VirtualBox's configured disk-first order did not reliably override its existing
UEFI boot variables. A completed installation now creates an active Boot####
load option containing the exact installed ESP GPT GUID, start LBA and length,
plus the architecture's EFI fallback loader path. It prepends that option to
BootOrder without dropping other entries and sets BootNext for the first reboot.
The ISO remains explicitly bootable for recovery or reinstall.

Registration occurs only after successful provisioning. Existing boot options
are not overwritten. Firmware failures stop the automatic reboot and report
that the installer medium must be removed before restart.

Behavioral tests validate encoded partition identity, preserved prior order,
one-shot target and failure handling. ARM64 cargo check passes. A fresh-install
reboot with the ISO still attached is required for firmware acceptance; host
tests are not evidence that a particular firmware honored the variables.

Reference: https://uefi.org/specs/UEFI/2.10/03_Boot_Manager.html
