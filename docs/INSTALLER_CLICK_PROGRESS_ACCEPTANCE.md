# Installer click and measured progress acceptance — 2026-09-08

Source: mainline `17238f6`.

- Installer actions now activate on a new pointer press, not a potentially lost firmware release. The depressed frame is presented first. Held motion and release cannot advance a second step; destructive confirmation still defaults to Cancel and retains its existing authority check.
- EFI/kernel copying and read-back verification report completed sector work. Bootstrap creation reports runtime, registry, capability policy, local AI, voice/agent policy, identity/network/node trust and shell configuration stages. No simulated intermediate percentages or deliberate progress sleeps remain.
- Existing screen artwork/layout was preserved. The measured kernel-transfer screenshot was visually reviewed: component name, transferred/total KiB and overall percentage fit without clipping.

## TESTED

`./build.sh` passed on the final source, including host/security, graphics, object-store and installer-payload parity regressions.

`python3 tools/ms9-installed-acceptance.py --output /tmp/infinity-installer-click-progress-20260908-a --installer-clicks` exited 0. Two independent blank disks each passed seven single-press installer advances, held-motion/release non-activation, 58 monotonic progress publications ending at 100%, complete installation, and ISO-detached boot into installed onboarding. Assertions consumed binary state and real QEMU pointer events, not UI/log text.

Artifacts:

- x86_64 ISO SHA-256: `bfced048e40de8c9b5a365353c88eeda46f4512cd8dc61727c8582aa9e966edb`
- ARM64 ISO SHA-256: `8cb30f7874ad6d85f427f07c8d3a588bcc43b6da502b4e04430177478351cf8c`
- Evidence: `/tmp/infinity-installer-click-progress-20260908-a/`
- Reviewed screenshot: `/tmp/installer-measured-progress.png`

ARM64 compiled/package parity passed; this run is not VirtualBox interactive acceptance. It does not close remaining MS9 remote-lifecycle or rapid-input failures.
