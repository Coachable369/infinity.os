# Secure Connections

Plain transport, secure transport, peer identity, trust policy, authorization,
and connection resource state are distinct typed fields. Connection and
discovery records already reserve optional future NodeIdentity association.

A deterministic secure fixture verifies expected versus observed identity and
is **TESTED** for success/failure signaling. It is not TLS and makes no
cryptographic assurance. Production TLS, certificate chain validation,
revocation status, trust exceptions, secrets storage, and authenticated remote
peer identity are **UNSUPPORTED**. Silent downgrade is prohibited: asking for
secure transport cannot fall back to plain transport.
