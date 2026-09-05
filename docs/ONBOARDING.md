# First-Boot Onboarding

Status: **TESTED** for durable state/recovery and keyboard-driven framebuffer
interaction in the final installed-disk image.

After a clean install, the installed kernel loads the native identity object.
If it is Required or InProgress it launches the Onboarding Service, not the
installer. The eight implemented stages are Welcome, Machine Identity, User
Identity, Profile, Authentication, AI and Voice, Network, and Ready.

Each state-changing step commits the native System Space identity object before
advancing. After power loss, the wizard reconstructs the next safe stage from
the existing machine, user, and credential objects. If a credential already
exists, its owner must authenticate again; no secret or volatile form buffer is
persisted. Completion requires a machine, active user, credential, profiles,
and Personal Space ownership. It then authenticates a fresh Session and enters
the shell.

The current onboarding artwork under `assets/desktop/` contains no baked
controls or text. Native framebuffer controls supply focus, masked input,
keyboard navigation, pointer hit targets, and damage-limited redraws. Text entry
repaints only the active field, keeping the high-resolution generated scene
stable and input responsive.

# Post-install network step

First boot includes a Network step after privacy/appearance selection and
before the final ready screen. The step reads typed `NetworkDevice` state from
the Network Runtime and offers Wired, Wi-Fi, and Continue Offline choices.
Mouse hit regions and Tab/Shift-Tab/arrow/Enter navigation address the same
semantic choices. Continue commits the selected native Network Profile and
setup mode to `/system/network/state` before onboarding completes.

Wired activation accepts discovered Ethernet and virtual Ethernet devices.
Wireless activation accepts an already-associated wireless device. Missing or
down links remain visible as unavailable and cannot be presented as connected.
Offline is always valid, preserves host-local operation, and can later be
changed in Settings.

Status: **TESTED** in the host behavioral harness; **IMPLEMENTED BUT UNTESTED
IN A VM** for the rendered first-boot screen; physical Wi-Fi discovery and
association remain **UNSUPPORTED**.
