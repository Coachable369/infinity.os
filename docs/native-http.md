# Native HTTP transport dependencies

## Implemented foundation

`kernel/runtime/http` is a `no_std`, no-allocator library, shared by the
kernel's live and installed build configurations. It uses smoltcp 0.12.0 with
Ethernet, IPv4, TCP and DNS enabled, plus httparse for bounded HTTP headers.
Host sockets, libc, general async frameworks and default logging are disabled.
Upstream: [smoltcp](https://github.com/smoltcp-rs/smoltcp).
Upstream TCP handles checksums, retransmission, ordering and flow control.

The adapter exposes one outbound stream backed by caller-owned RX/TX buffers
and one socket slot. Each poll handles at most four incoming packets and a
bounded egress pass. Connect is asynchronous, sends report partial capacity,
reads distinguish backpressure from EOF, and cancellation makes subsequent IO
fail. A hard transaction deadline also bounds a peer that keeps sending data.
Callers supply fresh entropy for sequence randomization and monotonic time.

An optional second caller-owned socket slot and one query slot enable wire DNS
through `Transport::enable_dns`. `resolve` starts a nonblocking A query;
`resolved_address` consumes its result. DNS uses the same interface, routes and
ARP cache as TCP, not a separate packet owner. Queries have hard deadlines,
cancellation releases their slots, and late results cannot be consumed after
timeout. There is no DNS cache in this adapter, so no invented TTL or stale
address reuse. The configured DNS server must be authorized before enabling it;
the resolved application destination must separately pass policy before connect.
This API is not yet wired to the desktop NetworkRuntime resolver.

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
- Real ARP/DNS packet exchange, wrong-server and transaction-ID rejection,
  one-shot A results, timeout, cancellation and query-slot reuse.

After the DNS transport addition, all ten transport/HTTPS tests pass and the
bare-metal native HTTPS probe still exits with success (33). The ARM64 installed
kernel also builds. DNS has packet-level behavioral coverage, but is not yet
exercised by an installed service or by the HTTPS bare-metal probe, which still
uses a fixed destination for its controlled TLS server.

The original six tests pass. The ARM64 installed kernel links successfully, and x86_64
kernel checks pass with and without the installer feature. Plain `cargo check`
initially failed because this host does not have prebuilt bare-metal `core`;
the repository's `-Z build-std=core` build path succeeds. Existing kernel
warnings remain. These checks are not installed-network acceptance.

## Original integration boundary (superseded by actor section below)

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

Support is AES-128-GCM/SHA-256 with ECDSA P256/SHA256 or P384/SHA384
certificates, plus RSA 2048-4096 bit ordinary rsaEncryption keys. RSA certificate
signatures support PKCS1 v1.5 and PSS with SHA256/384/512; TLS CertificateVerify
accepts RSA-PSS-RSAE only. PSS requires the exact digest-length salt and matching
MGF1 hash. Ed25519, SHA1 and restricted RSA-PSS key types fail closed. This
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

The native probes are included in `build.sh`. These are mechanism proofs, not
System Generation install tests. `make native-https-arm-test` boots the production
ARM UEFI loader and runs the same authenticated request through the ARM e1000
driver. It passes with guest exit 0 and the exact expected decrypted body; the
x86_64 regression probe passes with guest exit 33.

## ARM PCI network handoff

The loader retains the supported 82540EM's ACPI/MCFG ECAM function address in
`firmware_network`, discriminated by `network_reserved = 4`. BootInfo remains
version 8 and 264 bytes; older descriptor kinds are not interpreted as pointers.
On ARM the driver validates the PCI identity and memory BAR, enables bus mastering,
then uses the same MMIO/DMA ring implementation as x86. The ARM generic counter
supplies monotonic deadlines instead of x86 HPET. The production network pump
now also builds and runs on ARM, retaining its bounded RX/TX budgets.

This initial backend targets coherent, identity-mapped ARM virtual machines.
Noncoherent physical hardware, IOMMU/DMA translation and other NIC models are
not supported by this change. The installed kernel and shared UEFI loader both
build with the implementation; an existing installed VM needs **both** updated
loader and kernel, not a kernel-only patch, to obtain the new descriptor.

Production HTTPS service wiring is still absent. Neither the current desktop
nor weather fetching is claimed functional over HTTPS. Installed-system packet
verification, trust-store/service packaging and parity remain release
requirements, not satisfied by the probe's temporary root.

## Cooperative request orchestration

`client::get` now owns the DNS, TCP connection and TLS request lifecycle in one
allocation-free future. It accepts a NIC-queue `Link` adapter, a hard monotonic
deadline, genuine cryptographic RNG, trusted wall time and a trust store. DNS
authorization and resolved-destination authorization are separate. Every packet
pump rechecks authority before ingress/egress; revocation or deadline expiry
aborts the request without transmitting queued packets. Residual DNS frames are
discarded before switching to application-endpoint authority.

Pending work registers a network/timer wake with the adapter instead of
self-waking continuously. Each pump is limited to four frames per direction.
The owner must poll outside paint/input handlers and provide timer wakes even
when no packets arrive, so deadlines remain enforceable. This does not make
cryptographic computations preemptible within one future poll.

The x86 and ARM probes now use this shared client rather than implementing their
own connection/pumping loops. Three additional behavioral tests cover initial
denial, mid-request revocation and deadline expiry for both DNS and direct-IP
requests, including suppression of previously queued frames.

This is still not the installed application service. The production adapter
must bind `Link::allowed` to the requesting identity's capabilities and current
policy/configuration, demultiplex NIC queues without disrupting existing UDP
users, and register the task with the OS scheduler. Probe adapters restrict
traffic to their controlled test server; they are not production authorization.

## Production actor and RSA integration — September 13

The production `kernel/drivers/https.rs` actor now runs cooperatively from the
native network pump on the bootstrap processor, **after** the existing Runtime
borrow ends. Runtime is not safe for concurrent mutable access; dispatching the
actor onto a second CPU would be unsound. Fixed pinned future storage is reused
without a heap, with four retained Ethernet frames per direction. Existing UDP
ingress remains connected. The timer polls pending work even without RX traffic.

Requests are bound to an active authenticated session and that session's
NetworkConnect, NetworkSend, NetworkReceive and NetworkResolve capabilities.
Current inbound/outbound policy, DNS, source address/prefix/MAC, route, link and
profile generation are rechecked before traffic submission. Revocation,
configuration changes and session locking discard queued traffic. Another
session cannot collect or cancel the request. No permissions are auto-granted.
An egress filter confines automatic stack output to the authorized DNS/TCP
endpoint and its next-hop ARP; policy is checked against the actual UDP source
port. On-link DNS does not replace the interface's default Internet gateway.

The installed and live Console expose `https get HOST /PATH CONNECT_CAP SEND_CAP
RECEIVE_CAP RESOLVE_CAP`, `https result` and `https cancel`. These are diagnostic
entry points for an already-authorized session, not a graphical permission flow.
The subsequent [geturl command adapter](geturl.md) adds automatic selection of
existing session capabilities and an explicit temporary operator-consent path
through `https authorize`; it does not auto-grant rights on a transfer request.
The body is returned only after authenticated completion, up to 8192 bytes.
There is one request slot. Boot entropy is domain-separated into a dedicated
ChaCha20 generator; certificate time comes from EFI GetTime with validated
Gregorian fields and timezone. Unspecified EFI timezone follows the existing
UTC RTC convention. Missing entropy or time fails closed, with no fake fallback.

RSA uses allocation-free RustCrypto crypto-bigint public exponentiation and
pkcs1 DER parsing. The local verifier implements the strict RFC 8017 padding
checks from [RFC 8017](https://www.rfc-editor.org/rfc/rfc8017.html); it is new cryptographic integration, not an independently audited
cryptographic module. Tests verify all six SHA2/padding combinations against
OpenSSL at three key sizes, reject altered messages/signatures and bad lengths,
trailing key data, out-of-range representatives and nonstandard PSS salt lengths.
`make native-https-rsa-test` exercises both bare-metal CPU architectures.

`make https-service-test` uses the actual Runtime and actor to verify traffic
submission, owner isolation, capability revocation, DNS changes, cancellation
and session locking. Its NIC is a packet-counting fixture; it is not an
installed end-to-end HTTPS test. All tests are included in build.sh.

Remaining acceptance: ISO-detached installed request proof, application/IOP
and weather client integration, and a user-facing scoped permission workflow.
The new actor and Mozilla root store are linked into the installed kernel,
but a successful installed-kernel build alone does not prove installation or
live networking. Individual crypto operations still run within one cooperative
poll and have no preemptive latency guarantee. TLS1.2, IPv6, proxies, HTTP2,
redirects, compression, mutual TLS and online CRL/OCSP remain unsupported.

Verification of this revision: 18 HTTP library tests, actor lifecycle checks,
both architecture RSA probes and both installed-kernel links pass. Regenerating
the build images corrected a stale ARM loader; the artifact parity test now
verifies byte-identical installed kernels inside both installers and matching
bootloaders in their ESP/build images. This remains artifact parity, not a
fresh installation followed by an ISO-detached HTTPS request.
