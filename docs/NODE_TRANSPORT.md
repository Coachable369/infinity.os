# Native node transport status

Status: **PARTIAL — not Milestone 9 acceptance**.

`kernel/runtime/node/transport.rs` is an installed-runtime discovery adapter over
the existing connected-datagram NetworkRuntime. It does not implement pairing,
session negotiation, encrypted remote IOP or synchronized membership yet.

## Authority and scheduling

Four fixed link slots reference pre-existing connection-owner, send and receive
capabilities. Attaching cannot create a connection, firewall allowance, trust,
grant or membership. Every send/receive uses the network service's current
capability and policy checks. Inspection is scoped to the connection owner.

The native NIC service calls the adapter after bounded ingress processing.
Each call visits one link; each link processes at most one received packet and
one signature operation per second, and attempts at most one send per second.
There is one pending response per link, no allocation, wait or unbounded retry.
No links are registered by default. Operator-facing endpoint provisioning is
still missing; the attached-link behavior is tested through explicit fixtures.

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
The runtime then publishes Discovered/Recovered IEF outside the mutable runtime
borrow. Full lifecycle events, event-delivery retry/reconciliation, offline
transitions and authenticated reconnect remain incomplete.

## Evidence

`make milestone-9-test` exercises the adapter through the actual Ethernet/ARP/
IPv4/UDP encoding, connection queues, capabilities and firewall service in a
host harness. Assertions cover independent identities/clocks, actual endpoint
binding, one discovery transition, no automatic trust/session/grant, replay
without liveness refresh, invalid signatures with valid UDP checksums,
malformed/oversized node payloads, revocation and Offline quiescence.

This is **TESTED / HOST**, not native VM or detached-install node acceptance.
The normal installed-kernel binary parity gate covers packaging of this module
as part of the existing core node runtime. It does not prove the missing
operator workflow or two-node installed lifecycle.
