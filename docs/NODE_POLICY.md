# Per-Node Access Policy

Each peer has a versioned twelve-category policy for object, namespace, compute, AI, service, event, storage, clipboard, device, diagnostics, mesh, and administrative access. Decisions are Deny, Allow, SessionOnly, or Leased and are bounded by scope and expiry. Remote grants bind peer, operation, scope, rights, lease, and revocation state.

The node Console mutations and Settings controls resolve an explicitly selected
stable `NodeId` and update the same authoritative `NodeRuntime` policy objects.
`NodeOperationV1` carries policy mutations through bounded IOP queues with
service-call capability and deadline validation.

Status: deny-by-default, narrow grants, scope denial, lease expiry, immediate
revocation, IOP denial/deadline handling, and policy mutation are **TESTED**.
Distributed capability-envelope transport is **SCAFFOLDED**.
