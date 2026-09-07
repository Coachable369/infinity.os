# Mesh Membership and Health

Membership is explicit and separate from discovery and trust. Trusted nodes join with a bounded role; leaving does not revoke trust. Heartbeats update observed liveness only and never grant authority. Settings exposes Trusted Nodes, Pairing, Mesh Health, Access Policy, and Security Audit over the same runtime state as Console and IOP.

Status: join/leave invariants, capacity, persistence, and liveness expiry are **TESTED**. Real two-node transport, topology exchange, and multi-node acceptance are **SCAFFOLDED**.
