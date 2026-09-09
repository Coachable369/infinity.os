# MS10 distributed completion pass

Status: IN PROGRESS. Do not mark MS10 COMPLETE or unblock MS11 from host tests.
Current request is the distributed Pool completion pass, not a new foundation.

## Source integration

- `storage_coordinator`: bounded automatic policy placement, durable claims,
  native secure IOP transfers, recipient verification, manifest CAS and recovery.
- `storage_pool_publication`: persistent explicitly approved publication leases,
  real native measurements, and reconnect only for already trusted peers.
- `storage_pool_deletion`: separately granted retirement and durable outbox acks.
- `fabric_pool_upload` / `object_pool_stream`: bounded 256 KiB objects in up to
  sixteen 16 KiB extents; ordinary inline object limit is unchanged. One durable
  upload, <=60 byte canonical wire windows, <=1 KiB verification per step.
- `fabric_pool_cache`: fixed verified immutable extent cache for transfer reads;
  general application reads do not silently trust cached disk integrity.
- `fabric_pool_deletion`: logical deletion commits retirement obligations with
  local COW reclamation. Offline recipients remain pending; they are not erased
  from the obligation list simply because their link is unavailable.
- `pool_commit_manifest`: manifest successor and bounded structured audit ring
  commit under one native root. This is not a claim that all audit acceptance is complete.
- `storage_view`: live owner-scoped Settings projection using the same typed
  service as Console; no renderer disk/network calls or GUI-only policy database.
- `storage_fixture`: explicit operator-approved acceptance producer. It generates
  deterministic bytes but uses ordinary owned IOP upload operations. It cannot
  create fake placements, replica receipts, health, or healed state.

All runtime modules compile into the existing CORE Replica Storage service in
both installed and live boot profiles. Service registration includes the new
upload and retirement operations; behavioral parity checks compare both profiles.
The generated Pool design kit is documentation, not a runtime panel bitmap.

## Bounded contracts

Temporary / Protected / Critical still require 1 / 2 / 3 independently verified
node domains. Missing capacity leaves real degradation. No grant is inferred
from discovery, pairing, a resource advertisement or a successful earlier chunk.

Persistent configuration is one canonical 512-byte native object. New format
reads prior configurations; leases and explicit operation grants persist, while
traffic keys, sessions and transient requests do not. Publication, transfer and
retirement use distinct grants. Expired or revoked authority fails closed.

Deletion retains version/admission fences in recipient tombstones. A retry is
idempotent; an old Begin cannot resurrect retired data. Native transactions protect
blocks referenced by the previously committed root until the replacement root
commits, fixing a failure-injection-discovered multi-record reuse bug.

## Evidence boundaries

- Prior source `c59d9a8`: local Pool operations and detached cold-reboot persistence
  passed on three independently installed QEMU nodes.
- Source `18d9f76`: latest three-node fresh-install bootstrap passed. Its separate
  installed secure/remote regression is still running; it does not test new source.
- Current source: expanded behavioral suites cover large upload/resume, ownership,
  COW reclamation, power cuts, recipient retirement, audit atomicity and GUI/Console
  policy parity. Final totals and exact commit must be recorded after integration.
- Full x86_64 installer kernel checks passed during integration. These are not
  final ISO builds or installed distributed acceptance.

## Mandatory remaining verification

1. Final-source ARM64/x86_64 builds and existing performance/security/installer gates.
2. Three or more independent final-source installed nodes with media detached.
3. Real >16 KiB Critical object distributed and independently hash-verified.
4. Replica-host removal, read while degraded, replacement/heal, stale return.
5. Independent copy/divergence, distributed delete/retirement/reclamation.
6. Cold reboot, persistent authority/configuration, resumed work and verified reads.
7. Installed Pool screenshots, keyboard/pointer/drag during real fabric activity.
8. Audit/event-gap reconstruction and complete failure matrix, with no text-oracle tests.

No final completion claim has been made. No Git remote is configured; local
mainline commits cannot be pushed or pulled until an actual remote is supplied.
