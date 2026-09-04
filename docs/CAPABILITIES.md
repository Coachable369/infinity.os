# Capability Security

Authority is explicit and deny-by-default. A capability stores its reference, type, numeric target, rights mask, constraints, issuer, 128-bit holder, optional monotonic expiry, delegation rights, parent, and revocation state.

Validation checks holder, type, target, rights, constraints, lease, direct revocation, and ancestor revocation at every use. Delegation can only remove rights, add/narrow constraints, and shorten a lease. Revoking a parent immediately invalidates descendants without restarting a context.

TESTED: grant, constrained subset delegation, denial outside project scope, live revocation, ancestor revocation, and lease expiry.

The live Installer Context receives only leased Storage.Discover, Storage.Provision, Boot.Install, and System.Install authority plus audit publishing. Explicit destructive-popup confirmation emits a Record; completion emits another and revokes all installer authority. Installed profiles do not start the installer.

Milestone 6 adds distinct `AiInfer`, `ModelUse`, `ContextRead`, `ToolInvoke`,
`AudioInput`, `AudioOutput`, `AgentControl`, `RemoteAi`, and narrow inspection
capability types. Context views must be a subset of `ContextRead`; Tool Broker
operations require their mapped service capability; push-to-talk requires a
live `AudioInput` lease. Remote-provider authority is separate from local model
use. These boundaries and live revocation are HOST-TESTED.

Capability authenticity currently uses the trusted in-memory manager. Cryptographic tokens at an MMU boundary are SCAFFOLDED. There is no permanent unrestricted root equivalent.
# Session-scoped identity authority

Milestone 7 adds explicit self-profile, Personal Space, Settings, AI, Voice,
display/input, session-management, and identity-management rights. Logout
clears the complete session set. A secondary user cannot update another user or
inspect another Personal Space merely because both are local system users.
