# InfinityOS testing

`make milestone-9-correlation-test` runs nine HOST router-boundary tests, including
durable mutation rollback and complete Policy.Read reconstruction.
`make milestone-9-durable-mutation-test` selects the three durable cases.
Both are included in `make milestone-9-test` and therefore `./build.sh`.
See the [final-completion matrix](MILESTONE_9_FINAL_COMPLETION_STATUS.md): these
targets do not certify the mandatory detached-media two-node lifecycle.

`make milestone-9-remote-iop-test` adds native two-guest remote read, engineering
mutation, policy, scoped-grant, queued-revocation and timeout assertions to 9-A.
It is NOT the full mandatory `9b-test`; see [coverage gaps](MILESTONE_9B_CONTROL_PLANE.md).

`make milestone-9-wire-trust-test` runs two independent native engineering guests
using the production node transport and E1000/UDP path. External binary operator
commands compare independent verification material and explicitly approve it.
See [9-A evidence and limits](MILESTONE_9A_WIRE_TRUST.md). This is not installed GUI
or detached-media acceptance; log text is never a test oracle.

`make milestone-9-test` behaviorally verifies identity, discovery, pairing, sessions, replay rejection, remote authority, mesh membership, bounds, persistence, and corruption handling. Installed boot is checked from framebuffer pixels and structured disk state, never rendered text. Two independently installed networked-node acceptance remains **SCAFFOLDED**.

## Milestone 8 native networking

Run `make network-test` for behavior-only IPv4/IPv6 type behavior, route
selection, post-install network-mode activation and persistence, onboarding
network-row pointer targets, profile transaction/rollback/persistence,
per-identity policy,
capability revocation and leases, bounded connection queues, resolver
deadlines/cache expiry, discovery saturation/expiry, network IOP codecs,
Console operation mapping, Network-domain IEF delivery, metadata isolation, and
service restart. The test does not use rendered text or source searches as an
oracle.

## Installed component manifest boundary

Run `make component-manifest-test` to encode the complete current System
Generation registry into its production two-sector record, validate the final
component beyond the first-sector boundary, verify the native checksum, and
prove corruption in the second sector is rejected. The test asserts structured
binary state and does not use rendered text as an oracle.

Run `./build.sh` for all architecture ISOs and packaged System Generation
validation. Physical NIC, DNS, secure wire connection, and detached-media
clean-install acceptance cannot pass until a supported adapter exists and must
remain reported as unsupported.

## Milestone 7C compositor and Window Server

Run `make milestone-7c-test` for behavioral surface ownership and allocation
bounds, typed damage collapse, coherent damage-only presentation, privileged
z-order denial, secure input routing, pointer capture, rapid drag paths,
context-failure cleanup, priority deferral/convergence, IOP codecs, capability
revocation, and IEF correlation. The same target compile-checks x86_64 and
AArch64 kernels. It does not use rendered copy or source text as an acceptance
oracle.

Run `./build.sh`, then `make ui-install-parity-test` to extract and byte-compare
the real live and installed UI payload containers. Direct visual acceptance
requires booting the ARM64 ISO in VirtualBox and exercising onboarding/login,
cursor-only motion, title-bar capture, window movement, resize, menus, and
exposed wallpaper while observing for whole-frame flashes, stale pixels, and
cursor trails. Installed-disk-only acceptance remains separate and must never
be inferred from host tests.

The 2026-09-04 AArch64 VirtualBox pass behaviorally completed live-media boot,
disk discovery, destructive confirmation, animated installation, reboot
countdown, installed-System-Generation boot, onboarding, and desktop entry.
Pointer selection and timezone-map selection worked. Two independent automated
title-bar drag gestures left the Home window in its original bounds, so direct
window-relocation acceptance remains open even though the host drag and bounded
damage harnesses pass.

## Milestone 7.x

Run `make milestone-7x-test` for skin compiler safety, native package integrity,
scale/layout checks, focus and pointer behavior, retained damage, alternate
activation/rollback/SafeSkin, window ownership denial, and install-parity source
checks. Run `./build.sh` for all architecture images and `make install-test` for
fresh x86_64 installation followed by detached-media boot.

## Milestone 6 native AI host acceptance

```sh
make ai-test
make milestone-6-test
make milestone-6-5-test
```

`ai-test` validates model-object integrity/corruption, actual quantized CPU
classification, provider privacy selection, inference deadline/cancellation and
queue saturation, context minimization, Tool Broker capability/revocation,
destructive-plan rejection, microphone lease enforcement, honest unavailable
speech behavior, independent agent tools/budgets, service discovery, and
capability-filtered correlated AI events. It also rejects traditional
filesystem/process/JSON shortcuts in the AI runtime source.

`milestone-6-test` combines that suite with native object-store and Runtime/IOP/
IEF acceptance. VM boot and detached-media installation remain separate tests;
host success must not be described as VM proof.

## Milestone 5.x native installed boot

```sh
make install-boot-test
```

This resets only `build/infinity-test-disk.raw`, drives the real recovery UI, checks generation lifecycle and component verification markers, terminates that VM, and starts a second QEMU command containing only OVMF plus the target disk. It requires native-boot, valid-manifest, valid-kernel, runtime, storage, and Infinity Console markers and fails if the Recovery Node marker appears.

Two follow-up disk copies verify that INSTALLING state and kernel corruption both produce `No valid ACTIVE system generation found` and never reach `Kernel online.`. Individual reruns are `make boot-installed` and `make system-generation-test` after a successful install.
# Milestone 7

Run `make milestone-7-test` for stable-ID, password-verifier, rate-limit,
revocation, cross-user isolation, Personal Space, lock/unlock/logout,
profile-persistence, corruption, interrupted-onboarding, Console-schema, and
service-registry coverage. The test also audits the new native modules for
filesystem, process, socket, JSON/YAML, and credential-exposure shortcuts.
