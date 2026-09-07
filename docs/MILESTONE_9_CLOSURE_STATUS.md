# Milestone 9 closure status

This milestone is **PARTIAL**, not complete. Host fixtures are not independent
installed-node acceptance. This report records the narrowly scoped security
closure work; it does not certify the entire pre-existing implementation.

## Behavioral security corrections

The node runtime now rejects direct promotion of unpaired identities, duplicate
pending pairings, cancellation of terminal pairings, and confirmation after a
superseding revocation or block. Terminal pairing slots are reusable without
reusing their handles. Expiry clears pending trust as well as transaction state.
Confirmation rechecks the selected peer's fingerprint and protocol compatibility.
Restricting or removing trust closes sessions and revokes existing grants;
restoring trust does not resurrect those grants. Remote authorization checks
current trust on every call.

The IOP node dispatcher revalidates capability authority after dequeue, closing
the revocation/expiry window between queue admission and execution.

Coverage lives in `tools/milestone9-harness/pairing_acceptance.rs` and the existing
typed IOP harness. Assertions exercise state, handles, errors and grant outcomes,
not source code or displayed text. These cases are host-tested; they do not prove
human comparison of two independent screens.

## Installer and performance protection

The changes are in the kernel's shared node service and IOP implementation, used
by live and installed kernels. No additional component or live-only resource is
introduced. `build.sh` now gates ISO publication on Milestone 9, compositor
performance and resource-policy tests, in addition to its existing binary
installed-kernel parity and UI/input tests. No painting, pointer, scheduling,
window movement or resource-budget implementation was changed by this pass.

## Still required before COMPLETE

- Native network-carried node discovery and mutually authenticated session
  negotiation, including independent verification material on both machines.
- Session direction separation and independent handle negotiation: the existing
  host round-trip fixture uses matching local session IDs and exercises only one
  traffic direction. This is not adequate secure duplex transport acceptance.
- Runtime GUI and Console dispatch convergence through capability-validated IOP;
  the Console mutation adapter still calls the service executor directly.
- Full typed inspection responses: some existing inspection operations return
  counts rather than the selected object's detail.
- Complete post-commit IEF coverage and runtime sequence-gap reconciliation.
- Per-node and authorized-domain workflows, policy enforcement and restart
  behavior proven across actual independent machines.
- Two blank-disk installations, media detached, with interactive GUI/Console
  trust lifecycle, persistence, negative-path and performance acceptance.

Existing documentation marks network-carried discovery and session negotiation
as **SCAFFOLDED**. They must not be represented as working merely because the
host harness can exchange structures directly between two runtime instances.
