# Node Identity

Phase 9-A authenticates both long-term identities over a canonical signed wire
transcript with fresh ephemeral values and endpoint binding. Engineering guests
generate independent identities from firmware entropy; none are host-injected.
See [wire trust](MILESTONE_9A_WIRE_TRUST.md) for the exact verification boundary.

InfinityOS derives a stable `NodeId` from an Ed25519 public identity. Firmware entropy is mandatory; zero or unavailable entropy fails closed. Callers receive an opaque `KeyRef`, never private key bytes. The public API is independent of network address, machine name, storage location, and human identity.

Status: identity creation, stable derivation, signing, verification, persistence round-trip, and corruption rejection are **TESTED**. The persisted identity seed is integrity checked but only software protected; hardware-backed sealing is **PLANNED**.
