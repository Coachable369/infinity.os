# Native node control transactions

Status: implemented and HOST-tested; installed two-VM acceptance is still required.

GUI and Console writes enter `node_client` with the complete active user/session identity. The broker issues a single-operation capability and enters the ordinary bounded IOP queue. The service revalidates capability, deadline, cancellation, endpoint ownership and reply capacity at execution. Candidates are persisted before replacing live state. There is no mutation fallback in the read dispatcher or remote dispatcher when a durable writer is absent.

The version-three node object is 12,288 bytes. It preserves identity, trust/policy, bounded security audit, public confirmed pairing transcript receipts, domain state and pending membership intent. Version-one (4,096-byte) and version-two (8,192-byte) objects remain readable. Live session keys, pending consent and remote grants are not restored. A confirmed receipt permits a new authenticated exchange with fresh boot entropy; it is not a traffic key or an authority grant.

## Inspection and reconciliation

NodeOperationV1 remains 80 bytes. Public inspection pages carry 24 bytes in the scope, lease_deadline, rights and value fields. Response flags encode total byte length in the upper 16 bits and offset in the lower 16 bits. The handle is the continuation token. Requests retain their original authorization scope. Local session inspection selects its session in the request lease_deadline; remote session inspection requires that selector to be zero and selects the authenticated wire session instead.

Node records are 128 bytes; session records 96 bytes; domain records 104 bytes. Node collections contain 32-byte identifiers, session collections 40-byte peer/handle pairs, and domain collections 32-byte identifiers. Misaligned, wrong-object, stale, expired and oversized continuations fail. Four bounded remote snapshots retain public telemetry for at most 60 seconds, bound to authenticated session, object, operation, scope and control checkpoint. Authority is revalidated on every request. Counter updates do not prevent completion of a snapshot; committed policy changes invalidate it.

`Network` packet counters are not node-control commits. Relevant durable node transactions increment the restart-persistent checkpoint. `Node.CheckpointChanged` (0x9e013) is a coalescible StateChange hint, separate from Record-class audit notifications. Settings reconstructs node and domain projections through authorized IOP, at most once per second. A periodic checkpoint query also recovers a missing final event. Failed reconstruction keeps the last valid projection marked stale. None of this work requests full-screen redraws.

## Two-node membership

Join requires independent explicit local approval on both participants. A local intent is pending, not joined. The bounded M9DM protocol uses stable domain and transaction identifiers derived from sorted participant identities and ordered revisions. Withdrawal wins same-epoch conflicts. State and retry intent commit before proposals or acknowledgements. Duplicate committed messages do not write storage. Pending intent survives service reconstruction and retries after fresh session establishment.

## Verification boundaries

`make milestone-9-test` exercises typed local admission, full local and encrypted-frame remote inspection, failed persistence, authority revocation, session reconstruction, synchronized wire Join/Leave, lost request/ACK and duplicate membership processing. HOST persistence callbacks are explicitly test adapters, not disk evidence.

`make milestone-9-service-test` exercises actual bootstrapped services, IOP, capabilities and IEF: coalesced/missing checkpoints, stale-view recovery, capability exhaustion and reclamation, and bounded refresh frequency. It is included in `build.sh`.

The rendering performance gate passed 300 retained-drag frames with a 46,542 ns p95 on this host. This is not an installed UI measurement. Fresh build, detached-ISO installation, actual cold reboot and full GUI/Console installed lifecycle acceptance remain separate obligations; this document does not declare Milestone 9 complete.
