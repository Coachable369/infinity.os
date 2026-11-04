# SSH transport foundation — acceptance ledger

The foundation is integrated into the native shared NIC reactor on ARM64 and
x86_64. Submilestone 2 adds the restricted Console described in
[ssh-submilestone-2.md](ssh-submilestone-2.md).

## Implemented and host-verified

- Bounded inbound TCP acceptance, capability revocation, timeouts and slot reuse.
- Default-on listener with persistent enable/disable configuration and local key administration.
- Pinned Sunset 0.6.0, native fallible entropy, Curve25519/Ed25519 and
  chacha20-poly1305 AEAD-only negotiation.
- Signed public-key authentication, identity revocation, versioned credential migration.
- Service-private persistent host keys; generic reads, aliases, history and mutation denied.
- Bounded buffers, deadlines, structured diagnostics and connection cleanup.

Host behavioral checks and native ARM64/x86_64 no_std compilation passed.
Real macOS OpenSSH transport/authentication, rejection, connection bounds,
malformed-client cleanup and host-key reboot persistence passed on a freshly
installed, media-detached ARM64 system. x86_64 installed acceptance is blocked by
installer completion. Current manifests, evidence and release state are recorded
in [ssh-submilestone-2.md](ssh-submilestone-2.md).

## Security boundary

The host key uses native service-private System Space metadata, the same access
boundary used for private identity state. This is not hardware-backed sealing or
protection against offline disk access. Missing entropy, invalid protected records,
unsupported algorithms and invalid credentials fail closed.

Listener ports are restricted to 1–49151; existing outbound actors reserve
49152–65535. Default port is 22 and default quota is two connections (maximum four).
TCP backlog slots expire after five seconds, unauthenticated transports after 30
seconds, and authenticated connections after five minutes. These are absolute
limits. No file transfer or forwarding is provided.

Sunset's source provenance and local patches are documented in
`third_party/sunset-0.6.0/INFINITY-PATCHES.md`.
