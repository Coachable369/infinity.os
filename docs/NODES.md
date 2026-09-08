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

## Authenticated operator remote requests

The installed Console uses `node_operator` over the ordinary native IOP router.
An authenticated operator explicitly grants the other peer one operation using
`node capability-grant node:<full-id> name=inspect seconds=300 confirm=true`.
The native Trusted UI lease is required; the command without confirmation shows
the scope and consequence without issuing authority. Separate grants support
`domain-inspect` and `policy-update`. `node capability-revoke <handle>` revokes a
grant while peers remain running. Grants do not survive reboot.

`node remote-read node:<full-id> grant=<handle>` queues inspection of the caller's
own peer entry. `node remote-domain` selects only the shared domain. Use
`node remote-result <request-id>` to collect the submitting session's actual
completion. Complete 128-byte node and 104-byte domain records are automatically
assembled from pinned native pages; malformed/mixed pages are rejected. The
original 30-second deadline is not extended for continuation pages.

`node remote-policy-update node:<full-id> grant=<handle> name=object value=deny`
requires a distinct mutation grant and explicit peer control policy, not merely
an inspection grant or established session. The existing durable service commits
before its completion and IEF publication. There are eight pending operator
requests. Session loss retires their local capabilities; no automatic mutation
retries are performed. HOST ownership, bounds, timeout and page-integrity tests
are TESTED; the complete installed remote sequence remains IMPLEMENTED BUT
UNTESTED pending the [pairing continuation](PAIRING_LIFECYCLE_DEBUG.md).
