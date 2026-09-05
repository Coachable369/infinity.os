# Trusted UI

Authentication, lock, capability consent, destructive confirmation, and
recovery are reserved trusted surfaces. A `SecureInputLease` is exclusive,
bounded by monotonic expiry, and tied to one execution context. Only the active
lease can derive the opaque `TrustedWindowToken` accepted by the Window Server.

Application windows cannot request Trusted or Cursor z-order through the normal
creation path. The compositor repeats that check against the surface security
class, providing a second policy boundary. While secure input is active, pointer
and keyboard routing bypasses ordinary windows entirely.

## Status

- Exclusive lease, expiry, token derivation, input precedence, and trusted
  z-order denial: **TESTED**.
- Hardware-backed secure attention key: **PLANNED**.
