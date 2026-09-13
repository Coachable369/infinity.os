# Native HTTP transport dependencies

## Implemented foundation

`kernel/runtime/http` is a `no_std`, no-allocator library, shared by the
kernel's live and installed build configurations. It uses smoltcp 0.12.0 with
only Ethernet, IPv4 and TCP enabled, plus httparse for bounded HTTP headers.
Host sockets, libc, async frameworks, TLS and default logging are disabled.
Upstream: [smoltcp](https://github.com/smoltcp-rs/smoltcp).
Upstream TCP handles checksums, retransmission, ordering and flow control.

The adapter exposes one outbound stream backed by caller-owned RX/TX buffers
and one socket slot. Each poll handles at most four incoming packets and a
bounded egress pass. Connect is asynchronous, sends report partial capacity,
reads distinguish backpressure from EOF, and cancellation makes subsequent IO
fail. A hard transaction deadline also bounds a peer that keeps sending data.
Callers supply fresh entropy for sequence randomization and monotonic time.

`EthernetQueue` provides four 1514-byte slots per direction. The NIC must retain
pending output until submission succeeds. The request encoder rejects CR/LF
injection. Response headers are capped at 8 KiB and 32 fields and reject
conflicting Content-Length / Transfer-Encoding values. Body framing is reported
as empty, fixed length, chunked or connection-close; chunk decoding is not yet
implemented.

## Behavioral verification

`make http-transport-test` is included in `build.sh`. Tests run real protocol
state machines over in-memory Ethernet, without host networking:

- Dropped SYN and corrupt data checksum recovery, ordered 7000-byte delivery.
- Partial sends, receive backpressure, cancellation and hard timeout.
- NIC FIFO capacity and retention until transmit acknowledgment.
- Exact request bytes, injection rejection and output capacity limits.
- Fragmented response headers, body boundary and ambiguous framing rejection.

All six tests pass. The ARM64 installed kernel links successfully, and x86_64
kernel checks pass with and without the installer feature. Plain `cargo check`
initially failed because this host does not have prebuilt bare-metal `core`;
the repository's `-Z build-std=core` build path succeeds. Existing kernel
warnings remain. These checks are not installed-network acceptance.

## Integration still required

The library is available to both kernel configurations, but it is not yet
connected to the production NIC pump or exposed as an application service.
Do not report HTTP or HTTPS working on an installed system yet. Required next
steps are capability-checked stream operations, NIC demultiplexing that preserves
existing UDP traffic, configuration/revocation teardown, DNS resolution, body
decoding, TLS with certificate and hostname verification, and ISO-detached
network proof. A separate System Generation component is not introduced by
this library-only dependency change; integration must add behavioral installed
parity evidence before release acceptance.

No certificate bypass, fabricated success, host proxy or plaintext fallback is
provided. Interface creation and connection authorization must occur outside
paint/input paths. The service must cancel on permission revocation and must
not reuse an interface after its address or routing configuration changes.
