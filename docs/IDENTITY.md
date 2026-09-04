# Native Identity

Status: **TESTED** on the host harness and through first-boot identity creation
from the independently booted installed-disk GUI.

InfinityOS represents machines and people with stable 128-bit typed identities.
Names, handles, system generations, namespace projections, and physical storage
are mutable attributes and never identity. `MachineIdentity`, `UserIdentity`,
`UserProfile`, `AIProfile`, `VoiceProfile`, and `PersonalSpaceOwnership` are
architecture-neutral native records in the versioned `/system/identity/state`
object.

Creating a user atomically creates its profile, AI/voice policy, and unique
Personal Space ownership. Deactivation preserves audit identity and ownership;
it is not Unix account deletion and no UID/GID is introduced.

The initial active user receives constrained identity-management authority in
its authenticated session. Later users receive ordinary self-scoped authority.
Cross-user profile and Personal Space operations are denied without explicit
management authority.
