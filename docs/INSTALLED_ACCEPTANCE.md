# Installed acceptance instrumentation

`tools/ms9-installed-acceptance.py --output /absolute/new/evidence-directory`
creates two new 32 GiB disks, drives the real installer through keyboard input,
and boots each installed disk with no CD-ROM attached. The initial runner covers
installation and detached first-boot onboarding, not the full MS9 lifecycle.
It preserves screenshots, disks and diagnostic logs for investigation. Logs are
not acceptance oracles. Existing user VMs and disks are not selected or modified.

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
