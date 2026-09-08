# Native node transport status

The [partial 9-B remote IOP layer](MILESTONE_9B_CONTROL_PLANE.md) consumes a bounded
authenticated protocol subchannel above 9-A. Transport does not execute services.

Status: Phase 9-A TESTED — PRODUCTION WIRE / ENGINEERING GUEST; evidence is recorded in
[MILESTONE_9A_WIRE_TRUST.md](MILESTONE_9A_WIRE_TRUST.md). Overall Milestone 9 remains PARTIAL.

`kernel/runtime/node/transport.rs` is an installed-runtime discovery adapter over
the existing connected-datagram NetworkRuntime. Its `wire_trust` service adds
signed pairing, external confirmation and authenticated encrypted duplex sessions.
It does not implement encrypted remote IOP or synchronized membership.

## Authority and scheduling

Four fixed link slots reference connection-owner, send and receive capabilities.
`provision` validates the caller's scoped NetworkConnect capability, configured
static IPv4 address and selected native interface route; creates a connected UDP
endpoint; and derives only owner-scoped, connection-specific send/receive leases.
It rolls back partial failures. It cannot create firewall allowances, trust,
grants or membership. Every send/receive uses current network capability and policy
checks. Inspection is scoped to the connection owner. Missing native addresses or
routes fail; local-host fallback is forbidden.

The native NIC service calls the adapter after bounded ingress processing.
Each call visits one link; each link processes at most one received packet and
one protocol transition per second, and attempts at most one send per second.
A transition may verify, sign and perform agreement; it never waits for a peer.
There is one pending discovery response per link and one handshake/data packet per
transaction, no allocation or unbounded retry. No links or firewall allowances are
registered by default. The trusted engineering operator path supplies explicit
deployment policy and invokes the production provisioning API.

## Discovery envelope

All lengths are exact. Invalid lengths or message/version tags are rejected.

| Frame | Binary fields |
| --- | --- |
| Challenge, 40 bytes | versioned type (8), receiver nonce (32) |
| Announcement, 199 bytes | versioned type (8), echoed nonce (32), NodeId (32), public identity (32), minimum/maximum version (2 each), service bits (8), endpoint (19), signature (64) |

Endpoint encoding is a family byte, 16 address bytes (IPv4 zero-padded), and a
network-order port. The signature covers the entire announcement prefix,
including the reachable endpoint. No service bits are claimed yet. The public
key must hash to the supplied NodeId, and the endpoint must match the connected
peer. A bound link cannot silently switch identities.

Freshness uses a domain-separated SHA-256 challenge derived from **fresh boot
entropy**, a checked monotonic counter and the local connection reference.
This seed is never restored from persistent node identity. The receiver allows
five local seconds for the response and consumes its nonce once. Peer uptime
is not interpreted as local uptime. Challenges renew every ten seconds, with
bounded one-second retries for ARP/backpressure. Link detachment and disabled
discovery discard outstanding challenge state. Discovery cannot grant trust.

Successful discovery commits authoritative state before returning a change.
The runtime then publishes Discovered/Recovered/Offline IEF outside the mutable
runtime borrow. Peer observations expire after 30 seconds. Address/link loss and
Offline profile activation quiesce negotiation and close sessions. Full lifecycle
event delivery/reconciliation remains outside 9-A.

## Evidence

`make milestone-9-test` exercises the adapter through the actual Ethernet/ARP/
IPv4/UDP encoding, connection queues, capabilities and firewall service in a
host harness. Assertions cover independent identities/clocks, actual endpoint
binding, one discovery transition, no automatic trust/session/grant, replay
without liveness refresh, invalid signatures with valid UDP checksums,
malformed/oversized node payloads, revocation and Offline quiescence.

Those cases are **TESTED / HOST**. `make milestone-9-wire-trust-test` additionally
uses two independent native E1000 guests for production discovery and trust
establishment; see the phase report for the exact engineering evidence boundary.
The normal installed-kernel binary parity gate covers packaging of this module
as part of the existing core node runtime. It does not prove two-node installed
lifecycle or final installed GUI acceptance.
