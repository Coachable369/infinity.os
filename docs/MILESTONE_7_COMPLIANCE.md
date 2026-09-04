# Milestone 7 Compliance

## Architecture status

| Area | Status |
|---|---|
| Stable machine/user IDs and native persistence | TESTED |
| Password verifier, backoff, revocation | TESTED |
| Session capability isolation and Personal Spaces | TESTED |
| Interrupted onboarding recovery | TESTED |
| Typed GUI/CLI operation registry | TESTED |
| Six dependency-ordered services | TESTED |
| Graphical onboarding and authenticated desktop entry | TESTED in QEMU from the installed disk |
| Graphical menu, Settings, and lock | TESTED in QEMU from an authenticated installed session |
| Graphical logout | IMPLEMENTED BUT UNTESTED end to end |
| Installed-disk first-boot end-to-end automation | TESTED with paced QEMU keyboard input |
| Hardware-backed authentication and multiple concurrent GUI sessions | PLANNED |

## Deliberate limits

The kernel remains single-address-space, so service isolation is the strongest
practical logical/capability boundary rather than a claimed MMU boundary.
Passwords use an internal PBKDF2 provider until a replaceable memory-hard or
hardware-backed provider is available. The graphical shell is intentionally a
foundation, not the Milestone 8 compositor or third-party application runtime.
