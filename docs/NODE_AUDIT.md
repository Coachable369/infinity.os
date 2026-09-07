# Node Security Audit

Pairing, revocation, block/unblock, sessions, policy, grants, and membership produce bounded structured records after authoritative state commits. Records exclude secrets, keys, tokens, and payloads. Console and Settings project the same typed state.

Committed node mutations publish typed Mesh-domain IEF notifications with stable
node payloads and correlation/causation IDs. Subscription capability is checked
again at delivery, so revocation suppresses later delivery. Consumers reconcile
authoritative state through node IOP queries rather than treating events as state.

Status: ordering, bounded rotation, security record generation, correlated event
delivery, and delivery-time revocation are **TESTED**. Durable append-only
retention beyond the runtime ring is **SCAFFOLDED**.
