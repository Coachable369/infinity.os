# Per-Node Access Policy

Each peer has a versioned twelve-category policy for object, namespace, compute, AI, service, event, storage, clipboard, device, diagnostics, mesh, and administrative access. Decisions are Deny, Allow, SessionOnly, or Leased and are bounded by scope and expiry. Remote grants bind peer, operation, scope, rights, lease, and revocation state.

Status: deny-by-default, narrow grants, scope denial, lease expiry, and immediate revocation are **TESTED**. Distributed capability-envelope transport is **SCAFFOLDED**.
