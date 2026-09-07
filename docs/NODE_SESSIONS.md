# Secure Node Sessions

Sessions use ephemeral X25519 agreement, HKDF-SHA256 transcript binding, and ChaCha20-Poly1305 authenticated encryption. Frames use unique session/sequence nonces. Replays, expiry, untrusted peers, and failed authentication are rejected.

Status: agreement, authenticated round trip, tamper/replay rejection, expiry, revocation, and zeroization are **TESTED**. Network-carried mutual authentication and rekey negotiation are **SCAFFOLDED**.
