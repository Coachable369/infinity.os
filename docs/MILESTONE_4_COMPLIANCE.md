# Milestone 4 compliance

## Acceptance status

| Requirement | Status | Evidence / limitation |
|---|---|---|
| Execution Context, stable security identity, lifecycle | TESTED | Host acceptance creates/destroys contexts and separates handles from 128-bit identities. |
| Practical isolation | TESTED | Non-overlapping owned memory regions and explicit endpoints. No MMU enforcement yet. |
| Basic scheduler | TESTED | Cooperative weighted-class fairness, block/wake, idle accounting. Preemption is SCAFFOLDED. |
| Modular Runtime Core | TESTED | Six separate managers compile in all target kernels. |
| Manifests, graph, readiness, discovery | TESTED | Typed versioned manifests, cycle rejection, dependency start, explicit Ready, operation-provider lookup. |
| Fault isolation/restart/degraded mode | TESTED on host | Non-critical context removed and restarted after backoff; critical failure enters degraded mode; failure event delivered. VM crash injection is SCAFFOLDED. |
| Capabilities | TESTED | Grant, validation, subset delegation, constraints, live/ancestor revocation, and leases. |
| IOP | TESTED on host | Explicit 80-byte binary codec, request/response echo, capability denial, timeout, cancel hook, bounded backpressure. In-kernel transport acceptance is IMPLEMENTED BUT UNTESTED because returning the full message through the current bootstrap ABI faulted the diagnostic probe. |
| Event Fabric | TESTED on host | Typed classes/domains, filters, creation/delivery checks, lease, sequence gap, reconciliation, overflow/coalescing, rate gate, correlation/causation. |
| Durable Record foundation | TESTED | Fixed append-before-publish memory log plus persisted System security-bootstrap record. General durable stream is SCAFFOLDED. |
| Resource governance | TESTED on host | Memory and message admission, CPU ticks, priority metadata, console projection. Hard CPU/I/O enforcement is SCAFFOLDED. |
| Installer Service capabilities | TESTED in QEMU | Live-only leased authority gates provisioning; confirmation/completion Record events; completion revokes authority. |
| Runtime System objects | TESTED in QEMU | Runtime, service registry, and policy are written, independently remounted, validated, and booted from installed disk. |
| ISO/live and installed profiles | TESTED in QEMU | Same code; installed profile omits installer; boot succeeds with ISO detached. |
| Console operations | TESTED in installed QEMU | Service/runtime/capability/event/resource commands and natural-language Service.List alias. |
| Driver isolation preparation | SCAFFOLDED | Device Service has a context/endpoint. Hardware drivers remain privileged; input normalization is the next low-risk migration candidate. |

## Architecture support

| Architecture | Build | Boot smoke | Installed runtime |
|---|---|---|---|
| x86_64 UEFI | TESTED | TESTED in QEMU | TESTED in QEMU without ISO |
| AArch64 UEFI | TESTED | TESTED in QEMU | IMPLEMENTED BUT UNTESTED |
| x86 BIOS | TESTED | TESTED in QEMU | PLANNED (installer is UEFI-only) |

## Exact commands

```sh
make runtime-test
make object-test
make install-test
make runtime-vm-test
make object-vm-test
make test
make milestone-4-test
./build.sh
```

`make milestone-4-test` creates and erases only `build/infinity-test-disk.raw`, boots the ISO, installs, powers the VM off, boots the disk without installation media, exercises runtime console operations, and verifies object persistence through further reboots.

## Temporary compromises

- Context memory ownership is logical, not page-table protected.
- Services are separate typed contexts/components but are statically linked in one privilege domain.
- Scheduler is cooperative and single-core.
- Runtime limits are fixed-capacity because no general kernel allocator exists.
- The event Record log is bounded in memory; only bootstrap security metadata is durably stored today.
- IOP request/response semantics are host-tested from the exact shared module; the boot-time diagnostic was removed after it exposed a current ABI/stack-boundary fault. Router/service startup is VM-tested.
- The bounded service registry scans at most 16 entries; indexed lookup is deferred until scale requires it.

The next recommended architectural milestone is an architecture-specific x86_64 protection layer: page tables per Execution Context, guarded kernel stacks, saved-register context switching, and a user/kernel IOP syscall boundary. It should migrate the non-boot-critical input-normalization component first, then repeat the existing fault, capability, and backpressure suite across the hardware boundary.

