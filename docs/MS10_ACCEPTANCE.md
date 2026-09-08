# Milestone 10 execution checklist

Status: IN PROGRESS, not complete. Installer click/progress prerequisite passed; see INSTALLER_CLICK_PROGRESS_ACCEPTANCE.md.

Authoritative request: user-supplied Resource Fabric & Infinity Pool Storage milestone.

1. Authenticated, versioned resource advertisements with explicit owner, capacity, health and node/device failure domain.
2. Stable ObjectId independent of namespace, authority and placement; preserve independent copy identity and implement shared immutable-content COW. Existing local copy creates a new identity but currently copies the content bytes.
3. Explicit Temporary/Protected/Critical policy, independently placed verified replicas, capacity reservations and bounded transfer state.
4. Coherent durable manifest/version commits; incomplete or corrupt replicas never become available.
5. Integrity-checked reads survive a lost storage node; preserve known offline object identity and namespace.
6. Single authoritative bounded healing claims, resumable transfer, verified completion and safe returning-node reconciliation.
7. Native capability-governed IOP/IEF integration, common Console/GUI inspection, System Generation packaging.
8. Behavioral local/placement/integrity/failure/healing/reconciliation tests.
9. At least three independently installed nodes: write/protect/distribute, remove replica host, read/degraded/heal/healthy, return stale node, cold reboot and verify persistence.
10. Preserve MS9/security, desktop performance and installer regressions. Do not substitute host fixtures for installed acceptance.

Dependency still open: the last installed MS9 run lost its secure session before remote inspection admission. Rapid installed input also accepted six of ten keys. Neither is marked resolved by this milestone or by the installer fixes.

## Current implementation and evidence

TESTED (host behavioral harness, not installed multi-node acceptance):

- `kernel/runtime/fabric/resources.rs`: bounded authenticated resource admission, exact ownership/scope, revocation/expiry, ordered generations, capacity reservations and flood limits.
- `kernel/runtime/fabric/placement.rs`: deterministic eligible storage selection on independent nodes; Temporary/Protected/Critical require 1/2/3 verified independent replicas respectively.
- `kernel/runtime/fabric/replica.rs`: bounded contiguous transfer, duplicate/conflict handling, resumable checkpoints, SHA-256 read-back, publication failure retry and generation fencing.
- `kernel/storage/fabric.rs`: internal native object-store adapter commits checkpoint and staged bytes in one COW transaction. Recovery uses persisted bytes, not a process cache. Available readers verify SHA-256. Truncated/invalid records and corruption are rejected.
- `make fabric-test`: 39 tests passed (9 new fabric/persistence tests plus 30 imported runtime tests). Native persistence tests cut every sector-write boundary of a chunk commit and of Available publication, remount the actual object store, and recover coherently. This models synchronous sectors, not torn-sector or real device cache failure.
- The fabric behavioral gate is included in `build.sh`.

IMPLEMENTED BUT UNTESTED on installed hardware: the above algorithms and native persistence adapter. The adapter is internal, not yet exposed through an authenticated IOP service. Its initial envelope supports nonempty payloads up to 16,256 bytes; multi-extent/empty-object support remains required. It does not replace application ObjectId with its internal backing-object identity.

NOT YET IMPLEMENTED: authoritative durable manifests/coordinated healing, transparent remote reads, native transfer operations and per-chunk authority enforcement, complete resource producers/discovery wiring, shared immutable-copy storage, common Console/GUI surfaces, dedicated System Generation service registration and three-installed-node failure/healing/reboot acceptance. No claim of protected distributed storage availability or MS10 completion is made.

## Latest build

`./build.sh` exited 0 on source `8868564`; log `/tmp/ms10-foundation-build.log`. Both installer and installed kernels compiled, the new fabric gate and existing build gates passed, and fresh ISOs were assembled:

- ARM64 SHA-256: `271ba6f3491557a781e0a5f0db17534d8a4b255d1ae7cde39018b21f8cbd06dc`
- x86_64 SHA-256: `525f7dd82bd7c9722a82a5ede60e5b96b2d9440874da806133a2f19f10f020af`

Installer interaction evidence remains the two pinned earlier images in INSTALLER_CLICK_PROGRESS_ACCEPTANCE.md; installer interaction source was unchanged by these fabric additions. These latest images have not passed MS10 installed acceptance. Internal fabric code compiling into the source tree is not equivalent to a registered, running System Generation fabric service.

Commits are local on mainline; no Git remote is configured, so push/pull could not be performed.
