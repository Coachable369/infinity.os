# Native Cryptography Boundary

Milestone 9 uses Ed25519 signatures, X25519 ephemeral agreement, HKDF-SHA256 key derivation, ChaCha20-Poly1305 authenticated encryption, SHA-256 identifiers, and explicit key zeroization behind a replaceable `NodeCrypto` boundary.

Status: deterministic behavioral round trips are **TESTED**. Hardware roots of trust, remote attestation, production cryptographic audit, and side-channel certification are **UNSUPPORTED**.
