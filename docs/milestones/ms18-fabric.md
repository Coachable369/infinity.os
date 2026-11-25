# MS18 implementation inventory and acceptance ledger

Status: **in progress; not a completed milestone or release**.

## Existing implementation audit

| Area | Existing implementation | Remaining requirement |
|---|---|---|
| Discovery and trust | `kernel/runtime/node/` provides discovery, durable identity, membership, grants and authenticated transport | Verify two independent installed nodes and hardware metadata updates |
| Resource inventory | `runtime/fabric/resources.rs` owns bounded authoritative inventory, capacity reservations, advertisement expiry and disconnect handling | Real hardware descriptors and enumeration for each supported category |
| Distributed storage | `runtime/fabric/` plus `storage/fabric*.rs` implement placement, metadata, replicas, repair and deletion | Integrate management controls with existing storage authority |
| Compute | `runtime/compute.rs` and IOP dispatch provide bounded checksum/counter workloads and execution-context accounting | Actual Python/native execution, streaming output, compatibility, enforced runtime isolation |
| Application policy | `runtime/resource_policy.rs` provides revisions, persistence, local limits and leases | Connect provider policy to durable shared management API |
| Devices | Native camera, audio, USB-related and network adapters exist | Authorized per-device provider operations, removal, exclusivity and lease enforcement |
| GPU and Bluetooth | No usable generic fabric compute/Bluetooth provider established by this audit | Driver/provider implementation and hardware validation; do not advertise fixture execution as GPU support |
| Management | Existing Pool console and typed native operations | Fabric Manager, complete pool CLI/help, AIKit integration, shared persisted state |
| SDK | Existing SDK kits and application resources | Operational Fabric documentation and examples after APIs are implemented |

## Implemented foundation changes

- Preserve legacy v1 storage/compute/memory/accelerator advertisements; add v2 device-category encoding. New categories do not imply available drivers or remotely usable devices.
- Reject advertisements that would turn offline resources into online ones during encoding, and reject unknown capability bits.
- Make boot-local compute identity retirement atomic when any resource remains reserved.
- Bind provider execution contexts to the authenticated requesting node. Another node cannot resume or cancel an active colliding task identifier.
- Enforce provider aggregate memory, scheduler-weight and concurrent-context ceilings. CPU units are scheduler weights, **not physical cores or a demonstrated CPU percentage quota**.
- Require a current ResourcePolicyModify capability and expected revision to alter provider limits. Disabling contributions reclaims live contexts.
- Reclaim remote contexts on consumer disconnect or expiry of a provider-owned 60-time-unit inactivity lease, even if a consumer supplied no deadline.

The provider policy is currently a runtime API, not reboot-persistent configuration. Existing remote execution is still bounded built-in work, not arbitrary Python. A stale continuation after lease expiry still needs explicit replay-fencing coverage before general workload support. No desktop audio pipeline implementation was changed.

## Verification scope

Host behavioral harness only. These checks exercise typed APIs and actual state changes. They are **not** two-VM, installed-system, physical-device or UI acceptance.

Command: `./build-kit run env RUST_MIN_STACK=16777216 CARGO_TARGET_DIR=build/behavior-harness cargo test --quiet --release --manifest-path tools/behavior-harness/Cargo.toml --bin fabric-test`

Previous foundation pass: 242 passed, 0 failed, 1 ignored; manifest `builds/manifests/20261125T083054309876Z-49394.json`. Lease/disconnect extension: 243 passed, 0 failed, 1 ignored in 51.29 seconds; manifest `builds/manifests/20261125T083432426531Z-49495.json`.

## Capacity-management checkpoint

The shared read-only management API now authenticates an active owned session and
projects capacity from the existing directory. Internal local/provider breakdowns
are diagnostics, not an application placement requirement. The intended public
allocation contract remains one unified pool with automatic placement.

Behavioral coverage checks reservations, provider deduplication, expired/offline/
unshared resources, integer overflow, and rejected foreign or locked sessions.
Result: 246 passed, 0 failed, 1 ignored; manifest
`builds/manifests/20261125T084547148833Z-51615.json`.
This checkpoint adds no hardware providers or remotely addressable RAM and is not
end-to-end resource pooling acceptance.

## Milestone acceptance matrix

All entries remain NOT TESTED at the required end-to-end acceptance level. Unit-level evidence does not promote a broader criterion to PASS.

| ID | Criterion | Status | Outstanding verification |
|---|---|---|---|
| FAB-01 | Multiple InfinityOS nodes discover one another | NOT TESTED | Native integrated implementation and required VM/device evidence pending |
| FAB-02 | Authorized hardware resources are advertised | NOT TESTED | Native integrated implementation and required VM/device evidence pending |
| FAB-03 | Resource metadata reflects actual hardware | NOT TESTED | Native integrated implementation and required VM/device evidence pending |
| FAB-04 | Resource updates propagate across the fabric | NOT TESTED | Native integrated implementation and required VM/device evidence pending |
| FAB-05 | Disconnected resources become unavailable | NOT TESTED | Native integrated implementation and required VM/device evidence pending |
| CMP-01 | A Python workload executes on another node | NOT TESTED | Native integrated implementation and required VM/device evidence pending |
| CMP-02 | Remote output and exit status are returned | NOT TESTED | Native integrated implementation and required VM/device evidence pending |
| CMP-03 | Automatic placement selects a compatible node | NOT TESTED | Native integrated implementation and required VM/device evidence pending |
| CMP-04 | CPU contribution limits are enforced | NOT TESTED | Native integrated implementation and required VM/device evidence pending |
| CMP-05 | Remote workloads can be cancelled | NOT TESTED | Native integrated implementation and required VM/device evidence pending |
| CMP-06 | Incompatible workloads are rejected safely | NOT TESTED | Native integrated implementation and required VM/device evidence pending |
| CMP-07 | Workload failures release allocated resources | NOT TESTED | Native integrated implementation and required VM/device evidence pending |
| RES-01 | Shared memory capacity is discoverable | NOT TESTED | Native integrated implementation and required VM/device evidence pending |
| RES-02 | Remote workloads reserve provider memory | NOT TESTED | Native integrated implementation and required VM/device evidence pending |
| RES-03 | Memory allocation limits are enforced | NOT TESTED | Native integrated implementation and required VM/device evidence pending |
| RES-04 | GPU capabilities are accurately advertised | NOT TESTED | Native integrated implementation and required VM/device evidence pending |
| RES-05 | A supported GPU workload executes remotely | NOT TESTED | Native integrated implementation and required VM/device evidence pending |
| RES-06 | Unsupported GPU operations are reported accurately | NOT TESTED | Native integrated implementation and required VM/device evidence pending |
| DEV-01 | Authorized USB devices are discoverable | NOT TESTED | Native integrated implementation and required VM/device evidence pending |
| DEV-02 | A supported USB device operation executes remotely | NOT TESTED | Native integrated implementation and required VM/device evidence pending |
| DEV-03 | Authorized Bluetooth devices are discoverable | NOT TESTED | Native integrated implementation and required VM/device evidence pending |
| DEV-04 | A supported Bluetooth operation executes remotely | NOT TESTED | Native integrated implementation and required VM/device evidence pending |
| DEV-05 | Authorized camera streaming works | NOT TESTED | Native integrated implementation and required VM/device evidence pending |
| DEV-06 | Authorized remote audio playback works | NOT TESTED | Native integrated implementation and required VM/device evidence pending |
| DEV-07 | Device removal invalidates affected access | NOT TESTED | Native integrated implementation and required VM/device evidence pending |
| DEV-08 | Exclusive device ownership is enforced where required | NOT TESTED | Native integrated implementation and required VM/device evidence pending |
| MGT-01 | Fabric Manager launches successfully | NOT TESTED | Native integrated implementation and required VM/device evidence pending |
| MGT-02 | The topology displays actual discovered nodes | NOT TESTED | Native integrated implementation and required VM/device evidence pending |
| MGT-03 | Resource controls modify actual provider configuration | NOT TESTED | Native integrated implementation and required VM/device evidence pending |
| MGT-04 | CLI management commands are functional | NOT TESTED | Native integrated implementation and required VM/device evidence pending |
| MGT-05 | CLI changes appear in the GUI | NOT TESTED | Native integrated implementation and required VM/device evidence pending |
| MGT-06 | GUI changes appear in CLI queries | NOT TESTED | Native integrated implementation and required VM/device evidence pending |
| MGT-07 | AIKit executes authorized fabric operations | NOT TESTED | Native integrated implementation and required VM/device evidence pending |
| MGT-08 | All interfaces use the same management API | NOT TESTED | Native integrated implementation and required VM/device evidence pending |
| MGT-09 | Configuration persists across reboot | NOT TESTED | Native integrated implementation and required VM/device evidence pending |
| MGT-10 | Unauthorized controls and operations are rejected | NOT TESTED | Native integrated implementation and required VM/device evidence pending |
| SEC-01 | Unauthorized nodes cannot consume protected resources | NOT TESTED | Native integrated implementation and required VM/device evidence pending |
| SEC-02 | Unauthorized identities cannot modify sharing policies | NOT TESTED | Native integrated implementation and required VM/device evidence pending |
| SEC-03 | Resource leases expire and release allocations | NOT TESTED | Native integrated implementation and required VM/device evidence pending |
| SEC-04 | Revoked permissions prevent further resource access | NOT TESTED | Native integrated implementation and required VM/device evidence pending |
| SEC-05 | Local resource reservations are enforced | NOT TESTED | Native integrated implementation and required VM/device evidence pending |
| SEC-06 | Node failures do not crash the remaining fabric | NOT TESTED | Native integrated implementation and required VM/device evidence pending |
| SEC-07 | Sensitive peripherals require explicit authorization | NOT TESTED | Native integrated implementation and required VM/device evidence pending |
| SEC-08 | Remote workloads execute within supported isolation boundaries | NOT TESTED | Native integrated implementation and required VM/device evidence pending |
