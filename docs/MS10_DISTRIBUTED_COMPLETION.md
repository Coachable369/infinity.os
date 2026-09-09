# MS10 distributed completion pass

Status: ACTIVE — INCOMPLETE. The final closure request explicitly supersedes the
previous two-correction-loop stop for this pass. Do not mark MS10 COMPLETE or
unblock MS11 from host tests.
Current request is the distributed Pool completion pass, not a new foundation.

## Current closure checklist

- [x] Correct Settings → Desktop → Launcher acceptance helper. Three installed
  navigation cycles passed on the preserved ISO-detached node-1; helper behavioral
  suite passed. Source commit `7c9f18b`.
- [x] Repeat installed protected pairing with explicit reset and Settings refresh:
  three distinct two-sided ceremonies passed on the interim `40ccb79` runtime.
  Receipt: `/tmp/ms10-final-pacing-installed/repeated-pairing-trace.json`.
  This does not certify later metadata integration.
- [ ] Restore bounded authenticated DATA pacing; adversarial, fairness,
  backpressure, performance, build and installed timing proof.
- [ ] Integrate durable current metadata, namespace and scoped delegated normal
  reads while the original owner is offline; fence stale owner return.
- [ ] Installed 64–256 KiB three-copy hashes, owner loss/read/degradation,
  replacement healing, COW/reclamation, event gap, GUI/Console and cold persistence.
- [ ] Final-source clean installation and full security/input/performance gates.

### Current source evidence (not installed lifecycle acceptance)

Final-source checkpoint `338bb68`: `./build.sh` passed
(`/tmp/ms10-final-338bb68-iso-build.log`). The x86_64 ISO SHA-256 is
`56811099b48c4360a685693abda53573e636f16c8a2162583878b57c4946ada3`;
ARM64 is `166aea25601d21587c3f5ea44f519baa39ff5fc4ba1d10a16f754809a7d86137`.
This includes real Pool IEF gap reconstruction, fresh shared ObjectInspect,
observed health shared with Settings, and corrected Console health rendering.
Fabric remains 128 passed / one existing ignored; 18 acceptance-helper behavioral
tests pass. These new runtime paths still require installed verification.

The `8b3c995` four-node run passed all six protected pairings and committed the
directed metadata/replica configuration, then exposed a harness session-count
bug: automatic connections already provided three sessions, but the helper
waited for a fourth. All recorded transport error fields were zero. `994f0e9`
instead requires the exact peer's live established wire transaction on both
sides. A prepared-state resume preserves the failure evidence, skips pairing
and replica grants, and explicitly configures publication. No bulk transfer
or owner-offline success is inferred from this setup progress.

Corrected `8b3c995` runtime: full `./build.sh` passed
(`/tmp/ms10-stack-corrected-iso-build.log`). Four new independent disks passed
ISO-detached cold boot, onboarding/authentication, unique persistent identities
and native storage inspection. Receipt:
`/tmp/ms10-stack-corrected-8b3c995/result.json`; pinned artifacts are beside it.
The installed kernel SHA-256 is
`fb9908815556845e0025cf41c1d025b1c07c5420e4d6a73f45933dedd2036849`.
This closes the detached-startup regression, not the active distributed lifecycle.

The first integrated clean install completed on two disks but failed on detached
onboarding boot. The preserved CPU trace `/tmp/ms10-detached-fault.log` identifies
a stack-probe page fault in `execute_pool_metadata`, followed by a triple fault.
Checkpoint `8b3c995` separates lightweight dispatch (approximately 97 KiB compiled
startup call frames) from active transactions and reserves a fixed 1 MiB kernel
stack on both architectures. Active transaction frames exceeded the prior
256 KiB allocation. Release fabric still passes 128 tests; corrected installed
boot and active-operation proof are required, not inferred from compilation.

Integrated source checkpoint `0aad862` includes current metadata reads, repair
overlay recovery, shared Settings projection and namespace-independent deletion
bookkeeping. `/tmp/ms10-integrated-fabric-after-outbox.log` passed 128 tests,
with one existing captured-disk replay test ignored. The new stale-owner test
found and fixed late deletion-outbox allocation at the 32-entry namespace bound;
update/copy/delete now pass with repaired metadata and cold retry on the host.

Transport checkpoint `840b1eb` permits 64 authenticated DATA attempts per link
per second while retaining one RX/TX per poll, fairness and separate control
limits. Its host fixture delivered 640 ordered 64-byte payloads in ten virtual
seconds (40,960 bytes); this is not installed storage throughput. MS9's 39-test
gate passed. Final-source ISO build and installed lifecycle remain pending.

The restored authenticated DATA budget is bounded separately from control
traffic. MS9's 39-test gate passed after adding full-transmit-queue confirmation
retry and four-link fairness coverage. The host cadence fixture delivered 160
ordered authenticated payloads in ten simulated seconds; this is not an installed
throughput measurement.

Signed repair authority now binds a fixed metadata group, owner generation,
writer, approved destinations and expiry. A replacement must sign its exact
verified copy before either surviving metadata member can acknowledge staging.
Fresh repair reads require distinct R2 observations and W2 write-back; absence
must be observed, not inferred from timeout. Historical committed repairs remain
readable after admission expiry without renewing mutation authority.

The current combined fabric run (`/tmp/ms10-closure-fabric-full.log`) passed
120 tests with the existing one ignored captured-disk replay test. The retained
drag benchmark (`/tmp/ms10-closure-performance-current.log`) observed 300 host
frames, average 30,391 ns, p95 32,625 ns, worst 51,000 ns. These results predate
the final automatic-repair and recovery integration and must be rerun after it.

Native linked-child metadata removes repair namespace exhaustion. Shared content
mutation uses durable admission and result records rather than acknowledging a
local change as distributed success. Sector-cut/retry tests are mechanism proof;
the defining owner-offline, replacement, stale-return and reboot VM sequence is
still required. No MS10 completion or MS11-unblocked claim is made.

## Previous handoff (historical, before current closure changes)

- Latest complete `./build.sh` passed: `/tmp/ms10-pool-refresh-build-2.log`.
  Packaged runtime includes the targeted Settings cache fix and metadata
  primitives/backing; it does not include the experimental faster data pacing.
  ARM64 SHA-256: `31de98da76c7a47a68dd18a051a108d6744139122c4ffccb5fdafe8b2357728e`.
  x86_64 SHA-256: `2b92a4795fea3fd78342fca10bcc102fa37f6feec55eec481095d27d7604f237`.
- The already-running fresh-install UI recheck completed after stopping correction
  loops: `/tmp/ms10-pool-ui-fresh-9246a0a` passed installation, ISO-detached boot,
  onboarding, cold authentication and native storage inspection. Manual screenshot
  review confirms the Settings panel now leaves Loading and presents the observed
  one-node, empty-object state (`/tmp/ms10-pool-ui-fresh-presented.png`). This is
  not populated-object policy parity, drag-under-transfer or performance proof.
  All owned QA VMs are powered off; their disks and receipts remain preserved.
- Four `52c15e8` installed disks remain preserved. All passed detached cold boot,
  persistent unique identity, authentication and exact mutual peer discovery.
  A/B and A/C protected trust is now persisted; D is not paired. The final stopped
  measurement had no active secure sessions, grants or distributed Pool objects.
- A focused cold-start A/B ceremony passed after an earlier warm ceremony failed.
  The original intermittent failure is NOT FIXED. Discovery error counters also
  appeared during the passing run, so they do not establish its root cause.
- The last resumed run failed before pairing D because the launcher helper did not
  return from Settings to Desktop before issuing the launcher shortcut. The native
  route was inspected, but no further correction/retry was made after the limit.
  Incomplete resume/measurement harness edits were removed from active source and
  preserved as `MS10_PENDING_DISTRIBUTION_HARNESS.patch` for the next focused pass.
- No installed 1 KiB or >16 KiB distributed transfer, three-copy hash verification,
  remote read, loss/heal, stale return or distributed reclamation pass was reached.
- The unchanged transport's actual-cadence host fixture delivered ten authenticated
  64-byte payloads in ten simulated seconds. This is a protocol fixture measurement,
  not installed throughput. A targeted faster-pacing experiment passed one cadence
  test but lacks adversarial/fairness/backpressure and installed performance proof.
  Its runtime edits were removed; the exact proposal is retained in
  `MS10_PENDING_AUTHENTICATED_PACING.patch` and its companion document.
- Owner-offline normal reads still require wire/namespace/application integration
  of the signed metadata foundation. Physical replica ownership was not weakened
  to make an acceptance test pass.

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
  (`/tmp/ms10-lifecycle-final-build.log`). Four independent fresh installs passed
  detached cold boot, persistent unique identity, authentication and native storage
  inspection (`/tmp/infinity-ms10-distributed-installed-52c15e8/result.json`).
  ARM64 ISO SHA-256: `c14d1ea149e4cca997daba414638164c6b278829c5c9877ee29daf2a5dad104f`.
  x86_64 ISO SHA-256: `e8bbed01cb6d7ce2b8ac5fcf7fe3657e9aac1f954c50a5ce720b0bbda73339b7`.
- Installed screen review on `52c15e8` found a failing refresh boundary: typed Pool
  projection was ready with one node, but Storage Settings remained in its loading
  state after waiting and keyboard interaction. The subtitle overlap is fixed;
  the populated-state GUI refresh check FAILED and requires a targeted fix.
- The initial host-timed long-command producer lost six of 89 characters before
  mesh setup. Explicit make/break input with binary length acknowledgement passed
  89/89 without retransmission; bounded four-key batches also passed. These prove
  the acceptance producer, not installed UI responsiveness or the root cause of
  the original loss. Security leases were not extended.
- The final-source four-node distribution preflight passed mutual discovery but
  failed its first A/B pairing: both local confirmations completed with more than
  70 seconds remaining, but remote confirmation/trust did not commit. No Pool
  object was created. A matching actual-cadence four-node host transport fixture
  passed, so no scheduler or security-timeout change was justified from that
  hypothesis. Installed packet-metadata diagnosis is separate and still pending.
- `9246a0a` fixes Pool-revision invalidation for retained Settings slot 4 only.
  The pixel-cache behavioral test, kernel compile check and fresh-installed
  empty-Pool screenshot recheck pass. No full-screen redraw was introduced.
- `a605b3c` adds isolated signed R2/W2 metadata primitives and native durable
  backing, not owner-offline application reads. Integrated suites passed:
  105 fabric tests (one ignored) and 62 performance-harness tests. These suites
  share common runtime tests; their counts are not independent performance claims.

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
