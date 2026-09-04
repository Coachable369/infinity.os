# First-Boot Onboarding

Status: **TESTED** for durable state/recovery and keyboard-driven framebuffer
interaction in the final installed-disk image.

After a clean install, the installed kernel loads the native identity object.
If it is Required or InProgress it launches the Onboarding Service, not the
installer. The seven implemented stages are Welcome, Machine Identity, User
Identity, Profile, Authentication, AI and Voice, and Ready.

Each state-changing step commits the native System Space identity object before
advancing. After power loss, the wizard reconstructs the next safe stage from
the existing machine, user, and credential objects. If a credential already
exists, its owner must authenticate again; no secret or volatile form buffer is
persisted. Completion requires a machine, active user, credential, profiles,
and Personal Space ownership. It then authenticates a fresh Session and enters
the shell.

The artwork under `assets/identity/` contains no baked controls or text. Native
framebuffer controls supply focus, masked input, keyboard navigation, pointer
hit targets, and damage-limited redraws. Text entry repaints only the active
field, keeping the high-resolution generated scene stable and input responsive.
