# Node Identity

InfinityOS derives a stable `NodeId` from an Ed25519 public identity. Firmware entropy is mandatory; zero or unavailable entropy fails closed. Callers receive an opaque `KeyRef`, never private key bytes. The public API is independent of network address, machine name, storage location, and human identity.

Status: identity creation, stable derivation, signing, verification, persistence round-trip, and corruption rejection are **TESTED**. The persisted identity seed is integrity checked but only software protected; hardware-backed sealing is **PLANNED**.
