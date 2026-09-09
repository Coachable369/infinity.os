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
- Native lifecycle commits append a bounded structured audit record under the same
  root: create, copy, update, upload publication, policy/CAS, delete, recipient
  Available. Transfer windows do not emit an audit record per chunk.
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
- Source `18d9f76`: three-node fresh-install bootstrap passed. Separate installed
  secure allow/deny/revocation and remote Pool creation passed. The next read was
  denied after its one-hour secure session expired; the run failed honestly and
  did not verify remaining remote-read/publication steps.
- Current source: expanded behavioral suites cover large upload/resume, ownership,
  COW reclamation, power cuts, recipient retirement, audit atomicity and GUI/Console
  policy parity. Final totals and exact commit must be recorded after integration.
- Full x86_64 installer kernel checks passed during integration. These are not
  final ISO builds or installed distributed acceptance.
- Checkpoint `133a2a3`: `./build.sh` completed for both ISO architectures, including
  packaged parity and existing regression gates. Subsequent acceptance-only
  read preference/byte-verifier integration requires another source build.
- Integrated lifecycle host rerun: 99 fabric tests passed (one ignored), and 59 performance
  tests passed. Pointer, redraw and File Navigator executable harnesses passed.
  Missed-event projection recovery and unchanged-refresh suppression passed.
- Lifecycle checkpoint `52c15e8` adds explicit post-commit transition flags,
  bounded overflow/reconstruction signaling, atomic lifecycle audit coverage and
  exact async read-consumption identity. It requires final-source installed proof.
- The `5869efc` four-node install is an intermediate regression baseline. Its
  preflight stops before remote reads because that build lacks the corrected
  consumed-read diagnostic. This boundary is not counted as full acceptance.
  All four independent installs passed detached cold boot, persistent identity,
  authentication and native storage inspection (`/tmp/infinity-ms10-distributed-installed-5869efc/result.json`).
- `52c15e8`: full `./build.sh` completed successfully for both architectures
  (`/tmp/ms10-lifecycle-final-build.log`). Fresh four-node installed verification
  is running separately under `/tmp/infinity-ms10-distributed-installed-52c15e8`.
  ARM64 ISO SHA-256: `c14d1ea149e4cca997daba414638164c6b278829c5c9877ee29daf2a5dad104f`.
  x86_64 ISO SHA-256: `e8bbed01cb6d7ce2b8ac5fcf7fe3657e9aac1f954c50a5ce720b0bbda73339b7`.

## Read and byte-proof boundaries

`pool read obj:<id> generation=<g> version=<v> offset=<n> length=64 source=remote`
uses the asynchronous typed coordinator with `RemoteVerified` preference (bit 63
of ObjectRead.value; remaining bits are bounded length). The caller supplies no
peer. Live scope, session, grants, version and full-chunk integrity remain checked.
The default remains local preferred. The synchronous backend does not implement
this preference; native callers submit/take the coordinator request.
This proves remote resolution, not a simulated local fault or authority-node-loss
survivability. Those are separate acceptance claims.

Authority-node-offline normal reads are currently UNSUPPORTED, not merely untested.
Recipients persist replica bindings/content, not the owner's Pool catalog,
committed manifest and namespace. The coordinator loads locally owned manifests,
and recipient read authority requires the authenticated owner peer. Completion
therefore also requires explicitly delegated read authority and replicated
committed metadata with a defined freshness guarantee. Selecting the highest
observed replica version or silently assuming ownership is not a safe substitute.

The read-only installed-disk verifier opens raw images without write permission
and uses native catalog/extent decoding plus a complete content hash. Paused-VM
inspection after a committed Available receipt is persisted-byte evidence, not
cold-reboot evidence. Cold boot remains a separate mandatory phase.

## Mandatory remaining verification

1. Preserve the passing final-source ARM64/x86_64 builds and existing
   performance/security/installer gates through any subsequent runtime fixes.
2. Three or more independent final-source installed nodes with media detached.
3. Real >16 KiB Critical object distributed and independently hash-verified.
4. Replica-host removal, read while degraded, replacement/heal, stale return.
5. Independent copy/divergence, distributed delete/retirement/reclamation.
6. Cold reboot, persistent authority/configuration, resumed work and verified reads.
7. Installed Pool screenshots, keyboard/pointer/drag during real fabric activity.
8. Audit/event-gap reconstruction and complete failure matrix, with no text-oracle tests.
9. Implement and verify ordinary same-ObjectId/namespace reads on surviving nodes
   while the authoritative owner is offline, including metadata freshness and
   explicit delegated read authority.

No final completion claim has been made. No Git remote is configured; local
mainline commits cannot be pushed or pulled until an actual remote is supplied.
