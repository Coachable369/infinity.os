# Installed reset-pairing evidence

Status: **TESTED failure on pinned runtime `52c15e8`**. This is not a milestone completion claim.

The four independently installed, media-detached disks were preserved. A and B each performed normal Console `node trust-revoke` and `node unblock`; exact typed peer states changed to Revoked and then Untrusted, with zero sessions. Both Settings panels were reopened normally before a fresh protected pairing ceremony.

| Observation | A | B |
| --- | --- | --- |
| First PendingVerification guest clock | 233 | 231 |
| First LocallyConfirmed guest clock | 269 | 273 |
| Existing ceremony expiry | 346 | 348 |
| Remaining lease at local confirmation | 77 seconds | 75 seconds |
| Final approvals mask | 1 (local only) | 1 (local only) |
| Final state | Expired | Expired |
| Expiry site | 517 | 517 |

Both authenticated fingerprints and transaction IDs matched. The bounded Ethernet observer retained 1,162 metadata records and dropped zero. It observed two A→B HELLO frames, one B→A RESPONSE, one A→B PROOF, and one B→A ACK. **No CONFIRM frame appeared in either direction.** Payloads, keys and secrets were not retained.

The first established failure boundary is therefore local-confirmed state to actual CONFIRM transmission—not acceptance of a received CONFIRM. The old transport consumed retry attempts before checking network-send admission; queue pressure is a candidate explanation, not something proven by the old diagnostics. New pacing/admission source requires separate installed verification.

Evidence:

- `/tmp/ms10-repeated-pairing-trace.log` — finite process exited 1; later cycles did not run.
- `/tmp/infinity-ms10-distributed-installed-52c15e8/repeated-pairing-trace.json` — structured native counters and safe Ethernet metadata.
- `node-1/pairing-lifecycle.jsonl` and `node-2/pairing-lifecycle.jsonl` in that directory — actual stage/clock transitions.
- `focused-pairing-trace-original-pass.json` in that directory preserves the earlier successful ceremony; that success did not establish reliability.

All four guests were stopped normally by the harness after capture. No disks were deleted, no TTL was extended, no trust was injected, and no automatic retry was used.

## Pacing-generation repeated installed verification

Status: **TESTED: three consecutive reset/pair/Settings-refresh cycles passed** on the independently installed pacing-generation snapshot `40ccb79`. This is not evidence for later metadata/repair code or full MS10 completion.

The finite process exited 0. Four media-detached guests configured twelve directed discovery endpoints. A and B then completed three ordinary revoke → unblock → protected confirmation → reopened Settings cycles. All three authenticated transaction fingerprints were distinct, both peers reached Trusted each time, and the bounded observer saw 36 CONFIRM frames in each direction with zero metadata drops. No authority bypass, TTL extension, retransmission by the harness, or injected trust was used.

Evidence:

- `/tmp/ms10-pacing-repeated-pairing.log` — process 7766 exited 0.
- `/tmp/ms10-final-pacing-installed/repeated-pairing-trace.json` — three structured successful cycles and safe wire metadata.
- `/tmp/ms10-final-pacing-installed/result.json` — independent installed/cold-boot bootstrap provenance.

All four guests were stopped normally after the test. The old failing receipt remains preserved separately. The observed missing-CONFIRM failure did not recur in these three tests; this is a bounded reliability result, not a universal claim that pairing cannot fail.
