# Native TCP progress — September 22

The production HTTPS actor uses `runtime/http/transport.rs` and its existing
smoltcp IPv4 TCP implementation. The general Network connection manager still
supports loopback/configured datagrams; this change does not introduce generic
application TCP sockets or claim a complete TCP/IP implementation.

## Corrected stream lifecycle

- Read and write during SYN negotiation return `WouldBlock`, not false EOF or
  premature disconnection.
- Reset/refusal of an outbound connection returns `Disconnected`, not successful
  zero-byte completion. Cancellation and deadline errors retain precedence.
- A received FIN permits buffered data to drain before EOF. The other direction
  remains writable for TCP half-close.
- Reconnecting resets previous FIN state. No capabilities, policy decisions,
  scheduler ownership or buffer limits change.

## Behavioral evidence

`cargo test --manifest-path kernel/runtime/http/Cargo.toml --lib transport::tests`
passes four tests, exchanging real serialized ARP/DNS/TCP frames through the
existing stack with a simulated NIC. Tests cover a lost SYN, corrupted data,
reordered/duplicated segments, backpressure, exact ordered binary delivery,
handshake-pending state, reset, FIN drain, half-close, cancellation and deadlines.
These are host protocol tests, not an installed external-network test.

Both installed and live kernels link this same transport library. Updated ISO
packaging and ISO-detached verification must be recorded separately.

September 22 build evidence: the full 12-test HTTP library suite passed, ARM64 and
x86_64 installed kernels linked, and `sh tools/build-hermes.sh` completed including
Hermes/Ministral binary payload checks. Reassembling the ARM64 installer kernel
shards compares byte-for-byte equal to the installed kernel. The default
reprovision ISO at `builds/InfinityOS-aarch64.iso` is updated.
The running VirtualBox guest remains unmodified and at login; no new installed
network exchange is claimed. The ISO's legacy filename does not change its model set.

## Next transport increments

1. Capability-bound general stream connections through Network/IOP, bounded
   connection tables and endpoint isolation; reuse the TCP implementation.
2. Automatic address/route/DNS readiness and configuration-change cancellation.
3. Broader fault-injection coverage: zero-window reopening, sustained loss,
   connection reuse, sequence wraparound and FIN loss.
4. IPv6 and its discovery/addressing prerequisites, then controlled installed
   interoperability testing. Do not conflate this with HTTP/curl option parity.
