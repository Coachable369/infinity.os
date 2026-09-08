# Node wire security boundary

Phase 9-A uses the existing protected identity and directional session services;
it does not invent a new cryptographic primitive or grant remote application
authority. Discovery never grants trust. Both signed identities, fresh agreement
values, endpoint context and explicit independent verification/consent are
required before session setup. See [wire trust](MILESTONE_9A_WIRE_TRUST.md) for
schema, bounds, replay behavior, zeroization, audit and evidence limits.

The engineering UART is a trusted local operator channel, not an untrusted remote
API or a substitute for final Trusted UI. Confirmed adapter state is boot-scoped;
production installed restart recovery and remote IOP authorization integration
must not be inferred from this phase. Existing trust revocation and capability
restrictions continue to apply. No secrets or sensitive payloads are audit data.
