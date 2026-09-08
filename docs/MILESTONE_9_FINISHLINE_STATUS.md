# Milestone 9 finish-line status

**Overall: PARTIAL. Do not label this release Milestone 9 COMPLETE.**

## Compact acceptance checklist

1. Carry signed discovery and explicit pairing over the existing native UDP path.
2. Negotiate mutually authenticated duplex sessions with independent local handles.
3. Carry remote operations through secure transport and execution-time IOP checks.
4. Route both human interfaces through IOP; provide selected-resource detail.
5. Publish post-commit IEF events and reconcile sequence gaps through authoritative IOP.
6. Verify independent blank installs, detached media, lifecycle/security failures,
   GUI/Console lockstep, and responsiveness under active node networking.

NIC development and Milestone 10 remain out of scope.

## Verification for this increment — 2026-09-07

- `make milestone-9-test`: exit 0, including the new duplex and fresh reconnect regressions.
- `./build.sh`: exit 0; both architecture builds, installed-kernel/asset parity,
  UI/input, network-wire, security, performance and resource-policy gates passed.
- `make native-nic-test`: exit 0; paired engineering guests returned `[33, 33]`,
  the absent-peer case returned `[35]`. The NIC implementation was not changed.
- Host retained-drag measurement: 300 frames, average 33,203 ns, p95 44,541 ns,
  worst 95,208 ns. This is not an installed UI timing under active node traffic.
- x86_64 ISO SHA-256:
  `2a937b3d9fbbd2d52ed10c986dadfb41a45fa486045f84de10b19ab79742b89e`
- ARM64 ISO SHA-256:
  `cd4598b661dd4e2ca3f3ae8eabde439e4f670040e234da131568de47e9108ec7`

The common kernel source builds into both installer and installed payloads.
The session mechanism is not yet reachable through a completed native node
handshake, so artifact parity does not establish installed feature acceptance.

## This increment

The session mechanism previously used one bidirectional key and included the
runtime-local session handle in its nonce. It now derives separate low-to-high
and high-to-low keys through the existing X25519/HKDF-SHA256 Crypto service.
The derivation binds ordered NodeIds and the supplied transcript, rejects
non-contributory agreement, and derives a shared protocol reference independently
of local handles. Direction-local sequence numbers form the nonce. Reusing an
existing session reference, even after closure, is rejected. Closure, trust loss
and expiry cleanup zeroize both directional keys. Session keys are no longer
public fields or exposed by derived Debug formatting.

`tools/milestone9-harness/duplex_acceptance.rs` checks bidirectional traffic,
independent handles/sequences, replay in each direction, reflected ciphertext,
invalid tags, unknown/expired handles, fresh keys/transcripts after closure and
old-frame rejection. This is a **HOST mechanism regression**, not authenticated
network negotiation. Its inherited paired-node fixture is simulated authority;
it does not count as independently displayed or human-confirmed verification.

`open_session` still requires an already trusted peer and caller-provided
agreement/transcript material. The missing authenticated wire handshake must
provide fresh entropy, verify both signed identities and negotiate protocol
before calling it. The new KDF does not itself authenticate that transcript.

## Exact remaining implementation blockers

- No production Node Transport Adapter routes discovery, pairing, handshake
  and encrypted IOP requests through the native connected-datagram service.
- Console `execute_node_mutation` still calls `execute_node_operation` directly;
  GUI node actions also have direct mutations. Admission to a typed schema is
  not IOP router execution.
- Selected-resource IOP responses still contain aggregate/partial detail;
  domain operations do not yet establish synchronized two-node membership.
- Node subscribers do not yet demonstrate automatic sequence-gap detection,
  stale projection handling and authoritative IOP reconstruction.
- The required two independently installed, media-detached nodes have not
  exercised the 55-step native lifecycle, interactive parity and performance flow.

These are missing implementation and acceptance evidence, not merely missing
documentation. Engineering packet tests cannot close them.

## Required status report

“TESTED / HOST” below means a passing regression for the isolated mechanism.
It does **not** satisfy a mandatory native/installed acceptance item.

| Area | Status | Evidence boundary |
| --- | --- | --- |
| Reference NIC Path | TESTED | Native QEMU engineering fixture |
| Native A↔B Packet Exchange | TESTED | Native QEMU engineering fixture, not installed |
| Signed Wire Discovery | SCAFFOLDED | No production wire adapter |
| Node Inventory | PARTIAL | No native discovered-peer UI acceptance |
| Pairing Begin | TESTED | HOST |
| Independent Verification | PARTIAL | Two independently displayed systems unverified |
| Pairing Confirm | TESTED | HOST |
| Authenticated Handshake | SCAFFOLDED | No negotiated native handshake |
| Duplex Session | TESTED | HOST mechanism only |
| Independent Session Handles | TESTED | HOST |
| Directional Keys | TESTED | HOST |
| A→B Replay Protection | TESTED | HOST |
| B→A Replay Protection | TESTED | HOST |
| Reconnect / Fresh Session | PARTIAL | Fresh mechanism tested; native negotiation absent |
| Old-Session Rejection | TESTED | HOST |
| Remote IOP | SCAFFOLDED | No secure node carriage |
| Execution-Time Authority Validation | TESTED | HOST local IOP race |
| GUI IOP Path | PARTIAL | Direct mutation paths remain |
| Console IOP Path | PARTIAL | Direct executor bypass remains |
| GUI/CLI Lockstep | PARTIAL | Installed cross-interface matrix unverified |
| Node Inspection Detail | PARTIAL | Partial response fields |
| Session Inspection Detail | PARTIAL | Aggregate response remains |
| Domain Inspection Detail | PARTIAL | Aggregate response remains |
| Per-Node Policy | TESTED | HOST service mutation |
| Trust Restriction | TESTED | HOST |
| Trust Revocation | TESTED | HOST |
| Grant Revocation | TESTED | HOST |
| Mesh Join | PARTIAL | HOST local state only; no peer synchronization |
| Mesh Leave | PARTIAL | HOST local state only; no peer synchronization |
| IEF Publication | PARTIAL | Not complete across the native lifecycle |
| IEF Sequence Tracking | SCAFFOLDED | Node projection gap handling unverified |
| IEF Reconciliation | SCAFFOLDED | Authoritative node-query reconstruction absent |
| Offline / Recovery | PARTIAL | No native node reconnect acceptance |
| Persistence | TESTED | HOST binary round-trip/corruption gates |
| Audit | TESTED | HOST state records |
| Service Restart | PARTIAL | No complete native restart matrix |
| Two-Node Blank Install | IMPLEMENTED BUT UNTESTED | Required independent pair not exercised |
| Detached-Media Test | IMPLEMENTED BUT UNTESTED | Required node flow not exercised |
| Interactive GUI Test | IMPLEMENTED BUT UNTESTED | Required lifecycle not exercised |
| Interactive Console Test | IMPLEMENTED BUT UNTESTED | Required lifecycle not exercised |
| UI Performance Regression | PARTIAL | Host gates; active native-node UI load unverified |
| Resource-Policy Regression | TESTED | Existing host gates |
| Documentation | PARTIAL | Status and mechanism documented; no completion evidence |
