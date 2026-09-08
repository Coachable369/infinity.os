# Node Trust and Pairing

Remote IOP revalidates current peer trust and narrow authority after dequeue;
authenticated transport is insufficient. [9-B remains partial](MILESTONE_9B_CONTROL_PLANE.md),
with installed remote mutations disabled pending the commit/IEF lifecycle.

Trust is explicit and independent from discovery and mesh membership. Pairing is a leased transaction with a human-verifiable fingerprint and six-digit code. Confirmation is reserved to Trusted UI. Revocation and blocking close and zeroize sessions and revoke remote grants without reboot.

The Settings flow now keeps selection by stable `NodeId` and requires a six-digit
code entered by the operator while a Trusted UI secure-input lease is active.
The expected code is never read back into the confirmation control. Cancellation
is terminal: a cancelled transaction cannot later be confirmed.

Status: explicit human approval, correct/wrong code, cancellation, expiry,
revocation, blocking, and the bounded typed IOP pairing path are **TESTED** in the
host behavioral harness. GUI code entry is **IMPLEMENTED BUT UNTESTED** in a
two-machine UI run. Phase 9-A adds production wire pairing with independent
verification and explicit external approvals; see
[the phase evidence](MILESTONE_9A_WIRE_TRUST.md). It does not certify that GUI flow.

## Duplex mechanism

Sessions now use identity/transcript-bound directional keys, separate TX/RX
sequence state and a shared protocol reference independent of local handles.
Both traffic keys are zeroized on session invalidation. Reused session material
is rejected while its reference remains in the bounded session table. Reconnect
must supply fresh agreement material and a fresh authenticated transcript.
The duplex mechanism is host-tested; native handshake and installed two-node
acceptance remain unfinished. See [the finish-line report](MILESTONE_9_FINISHLINE_STATUS.md).
