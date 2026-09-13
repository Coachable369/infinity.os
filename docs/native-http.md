# Native HTTP transport dependencies

## Implemented foundation

`kernel/runtime/http` is a `no_std`, no-allocator library, shared by the
kernel's live and installed build configurations. It uses smoltcp 0.12.0 with
only Ethernet, IPv4 and TCP enabled, plus httparse for bounded HTTP headers.
Host sockets, libc, general async frameworks and default logging are disabled.
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
as empty, fixed length, chunked or connection-close. The HTTPS layer now decodes
these forms, including bounded trailers and fragmented chunk input.

## Behavioral verification

`make http-transport-test` is included in `build.sh`. Tests run real protocol
state machines over in-memory Ethernet, without host networking:

- Dropped SYN and corrupt data checksum recovery, ordered 7000-byte delivery.
- Partial sends, receive backpressure, cancellation and hard timeout.
- NIC FIFO capacity and retention until transmit acknowledgment.
- Exact request bytes, injection rejection and output capacity limits.
- Fragmented response headers, body boundary and ambiguous framing rejection.

The original six tests pass. The ARM64 installed kernel links successfully, and x86_64
kernel checks pass with and without the installer feature. Plain `cargo check`
initially failed because this host does not have prebuilt bare-metal `core`;
the repository's `-Z build-std=core` build path succeeds. Existing kernel
warnings remain. These checks are not installed-network acceptance.

## Integration still required

The library is available to both kernel configurations, but it is not yet
connected to the production NIC pump or exposed as an application service.
Do not report HTTP or HTTPS working on an installed system yet. Required next
steps are capability-checked stream operations, NIC demultiplexing that preserves
existing UDP traffic, configuration/revocation teardown, DNS resolution and
ISO-detached network proof. A separate System Generation component is not introduced by
this library-only dependency change; integration must add behavioral installed
parity evidence before release acceptance.

No certificate bypass, fabricated success, host proxy or plaintext fallback is
provided. Interface creation and connection authorization must occur outside
paint/input paths. The service must cancel on permission revocation and must
not reuse an interface after its address or routing configuration changes.

## Authenticated HTTPS mechanism

`https::get` performs a TLS 1.3 handshake before sending any HTTP application
bytes. `embedded-tls` implements the record/handshake protocol, RustCrypto
implements ECDSA and record cryptography, and rustls-webpki validates certificate
paths, validity dates, server usage and subject names. CertificateVerify is
checked against the validated leaf and handshake transcript. The verifier
requires an explicit hostname, nonzero trusted wall clock and root set. No
`NoVerify`, plaintext fallback, host proxy or dynamically downloaded trust is
used. Versioned Mozilla anchors are available through `tls::system_roots`.

Initial support is AES-128-GCM/SHA-256 with ECDSA P256/SHA256 or P384/SHA384
certificates. RSA, Ed25519 and other certificate algorithms fail closed. This
is not universal public-web compatibility. CRL/OCSP revocation retrieval,
TLS 1.2, redirects, HTTP/2, compression and mutual TLS are not implemented.

The request API takes caller-owned record, request and response buffers, rejects
response buffers larger than 1 MiB, caps headers and certificate-chain storage,
and returns only a complete decoded response. Premature EOF fails. At most
eight informational responses are accepted; protocol upgrades are rejected.
`async_stream` suspends IO on TCP backpressure and wakes through an explicit
native service pump. The owner must authorize endpoints, enforce revocation,
provide fresh entropy/time, and cancel the stream when dropping a request.

Verification added:

- Independent TLS server interoperability, with exact decrypted body bytes.
- Wrong hostname, root, date and corrupted-certificate rejection.
- Fragmented chunk framing, malformed terminator and truncated EOF checks.
- `make native-https-test`: a bare-metal QEMU client uses the production e1000
  driver plus the native TCP/TLS code to fetch and decode a chunked response.
  The remote test server runs on the host; no host client stack performs the
  guest's TLS, certificate verification or HTTP processing. Guest exit: 33.
  Test entropy and temporary test certificates are confined to the probe.

The native probe is included in `build.sh`. This is mechanism proof, not a
System Generation install test. The installed ARM VirtualBox VM has an 82540EM,
while current native e1000 initialization uses x86 PCI ports and is gated to
x86_64. The ARM driver and production HTTPS service wiring are still absent.
Neither the current desktop nor weather fetching is claimed functional over
HTTPS. The installed trust-store/service packaging and parity check remain
release requirements, not satisfied by the probe's temporary root.
