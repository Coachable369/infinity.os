# Milestone 9 final-completion pass — PARTIAL

This is **not** a Milestone 9 completion certificate. The mandatory two-installed-node
lifecycle has not passed. M10 was not started. Engineering guests and HOST tests
are not substitutes for installed-system evidence.

## Changes in this pass

- `node/control.rs` stages the bounded trust/policy/session-close subset using the
  shared service executor. A failed persistence callback cannot change live peers,
  sessions, grants, audit checkpoint or control checkpoint. Temporary identity and
  traffic-key copies are zeroized. Pairing and unsynchronized membership are excluded.
- `poll_node_transport` uses the durable router entry point, backed by the native
  node-state object writer on the installed kernel. No development-tree writer is
  used; non-kernel runtime builds return persistence failure. Engineering fixtures
  still explicitly use their separate in-memory execution path.
- The router reserves reply capacity, then revalidates authority, stages, persists,
  commits live state, and returns a committed-control notice. The runtime publishes
  the notice before the next poll can transmit its queued reply. Publication failure
  does not roll back committed state. Subscriber reconstruction remains a blocker.
- Successful control commits advance a checked local control-service checkpoint,
  persisted at bytes 104–111 of the existing checksummed node-state object. Old
  objects have zero in this reserved field. This is **not** a global IEF sequence,
  nor a checkpoint for legacy direct GUI/Console/discovery mutations.
- Post-commit control notices contain subject NodeId (32 bytes), checkpoint (8 LE),
  operation (4 LE). `Node.PolicyChanged` is `0x9e011`; other covered controls use
  their existing event types. Correlation is preserved; causation is the request ID.
- `PersistenceFailed` is appended to the remote error table as discriminator 24.
- Correctly correlated replies revalidate the caller capability before disclosing
  the result. Replies for a different selected NodeId are rejected.
- `Policy.Read` now returns every category, rather than a trust-state summary.

### Policy.Read response (80-byte NodeOperationV1)

| Field | Meaning |
|---|---|
| node_id | Exact selected peer |
| handle | Local durable control-service checkpoint |
| scope | Authoritative policy scope |
| lease_deadline | Policy expiry |
| value | Policy version |
| flags | Category count: 12 |
| rights | 12 two-bit decisions, least-significant category first |

Decision values are Deny=0, Allow=1, SessionOnly=2, Leased=3. This fits the existing
128-byte remote envelope; there is no truncation, larger packet, or stream API.

## Focused behavioral evidence

`make milestone-9-correlation-test` runs nine HOST tests at the production router
boundary. Test setup explicitly constructs local sessions; it is not a wire proof.
Coverage: wrong peer/session/operation/correlation/causation/subject, caller revocation
at response delivery, duplicate request/response, deadline, queue capacity, mailbox
cleanup, checked ID exhaustion, outstanding-session loss, fresh-reference isolation,
old-correlation rejection, exactly-once execution of a duplicate policy request,
durable-write failure, restart decoding, dequeue-time grant revocation before storage,
and reconstruction of all committed policy fields.

`make milestone-9-durable-mutation-test` selects the three durable HOST tests.
`make milestone-9-test` now runs these unit tests before the existing behavioral harness.
Neither target uses source text, rendered text or log messages as an oracle.

## Build evidence (2026-09-08)

- `./build.sh`: exit 0 from the final source state. Includes both UEFI builds,
  extracted installed-kernel byte parity, installer/template/component gates,
  the nine focused tests plus existing M9 harness, network-wire, retained graphics,
  application resource policy and input/launcher behavior gates.
- `make milestone-9-durable-mutation-test`: exit 0, three HOST tests passed.
- `make milestone-9-correlation-test`: exit 0, all nine HOST tests passed.
- Retained-drag HOST fixture: 300 frames; average 35,513 ns; p95 55,416 ns;
  worst 218,917 ns; 23,159,280 pixels. This is not installed networking performance.
- Final build log: `/tmp/infinity-final-build-verified.log`.
- Durable-test log: `/tmp/infinity-final-durable.log`.
- `make milestone-9-remote-iop-test`: exit 0. PRODUCTION WIRE / ENGINEERING GUEST:
  independent NodeIds, independent handles 3/4, explicit dual confirmation,
  authenticated duplex, replay/reflection/tamper rejection, fresh reconnect,
  bidirectional remote IOP, queued grant revocation, policy enforcement and deadline.
  This continues to use the explicit engineering mutation adapter, not the new
  installed durable writer. Peer return is stop/continue, **not cold reboot**.
- `make native-nic-test`: exit 0; paired guest exit codes 33/33, absent NIC 35.
  Boundary: PRODUCTION WIRE / ENGINEERING GUEST, not installed OS acceptance.
- Wire proof: `build/milestone-9b-remote-iop-proof.json`; logs:
  `/tmp/infinity-final-wire.log`, `/tmp/infinity-final-nic.log`.
  Proof SHA-256: `33eceecbade695e142dd0aef9d42beb37b4d2f3a8f00323d4a62467473a01a9f`.

Release SHA-256:

| ISO in `builds/` | SHA-256 |
|---|---|
| InfinityOS-aarch64.iso | `9bb7f64ba7af7c05e02964a0fec9818ed94720d393fdf5db4ac23b825b3d83a0` |
| InfinityOS-x86_64.iso | `1427c3f223671be3c86cfe8a7ea91bb621a54004de0d45abe19fa0dc53b42d35` |

## Remaining completion blockers

1. Console still calls the service executor directly; GUI still mutates NodeRuntime.
   These paths do not use the new durable control transaction and must be replaced,
   not described as parity-complete.
2. Node/Session/Domain inspection is incomplete; general bounded pagination is absent.
3. Legacy state changes are not all checkpointed/committed before IEF. Subscribers
   do not reconstruct through authoritative IOP on a deliberately dropped event.
4. Membership is still local-only: no versioned two-node join/leave, idempotent
   membership journal, lost-ACK reconciliation or domain convergence acceptance.
5. Wire pairing transcript recovery, audit persistence and installed reboot lifecycle
   are not demonstrated by the control-state decoding tests.
6. No two independently installed systems have passed the detached-media lifecycle,
   GUI/Console lockstep, or active-network desktop performance acceptance.

## Required status matrix

TESTED entries identify a mechanism evidence boundary, not completion of the
mandatory installed scenario. “Not run” is never a successful acceptance result.

| Required item | Status | Boundary / remaining gap |
|---|---|---|
| Durable Mutation Commit | PARTIAL | HOST subset tested; all persistent mutations not unified |
| Installed Mutation Readiness | IMPLEMENTED BUT UNTESTED | Native writer wired; installed remote transaction not exercised |
| Authoritative State Versioning | PARTIAL | HOST control checkpoint; legacy streams not covered |
| Post-Commit IEF | IMPLEMENTED BUT UNTESTED | Runtime control notice publication; installed evidence absent |
| IEF Publication Failure Recovery | PARTIAL | Committed state retained; subscriber recovery absent |
| Console IOP Path | PARTIAL | Direct executor remains |
| GUI IOP Path | PARTIAL | Direct mutations remain |
| Direct Console Bypass Removed | SCAFFOLDED | Required replacement not implemented |
| Direct GUI Bypass Removed | SCAFFOLDED | Required replacement not implemented |
| GUI→CLI Lockstep | SCAFFOLDED | Required cross-interface acceptance not run |
| CLI→GUI Lockstep | SCAFFOLDED | Required cross-interface acceptance not run |
| Node.Inspect | PARTIAL | Abbreviated selected-node response |
| Session.Inspect | PARTIAL | Aggregate response remains |
| Domain.Inspect | SCAFFOLDED | Selected-domain detail absent |
| Policy.Read | TESTED | HOST: all committed fields reconstructed through router |
| Pagination | SCAFFOLDED | General inspection pagination absent |
| IEF Sequence Tracking | PARTIAL | Generic fabric sequencing; node reconstruction not complete |
| Gap Detection | PARTIAL | Required node subscriber tests not run |
| Stale Projection Handling | SCAFFOLDED | Required subscriber workflow absent |
| Authoritative IOP Reconstruction | SCAFFOLDED | Required node workflow absent |
| Trust Event Reconciliation | SCAFFOLDED | Dropped-event acceptance not run |
| Policy Event Reconciliation | SCAFFOLDED | Dropped-event acceptance not run |
| Membership Event Reconciliation | SCAFFOLDED | Dropped-event acceptance not run |
| Offline/Recovery Event Reconciliation | SCAFFOLDED | Dropped-event acceptance not run |
| Synchronized Join | SCAFFOLDED | Membership remains local-only |
| Synchronized Leave | SCAFFOLDED | Membership remains local-only |
| Lost-ACK Recovery | SCAFFOLDED | Idempotent membership protocol absent |
| Membership Versioning | SCAFFOLDED | Control checkpoint is not a domain transaction version |
| Remote Capability Allow | TESTED | HOST + PRODUCTION WIRE / ENGINEERING GUEST |
| Remote Capability Deny | TESTED | HOST + PRODUCTION WIRE / ENGINEERING GUEST |
| Remote Capability Revoke | TESTED | HOST + PRODUCTION WIRE / ENGINEERING GUEST; result-delivery race is HOST |
| Trust Restriction | PARTIAL | HOST mechanisms; installed contraction scenario not run |
| Trust Revocation | PARTIAL | HOST mechanisms; installed lifecycle not run |
| Session Loss During IOP | TESTED | HOST: cleanup, reconnect isolation and old response rejection |
| Duplicate Request Protection | TESTED | HOST: mutation executes once |
| Duplicate Response Protection | TESTED | HOST: duplicate cannot re-complete a mailbox |
| Late Response Rejection | TESTED | HOST: deadline, closed session and consumed correlation |
| Persistence | PARTIAL | HOST control-state reconstruction; installed reboot not run |
| Audit | PARTIAL | Bounded audit exists; durable full audit history incomplete |
| Two Independent Blank Installs | IMPLEMENTED BUT UNTESTED | Installer exists; required two-node run not performed |
| Detached-Media Boot | IMPLEMENTED BUT UNTESTED | Not run for this pass |
| Installed Discovery | IMPLEMENTED BUT UNTESTED | Not run for this pass |
| Installed Pairing | IMPLEMENTED BUT UNTESTED | Not run for this pass |
| Installed Secure Session | IMPLEMENTED BUT UNTESTED | Not run for this pass |
| Installed Remote Read | IMPLEMENTED BUT UNTESTED | Not run for this pass |
| Installed Remote Mutation | IMPLEMENTED BUT UNTESTED | Native commit path added; not exercised installed |
| Installed GUI Acceptance | PARTIAL | UI exists; direct bypass and acceptance gaps |
| Installed Console Acceptance | PARTIAL | Console exists; direct bypass and acceptance gaps |
| Installed GUI/CLI Lockstep | SCAFFOLDED | Not run |
| Installed IEF Reconciliation | SCAFFOLDED | Required reconstruction absent |
| Installed Mesh Membership | SCAFFOLDED | Two-node synchronized protocol absent |
| Installed Offline/Recovery | IMPLEMENTED BUT UNTESTED | Not run for this pass |
| Installed Reboot Persistence | PARTIAL | Full trust/policy/membership lifecycle not run |
| Installed UI Performance | IMPLEMENTED BUT UNTESTED | HOST timing cannot satisfy this item |
| Resource-Policy Regression | TESTED | HOST: build's existing resource-policy gate passed |
| Documentation | PARTIAL | This report matches the increment, not full M9 implementation |
