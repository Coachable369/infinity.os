# Milestone 9-B control-plane increment

**Milestone 9-B: PARTIAL. Overall Milestone 9: PARTIAL.**

Historical increment report: the subsequent [final-completion pass](MILESTONE_9_FINAL_COMPLETION_STATUS.md)
supersedes the installed-write-disabled and abbreviated Policy.Read descriptions
below. It does not supersede the outstanding GUI/Console, reconciliation,
membership and installed-lifecycle blockers.
M9-A remains a separate regression-tested engineering wire foundation.
This increment does not meet the complete 9-B stop condition and does not begin
9-C or M10. No final installed-system acceptance is claimed.

## Implemented slice

`kernel/runtime/iop_remote.rs` extends the ordinary `IopRouter` with bounded
remote request admission, dispatch, correlation, typed completion and cleanup.
NodeTransport only authenticates and transports data; it never calls the service
executor. The normal router rechecks authority after dequeue, then calls the same
NodeOperationV1 service executor used by local IOP. A private protocol-prefix
demultiplexer preserves unrelated M9-A application data.

Local submission validates an exact ServiceCall capability for the caller and
operation. The peer router requires the same live session reference, current
Trusted state, an unexpired/unrevoked exact-operation remote grant, matching
scope and one required right, subject binding, and current per-node policy.
Authenticated identity is not remote authority. The supported reference operations
act only on the caller's own peer entry, not arbitrary third-party identities.
Policy category 0 gates remote metadata reads; category 1 gates delegated node
control. Policy scope must match the granted request scope. Deny is the default;
Leased decisions recheck local expiry at execution time.

Supported remote registry subset: Node.Inspect, TrustRead, TrustUpdate,
RevokeTrust, Block, Unblock, PolicyRead, PolicyUpdate, Health and Diagnostics.
Pairing consent, session-handle operations and unsynchronized mesh mutation are
not accepted by this remote dispatcher. Unsupported operations fail explicitly.
This allowlist must not be represented as the complete 9-B API.

## Fail-closed mutation readiness

**The installed runtime leaves remote mutation readiness false.** Even a valid
grant returns typed ServiceUnavailable for writes. The missing dependency is the
installed node service's durable commit + post-commit IEF lifecycle. It would be
incorrect to silently accept volatile writes and report full control-plane
success. Enabling readiness grants no authority; all grant/trust/policy checks
still run. Only the engineering fixture currently enables it for in-memory
service-execution tests. Its mutation proof is NOT installed persistence or IEF
publication proof. The existing Console/GUI direct paths are unchanged.

## Versioned remote wire frame

One 128-byte record is encrypted inside existing M9-A SessionData (192-byte limit).
No fragmentation, socket interface, stream emulation or cryptographic redesign.

| Offset | Encoding |
| --- | --- |
| 0..4 | IOP9 protocol discriminator |
| 4 | Version 1 |
| 5 | Request 1 / Response 2 |
| 6 | Explicit typed error discriminator, zero for success/request |
| 7 | Reserved zero |
| 8..16 | Request ID, little-endian u64 |
| 16..24 | CorrelationId |
| 24..32 | CausationId |
| 32..40 | Peer-issued authority reference |
| 40..44 | Remaining receiver lease, u32 seconds (1..30) |
| 44..48 | Reserved zero |
| 48..128 | Exact version-one NodeOperationV1 payload, including operation ID |

Source NodeId, target NodeId and protocol session reference are bound by the
outer M9-A authenticated envelope and supplied to the router only after successful
authentication. Runtime-local handles never appear in the remote record. Both
request and response remain protected by directional AEAD.
Responses preserve CorrelationId and set CausationId to the request ID, matching
the ordinary IOP response contract; the request's own causation travels to admission.

Clocks need not share uptime: the caller enforces its absolute local deadline;
the receiver enforces a bounded relative lease from admission. This is NOT a
globally synchronized execution deadline. A severely delayed datagram can reach
the receiver after the caller has stopped waiting; responses then fail correlation
freshness. There is no exactly-once guarantee or automatic mutation retry.

## Bounds, correlation and errors

- Eight outstanding requests; completed outcomes occupy a caller-owned bounded
  mailbox until consumed or 30 seconds beyond the original deadline.
- Eight incoming requests, eight queued replies, four active-session replay streams.
- Unique checked-increment request IDs; strict request high-water mark per
  peer/session prevents repeated execution. Old stream slots are reclaimed only
  when their authenticated session is no longer live.
- Responses match request ID, peer, protocol reference, operation, correlation and
  causation; unknown, duplicate and late responses cannot become successful calls.
- One receive, at most one response send and one request send per pump; one service
  request per dispatch. No waiting on peers, unbounded queues or per-packet IEF.
- Success, error, deadline and session-loss outcomes are typed. Local capability
  revocation is rechecked before sending. Peer capability/trust/policy are checked
  after dequeue. A full peer queue returns QueueFull when a bounded reply slot is
  available; otherwise the caller's deadline bounds failure.
- Malformed/oversized/version-invalid records are rejected without executing.
  Malformed frames that cannot safely correlate are dropped, not guessed into
  replies. No private keys or payload contents enter structured audit.

Error discriminants 1..23: MalformedRequest, UnsupportedSchemaVersion,
UnsupportedOperation, SessionNotFound, TransportClosed, TrustRequired,
TrustRevoked, NodeBlocked, CapabilityRequired, CapabilityExpired,
CapabilityRevoked, CapabilityScopeDenied, PolicyDenied, AccessDenied,
DeadlineExceeded, QueueFull, InvalidState, NotFound, Conflict, ReplayRejected,
UnknownResponse, RemoteFailure, ServiceUnavailable.

Audit 0xdb01 records the peer, correlation, timestamp and typed router outcome.
This audit record is not a substitute for authoritative version advancement or
post-commit IEF publication, which remain unresolved below.

## Evidence

HOST: `make milestone-9-test` adds actual Ethernet/UDP frame tests for duplex
remote reads, correlation/causation, one-time completion consumption, wrong grant
scope, unknown/expired/revoked grants, queued revocation before execution, policy
authorization changes, fail-closed mutation readiness, malformed/oversized/version
frames, unknown replies, queue saturation and bounded timeout without mutation.
These augment rather than replace the existing M9-A tests.

PRODUCTION WIRE / ENGINEERING GUEST: `make milestone-9-remote-iop-test` runs the
existing independently booted firmware-entropy E1000 guests and M9-A operator
verification. It then invokes both peer routers, verifies typed read results,
executes an explicitly authorized engineering in-memory policy mutation, denies
out-of-scope/revoked grants, revokes a queued mutation before dequeue and proves
no execution/state change, verifies policy denial/restoration and peer-loss timeout.
The host uses trusted binary operator commands to configure each guest locally;
it does not carry IOP protocol frames between guests or inject session material.
The result is written to `build/milestone-9b-remote-iop-proof.json`.

Final build/native regression evidence will be recorded below. Test output prose
is diagnostic only; assertions use bytes, typed values, counters, state and exit
statuses. No GUI screenshots or final installed behavior are claimed here.

## Exact remaining 9-B blockers

1. Durable node-service mutation commit, authoritative state sequence and real
   post-commit IEF must be integrated before installed remote writes are enabled.
2. Console `execute_node_mutation` still calls the service executor directly;
   Settings actions still mutate NodeRuntime directly. Both require scoped local
   bootstrap authority and shared router-backed workflows, not automatic fresh
   grants added merely to make a wrapper pass. GUI↔CLI lockstep is not verified.
3. Node inspection remains abbreviated; Session/Domain inspection is aggregate;
   Policy.Read does not return all category fields. Complete selected-resource
   schemas and bounded pagination are still needed.
4. Subscribers lack the required authoritative IOP reconstruction loop for dropped
   trust/policy/membership/offline events. Declared events and an audit counter do
   not satisfy sequence tracking, stale-state handling or gap recovery.
5. Mesh membership remains local-only. There is no versioned two-node domain
   membership transaction, idempotent join/leave synchronization or lost-ACK
   reconciliation. Remote Join/Leave are deliberately unsupported here.
6. The native fixture does not exercise GUI/Console projections, full inspection,
   synchronized membership, event suppression/reconstruction, all adversarial IOP
   cases or a session-close-mid-request test. Existing M9-A session-close proof
   does not substitute for the latter.

These are M9-B implementation/evidence gaps, not evidence of a M9-A or compositor
regression. Mandatory 9-B acceptance remains blocked. Do not mark 9-B TESTED.

## Packaging

The remote router is compiled into the existing core IOP/runtime component on
both live and installed kernel paths. Existing exact installed-kernel/loader
artifact parity covers its binary inclusion; no standalone engineering component
is installed. No network grant or endpoint is enabled by default. Engineering
mutation readiness is not installed configuration. Both architecture ISOs retain
the same safe default. No ARM64 native wire support is claimed.

## Independent status matrix

H = HOST. W = PRODUCTION WIRE / ENGINEERING GUEST. No INSTALLED SYSTEM claim.
TESTED rows below describe only the explicit implemented scope, not full 9-B.

| Item | Status | Evidence / remaining limit |
| --- | --- | --- |
| Remote IOP Wire Envelope | TESTED | H + W, bounded NodeOperationV1 subset |
| A→B Remote IOP | TESTED | W |
| B→A Remote IOP | TESTED | W |
| Request Correlation | PARTIAL | H + W basic correlation; full duplicate/late suite outstanding |
| Request Bounds | TESTED | H, saturation/length/deadline |
| Peer IOP Router Dispatch | TESTED | W |
| Execution-Time Authority Revalidation | TESTED | W, queued capability revocation |
| Remote Capability Allow | TESTED | W |
| Remote Capability Deny | TESTED | W |
| Remote Capability Revoke | TESTED | W, old grant remains denied; new grant explicit |
| Remote Policy Enforcement | TESTED | W, engineering in-memory policy mutation |
| Console IOP Path | PARTIAL | Direct node mutation remains |
| GUI IOP Path | PARTIAL | Direct node mutation remains |
| Direct Console Bypass Removed | PARTIAL | Not removed |
| Direct GUI Bypass Removed | PARTIAL | Not removed |
| GUI→CLI Parity | SCAFFOLDED | No native projection acceptance |
| CLI→GUI Parity | SCAFFOLDED | No native projection acceptance |
| Node.Inspect Detail | PARTIAL | Abbreviated fields |
| Session.Inspect Detail | PARTIAL | Aggregate response |
| Domain.Inspect Detail | SCAFFOLDED | No synchronized domain model |
| Policy.Read Detail | PARTIAL | Missing complete category readback |
| Post-Commit IEF Publication | PARTIAL | Remote mutation lifecycle not integrated |
| IEF Sequence Tracking | PARTIAL | No complete authoritative node stream |
| Gap Detection | PARTIAL | Existing fabric primitives, no projection flow |
| Stale Projection Handling | SCAFFOLDED | No frontend reconstruction loop |
| Authoritative IOP Reconstruction | SCAFFOLDED | Not implemented |
| Trust Event Reconciliation | SCAFFOLDED | Not verified |
| Policy Event Reconciliation | SCAFFOLDED | Not verified |
| Membership Event Reconciliation | SCAFFOLDED | Not verified |
| Offline/Recovery Event Reconciliation | SCAFFOLDED | Not verified |
| Synchronized Mesh Join | SCAFFOLDED | Local-only foundation |
| Synchronized Mesh Leave | SCAFFOLDED | Local-only foundation |
| Membership Partial-Failure Recovery | SCAFFOLDED | No lost-ACK convergence |
| Peer Loss During IOP | TESTED | W, bounded caller timeout |
| Session Loss During IOP | IMPLEMENTED BUT UNTESTED | Cleanup implemented, focused pending-request test missing |
| M9-A Regression | TESTED | H + W, full original flow included in remote target |
| UI Performance Regression | TESTED | H, final build's existing UI/input/compositor gates |
| Resource-Policy Regression | TESTED | H, final build's existing application-resource gates |
| Documentation | PARTIAL | Implemented slice and blockers documented, not a finished 9-B contract |

## Final build evidence — 2026-09-07

`make milestone-9-test` and `./build.sh` exited 0 after the final source changes,
including response CausationId alignment with normal IOP. Build gates include
network-wire, UI/input, retained-drag/compositor, application-resource policy,
both architecture ISOs and exact installed-kernel/loader binary parity.

HOST retained drag: 300 frames, average 35,154 ns, p95 60,458 ns, worst 96,083 ns.
These are host gate measurements, not installed active-network desktop timings.
No compositor/window movement or application resource-policy implementation was
changed by this increment.

SHA-256:

- `builds/InfinityOS-x86_64.iso`:
  `be95fefa2ca6c5cb9508baed01d08e1a7b95132e1e2ec360f89bfc14f52033ef`
- `builds/InfinityOS-aarch64.iso`:
  `0634cf485437d9096152516c2fde6da3cd1284df891cc46de08042936a8dd856`

The build includes the working tree's pre-existing user UI/template edits; this
increment preserves and does not commit those unrelated edits.

Final `make milestone-9-remote-iop-test` exited 0 after the finished build. It runs
the existing `milestone-9-wire-trust-test` target with the additional remote IOP
sequence enabled. All original 9-A discovery/consent/duplex/rejection/close/fresh
reconnect assertions and the remote IOP subset passed. `make native-nic-test`
also exited 0 with paired outcomes [33, 33] and absent-peer outcome [35].

Final independent native NodeIds:
`3dc0a3ba6521040e86cec9be884832ca57f046b94b0e8042af1031751d5f9e74`
and `1fabc96c85d5d1c6ebcce669508c679d748bbfb2c6a05cddc712d58596ee78c1`.
Local handles were 3 and 4; session references were
`a393bc049d2aa103b4ddf8c2b6bb7016`,
`49fb721f3729ada486860c40540b02c0`, and
`3aa44f32793e4a73d8ffecf9fd7c3cb9`.
The proof explicitly records `overall_9B: PARTIAL` and installed GUI acceptance
false. Native mutation evidence uses the explicitly enabled engineering in-memory
service; installed mutation readiness remains false.
