# Pairing lifecycle investigation

The owner is `WireTrust::transactions`, selected by stable peer NodeId and wire
transaction ID. Settings reads `Verification`; it does not own its lifetime.

| Transition | Mutation and durability |
|---|---|
| Begin / incoming INIT | `begin` / `process` allocate a transaction and NodeRuntime pairing handle; new identities cannot replace a pending transaction |
| RESPONSE / PROOF / ACK | Signed identity and agreement checks populate the digest; `verification_ready` sets the code and PendingVerification, zeroizing the agreement secret |
| Local consent | GUI/Console use `commit_wire_control`, cloning both owners, validating consent, persisting the candidate, then replacing live state and publishing committed IEF |
| Remote consent | Signed CONFIRM validates the exact transaction and digest; confirmation bits advance; dual consent calls `commit_wire_pairing` to stage and persist trust plus public receipt |
| Completion | Confirmed remains observable until expiry or a separately authorized fresh-session exchange; completion itself does not erase the receipt |
| Cleanup | `terminate` clears pending packets and secrets, cancels a pending pairing or closes its session; explicit cancel, expiry, invalid/stale peer, disconnect, or fatal handshake authentication errors can invoke it |
| Refresh | Settings presentation and IOP reconstruction are readers; repeated reads do not replace authoritative transaction state |

Diagnostic snapshot words 56–67 expose state-plus-one (zero means absent),
confirmation bitmap, transaction expiry, authoritative peer last_seen, wire error
plus one, transport error plus one, termination time, termination source line, and
the public transaction ID. These remain readable after the confirmation view
disappears. No private or traffic keys, agreement material or input digits are added.
Source lines are diagnostics only, never acceptance assertions.

The original delayed HOST regression had network capability and policy expiry at
15 while submitting consent at 45/65. The trace showed PendingVerification becoming
Failed after last_seen aged beyond 30. The test now explicitly provisions the same
narrow network authority through its scenario duration and supplies a HOST pairing
persistence callback. Existing short-lived NIC fixtures still expire at 15.
Production deadlines, scheduling and authority are unchanged. Immediate, delayed,
near-expiry and expired-consent scenarios pass. This fixture correction is not a
diagnosis or fix of the installed failure.

The diagnostic ISO build passed; x86_64 SHA-256:
`518ed0697e22e1defadb5ef334278550b20e3372bdc01ae30e6e0259c01f2b1d`;
ARM64: `ea4a39ebb790291e8ef53792bba3486ba045a92721364ddd4e43222ede282773`.
The independent installed evidence directory is
`/tmp/infinity-ms9-installed-20260908-e`. No installed pairing success is claimed
until that run passes. The previous rejected scheduling patch remains absent.
