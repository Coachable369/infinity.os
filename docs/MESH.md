# Mesh Membership and Health

9-B synchronized domain Join/Leave is not implemented by this increment. Existing
local membership does not prove cross-node convergence. The new remote router
rejects unsynchronized membership operations. See [9-B blockers](MILESTONE_9B_CONTROL_PLANE.md).

Membership is explicit and separate from discovery and trust. Trusted nodes join with a bounded role; leaving does not revoke trust. Heartbeats update observed liveness only and never grant authority. Settings exposes Trusted Nodes, Pairing, Mesh Health, Access Policy, and Security Audit over the same runtime state as Console and IOP.

The operation registry now executes node inventory, inspection, pairing, trust,
policy, mesh membership, and audit queries/mutations through the versioned
`NodeOperationV1` binary contract and bounded IOP endpoints. Malformed schema
versions, reserved bytes, short payloads, expired requests, and missing authority
are rejected before mutation.

Status: join/leave invariants, capacity, persistence, liveness expiry, typed IOP
management operations, malformed input, authority denial, and deadlines are
**TESTED**. Real two-node transport, topology exchange, and independently
installed two-node acceptance are **SCAFFOLDED**.
