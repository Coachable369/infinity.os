# Installed acceptance instrumentation

`tools/ms9-installed-acceptance.py --output /absolute/new/evidence-directory`
creates two new 32 GiB disks, drives the real installer through keyboard input,
and boots each installed disk with no CD-ROM attached. The initial runner covers
installation and detached first-boot onboarding, not the full MS9 lifecycle.
It preserves screenshots, disks and diagnostic logs for investigation. Logs are
not acceptance oracles. Existing user VMs and disks are not selected or modified.

`--resume-installed` continues only disks owned by a previous run, completes the
real account wizard, then terminates and cold-boots each guest without installation
media. It checks authentication and preservation of the public node identity.

`--mesh-installed` continues those authenticated installations on a private virtual
Ethernet link. It uses native Settings for static addressing, the native Command
Window for explicit peer endpoints, and the node Settings controls for discovery,
dual operator confirmation and a fresh secure session. Each stage asserts binary
state transitions. This phase does not yet cover the entire required MS9 lifecycle.
The runner applies keyboard backpressure using actual guest snapshot progress;
passing with that pacing would not prove fast-typing or UI performance acceptance.

## Observed acceptance, 2026-09-08

Two independent blank-disk installations, detached-media account setup and
authenticated cold boots passed in `/tmp/infinity-ms9-installed-20260908-a`.
Both nodes preserved distinct public identities. These were QEMU x86_64 installed
guests, not physical VirtualBox or ARM64 proof. `install-result.json` and
`onboarding-result.json` record those specific boundaries.

The subsequent installed mesh run exposed dropped keyboard input in the network
editor and Command Window. Static address commit passed after slower input, but
the peer endpoint transaction did not commit: its checkpoint and configured-link
count remained zero. Captured screenshots also show overlapping header text and
clipped Network details. Pairing, membership, installed remote IOP and installed
UI responsiveness must not be marked passed on this evidence.

The final retry, paced against observed event-loop progress, successfully committed
the endpoint intent: authoritative checkpoint 1, projection checkpoint 1, one
configured link and one audit record. It then timed out with zero connections and
zero discovered peers. The remaining failure is native endpoint binding, not a
failed durable commit. Evidence: `/tmp/ms9-installed-mesh-paced.log` and the
node-1 binary snapshot/screenshot in the directory above. Work stopped after the
two correction loops required by repository instructions; both test guests exited.
Slow-input pacing does not resolve or certify high-rate keyboard responsiveness.

The ISO used for these runs precedes commits `fbc8900` (launcher/policy controls)
and `dbd6e29` (trusted input dismissal). Those changes require a new ISO build and
installed verification; the existing build must not be represented as containing
them. No repository remote is configured for publishing these commits.

The debugger reads two fixed binary exports using QEMU physical-memory reads.
ELF symbols locate the data; symbol presence alone cannot pass an acceptance test.
Neither export accepts commands, grants capabilities, or changes authoritative
state. Reading them requires the same host authority as reading all guest memory.
Do not expose the VM management socket to untrusted users.

## UI/service snapshot, schema 1

`INFINITY_DIAGNOSTIC_SNAPSHOT` contains 512 little-endian u64 words. Word 0 is
`0x494e464449414731`, word 1 is the schema, and words 2/511 must match and be even.
An odd generation denotes an in-progress write. The snapshot is published on the
existing UI clock; a stalled clock must not be interpreted as fresh evidence.

| Words | Observed value |
|---|---|
| 3 | Installed boot (1) versus live installer (0) |
| 4–8 | Console mode, installer step, installer focus, configuration step, Settings section |
| 9 | User present, session present, editing, validation-error bits |
| 10–12 | Observed service clock, framebuffer width, framebuffer height |
| 16–19 | Local public NodeId |
| 20–23 | Authoritative checkpoint, projection checkpoint, stale flag, detected gaps |
| 24–30 | Discovered peers, trusted peers, established sessions, projected active domains, projected pending domains, configured links, audit records |
| 32–47 | Selected peer, pairing handle/code/expiry/state, transcript fingerprint, transaction ID |
| 48–49 | Connection and capability counts |
| 50–53 | Native endpoint binding errors, zero for none, otherwise NetworkError discriminator plus one |
| 128–383 | Sixteen 128-byte public node projection records |
| 384–487 | Eight 104-byte public domain projection records |

Projection records remain explicitly distinguished from authoritative checkpoints.
The pairing values are the actual authenticated verification view. No password,
private signing key, traffic key, or payload is copied.

## Frame measurements

`INFINITY_DIAGNOSTIC_FRAMES` contains 10802 little-endian u64 words. Word 0 is
schema 1; word 1 is the latest completed sequence. The remaining 3600 ring slots
contain `(sequence, measured frame_ns, observed timestamp_ns)`; `u64::MAX` means
unavailable. Sequence N occupies slot `(N - 1) % 3600`. Capture while paused or
validate slot sequences when reading a live guest. Percentiles are calculated by
the host from actual captured samples, never sorted in the guest input loop.
The additional recording work is constant-sized and allocation-free per frame.

## Explicit peer endpoints

The installed Console supports typed operations through the ordinary IOP broker:

```
node link-configure 1 local=10.42.0.1 remote=10.42.0.2 local-port=49152 remote-port=49152
node link-list
node link-remove 1
node session-open node:<full-peer-id>
```

Four persistent endpoint slots are included in the native node-state object.
The runtime retries unavailable NIC/address/route bindings once per second and
reclaims exact endpoint policy rules and capabilities on removal or failed setup.
Link configuration does not fabricate discovery, trust, membership, or a session.
Session-open reports a pending handshake, not established connectivity.
