# Pairing lifecycle investigation

## Current continuation boundary

Run j completed two blank-disk installations within the 900-second host bound,
detached onboarding and authenticated cold reboots with distinct persistent IDs.
Its deliberate ten-key/60-ms-spacing burst then exposed lost command input:
node A accepted six keys. The captured execution address resolves to
`DisplayDevice::fill_rounded_rect_alpha`; input synchronously reconstructs the
entire cached command window before returning to controller polling.

Command-input-only changes now retain window history/chrome and update only the
prompt's bounded premultiplied cache region. Output, appearance, geometry and
other content changes retain their existing reconstruction paths. The active
painter test proves partial-cache/full-render pixel equivalence, unchanged
surrounding pixels, and translation reuse after patching. This is HOST evidence;
the unchanged installed rapid-input assertion must pass on newly built media.
Register failure files now retain only instruction/stack addresses, never SIMD
or general register contents. No protocol deadline or keyboard cadence changed.

Run i reached storage verification but exceeded the harness's 300-second install
bound before the synchronous installer returned. Its last console snapshot still
describes step 6; the captured display shows installation progress, not a panic.
This is not pairing evidence. A subsequent run uses a 900-second host verification
bound and captures CPU registers on failure. No guest protocol deadline changes.

The Console remote operator path now assembles the complete 128-byte Node.Inspect
and 104-byte Domain.Inspect records automatically. Pages retain the original
operation capability, peer grant, correlation and 30-second request deadline.
Object, operation, schema, total size, offset and pinned token are checked before
copying. Results remain bound to the submitting authenticated session; closing
that session retires authority and pending requests. Two HOST broker tests pass,
including exact native-record reconstruction and malformed-page rejection.
Installed remote acceptance remains pending.

Earlier keyboard timings measured submission to the next periodic diagnostic
snapshot, not input completion. They must not be interpreted as actual per-key
UI latency. Snapshot publication now also runs after completed input/presentation
so subsequent installed measurements can separate the observation delay.

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

## Confirmed installed failure and correction

Run e reached both consent bits on A at clock 1035, but the native durable
commit returned StateCorrupt. At 1042 stale-peer cleanup terminated that still
uncommitted transaction, before its 1084 pairing deadline. B committed successfully.
Read-only inspection of the independently installed object stores found A at
generation 19 with all 32 version slots occupied, versus B at generation 18 with
31 occupied. Six A versions belonged to the mutable node checkpoint. This is
version-table exhaustion, not a Settings ownership or scheduling defect.

`node_state_commit` now uses an explicit bounded checkpoint replacement. Old data
remains allocated while new content is written; only the atomic metadata commit
retires the checkpoint's old versions. Ordinary document writes retain history.
The checkpoint still contains its own bounded durable audit and receipt state.
Authenticated valid CONFIRM refreshes peer liveness even when local storage fails;
the same dual-confirmed, unexpired transaction permits an explicit operator retry.
No trust is granted before durable success and no lease/retransmission limit changes.

Behavioral storage coverage saturates the old version API, verifies bounded
replacement across 64 subsequent commits, preserves another object's history,
injects content/metadata write failures, remounts, and retries successfully.
The native packet regression injects a failed final commit, asserts no trust and
retained consent after 40 ticks, then retries under the original authority/lease.
The 25 correlation/pairing and six service tests pass. Installed proof against
fresh fixed ISOs remains a separate gate.

Fixed-source `./build.sh` completed successfully, including extracted installed
kernel parity. Release hashes:

- x86_64: `ed0e2350735cec8ab6a9460229d4ba5af8313c98ecf5be8cfe5fc67b248660d1`
- ARM64: `c5feb84d982a2bd0ca1862df21998cb0454f274d7a89ef66a68643e6d029f4bb`

Build log: `/tmp/ms9-pairing-fix-build.log`. Post-build correlation, durable
mutation, remote-IOP engineering-wire and native-NIC gates also passed
(`/tmp/ms9-checkpoint-security-gates.log`). Run f uses fresh independent installs
from these artifacts, not updated or injected kernels. Its two installations and
first detached boots passed; the remaining installed lifecycle is in progress.
`installed-object-test --bounded-node-checkpoint` adds a read-only artifact
assertion for the one-version checkpoint and full 12-KiB payload. The previous
run-e disk fails this assertion (six retained checkpoint versions), as expected.

## Second installed failure: sparse polling

Run f passed installation, onboarding, cold authentication, native endpoint
configuration, discovery and matching verification. Both installed checkpoint
artifacts now pass the bounded-history assertion. Pairing nevertheless expired
through stale-peer cleanup during digit entry: A last_seen 906, termination 937,
pairing expiry 1001, input expiry 963. No durable-write error occurred. B likewise
stopped updating liveness at 908 and was terminated at 940.

`pairing_single_runtime_poll` deterministically reproduced transcript loss when
each runtime receives one polling opportunity per tick instead of the fixture's
four. The transport advanced its cursor through all four capacity slots, including
three absent links. It therefore serviced the only active link once every four
calls, allowing the five-tick discovery challenge to expire before its response
was consumed. Selecting the next occupied slot makes the regression pass. The
per-call one-link bound and per-link one-received-packet-per-second limit remain
unchanged, as do nonce, discovery, pairing and input deadlines. Snapshot words
68–70 now expose poll calls, serviced links and received packets for installed
verification. This is direct sparse-poll reproduction, not restoration of the
previous rejected scheduling patch. Installed acceptance must be repeated from
newly built artifacts before this second correction is accepted.

## Installed repetition and authoritative lease ownership

Run g (`/tmp/infinity-ms9-installed-20260908-g`) passed dual confirmation with
the final digit deliberately delayed five node-clock ticks, fresh secure session,
synchronized join/leave, and disk-only cold reboot preserving both distinct
identities, trust and membership records. Its pinned x86 ISO SHA-256 is
`44a9d74394bfb4ce19caae0ef591c457cddec1544a8a916ccf8a80fcaa1e1f6f`.
This is installed QEMU evidence, not ARM64/VirtualBox evidence or full MS9 closure.

Run h, rebuilt with the readability corrections, passed both clean installations,
onboarding, detached cold authentication, discovery, matching verification, and
both explicit operator submissions. It did NOT complete dual trust. At clock 673,
A's last authenticated discovery observation was 642 and `WireTrust::tick`
terminated the transaction, despite its pairing expiry of 727. B committed dual
consent at 674; A rejected the subsequent signed confirmation because its state
was already terminal. No persistence error occurred. The earlier occupied-slot
fix therefore did not fully resolve installed pairing reliability.

The confirmed remaining fault is lease coupling: discovery's 30-tick liveness
hint owned destruction of an already mutually authenticated 120-tick pairing.
`checked_transaction_peer` now preserves verified confirmation under the original
transaction deadline, validating the stored peer public key and current
block/revoke/protocol state. New handshakes and established sessions still use
current discovery liveness. Explicit connection loss, Offline-profile shutdown,
revocation, cancellation, and transaction expiry remain terminal. No deadline,
retry limit, or automatic approval changed.

`verified_pairing_outlives_discovery_hint` drops native discovery traffic for 40
ticks after initial consent. Before the fix it loses the verification transaction;
afterward, signed second consent completes both peers. It additionally rejects
revoked and actually expired consent. All 28 HOST pairing/security tests pass.
Fresh installed repetition of this last correction remains required.

The run-h pairing screenshot was reviewed directly: the title/subtitle, wrapped
tabs, control descriptions, code, fingerprint and footer no longer overlap at the
tested 2048-by-2048 framebuffer. Frame telemetry returned zero usable samples;
this is not a performance pass.
