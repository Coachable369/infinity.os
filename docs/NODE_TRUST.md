# Node Trust and Pairing

Trust is explicit and independent from discovery and mesh membership. Pairing is a leased transaction with a human-verifiable fingerprint and six-digit code. Confirmation is reserved to Trusted UI. Revocation and blocking close and zeroize sessions and revoke remote grants without reboot.

The Settings flow now keeps selection by stable `NodeId` and requires a six-digit
code entered by the operator while a Trusted UI secure-input lease is active.
The expected code is never read back into the confirmation control. Cancellation
is terminal: a cancelled transaction cannot later be confirmed.

Status: explicit human approval, correct/wrong code, cancellation, expiry,
revocation, blocking, and the bounded typed IOP pairing path are **TESTED** in the
host behavioral harness. GUI code entry is **IMPLEMENTED BUT UNTESTED** in a
two-machine UI run. Cross-machine transport remains **SCAFFOLDED**.
