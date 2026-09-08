# Nodes

See [9-B control-plane status](MILESTONE_9B_CONTROL_PLANE.md) for the tested remote
IOP slice and the explicit blockers preventing a complete distributed-control claim.

Discovery identifies reachable peers but grants no trust, session, membership or
capability. [Node transport](NODE_TRANSPORT.md) owns scoped native endpoints;
[wire trust](MILESTONE_9A_WIRE_TRUST.md) defines Phase 9-A pairing and authenticated
duplex sessions. [Node identity](NODE_IDENTITY.md) defines stable cryptographic
identity, and [node trust](NODE_TRUST.md) defines explicit approval/revocation.

Milestone 9 overall remains PARTIAL. Remote IOP convergence and full independent
installed-system lifecycle acceptance are subsequent phases, not inferred from
successful engineering-guest packet exchange.
