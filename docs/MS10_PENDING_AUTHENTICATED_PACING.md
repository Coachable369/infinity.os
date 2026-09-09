# Pending authenticated DATA pacing proposal

Status: IMPLEMENTED in current source; final ISO/installed verification pending.

The final closure pass restored the bounded DATA gate and deferred retry
consumption until successful network admission. Behavioral host checks now pass
for forged tags, unknown sessions, replay rejection, all four occupied links,
20-second transmit backpressure for DATA, and bilateral CONFIRM delivery after
20 seconds of full NIC queues. The installed historical-source pairing rerun
reproduced accepted local approvals with zero emitted CONFIRM frames; queue
pressure is a demonstrated mechanism, not yet the proven installed root cause.

Current evidence: `/tmp/ms10-closure-ms9.log`,
`/tmp/ms10-confirm-backpressure.log`, `/tmp/ms10-four-link.log`,
`/tmp/ms10-closure-performance.log` (62 tests), and
`/tmp/ms10-closure-input.log` (eight executable input/UI harnesses, exit 0).
Actual installed transfer throughput and loaded desktop responsiveness remain
mandatory acceptance, not inferred from simulated cadence.

## Historical proposal below

The companion `MS10_PENDING_AUTHENTICATED_PACING.patch` preserves the exact
experimental transport changes. They were removed from runtime source at the
acceptance stop boundary. Existing control/signature and DATA pacing therefore
remain at the previously accepted baseline.

## Observed host behavior

The isolated `transport_cadence_measurement` fixture establishes real signed trust
and an encrypted session, then polls the production transport every simulated
millisecond with bounded NIC pumping. It verifies exact ordered payload bytes.

| Source | Simulated duration | Poll opportunities | Admitted | Received | Payload bytes |
| --- | ---: | ---: | ---: | ---: | ---: |
| Accepted baseline | 10 seconds | 10,000 | 11 | 10 | 640 |
| Archived experimental patch | 10 seconds | 10,000 | 161 | 160 | 10,240 |

These are host scheduler-cadence measurements, not installed throughput. Logs:
`/tmp/ms10-cadence.log` and `/tmp/ms10-cadence-fast.log`. The retained fixture
assertions target the baseline, not the experimental rate.

32 KiB needs 512 current 64-byte replica chunks (547 at 60-byte upload windows).
The baseline one-frame/second gate cannot meet a 180-second distribution gate,
even before acknowledgements, verification, persistence, and multiple replicas.

## Proposed mechanism

Sixteen exact established-session DATA attempts per link per second, retaining
one control/signature operation per link per second, one RX/TX per poll and the
existing round-robin link scheduler. Canonical session-header classification
does not authenticate packets: all tags, replay checks, live trust and authority
remain checked by the existing receiver. Invalid tags consume the DATA budget;
unknown headers remain under the control budget. No queue expansion or TTL
changes. A non-mutating outgoing peek avoids consuming retry state before
network admission.

## Required before activation

- Dedicated forged-tag, unknown-header and control-budget adversarial tests.
- Fairness and backpressure tests across all four links.
- Full normal milestone-9 regression tests with bounded fixture stacks.
- Pointer, redraw and performance regression gates.
- Kernel/build verification, fresh ISO packaging and actual installed transfer
  timing; the theoretical 16-frame ceiling does not guarantee 180 seconds.

Only the targeted cadence test compiled and passed for the experimental patch.
No additional implementation iterations or tests were run during cleanup.
