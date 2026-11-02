# Native Identity

Account-management verification is recorded separately from the historical
first-boot identity checks. A host test or an ISO build is not installed-runtime
acceptance.

InfinityOS represents machines and people with stable 128-bit typed identities.
Names, handles, system generations, namespace projections, and physical storage
are mutable attributes and never identity. `MachineIdentity`, `UserIdentity`,
`UserProfile`, `AIProfile`, `VoiceProfile`, and `PersonalSpaceOwnership` are
architecture-neutral native records in the versioned `/system/identity/state`
object.

Settings > Users & Accounts and Console use the same authenticated account
service. It supports user and group creation, inspection, renaming and deletion,
user enable/disable, password replacement, and adding/removing memberships.
The bounded native directory currently holds eight users and sixteen groups,
including the protected `administrators` group. Capacity exhaustion is an error,
not a silently truncated directory.

Creating a user atomically commits its password verifier, profiles, unique
Personal Space ownership and personal folders. Failed storage writes restore
identity state, sessions, rate limits and allocation counters. Same-name or
retained personal folders are never adopted by another identity; a conflicting
new directory receives an immutable-ID suffix.

An ambiguous root-write/final-flush failure is not reported as a successful
rollback. Storage refuses further writes and account authentication is suspended
until restart, when native mount selects a validated committed generation. Generic
object operations cannot read, overwrite, rename, copy or remove identity state;
only the trusted identity persistence adapter can access its credential contents.

Disable preserves the account and memberships but closes its sessions. Delete
removes the account, credentials, memberships and saved desktop profile, and
closes its sessions. Personal files remain owned by the old immutable identity
for deliberate recovery; deletion is not a file-erasure command. Recreating a
handle does not inherit those files or permissions.

Ordinary active users can inspect the directory and edit their own name, handle
and password. Administrators manage other users, groups and memberships.
The active user cannot delete or disable themselves, and the last active
administrator cannot be removed from the protected group. Membership changes
immediately update existing session authority. Custom groups do not implicitly
grant arbitrary filesystem, network or application capabilities.

Passwords use private masked entry and confirmation, never ordinary command
arguments or history. Resetting another user's password revokes their sessions.
Changing one's own password preserves the initiating authenticated session and
revokes other sessions.

## Persistence

Identity format 4 appends explicit groups and membership to the checksummed
architecture-neutral record. Formats 1, 2 and 3 remain readable; migration gives
the previous first-active-user administrator explicit membership without changing
user IDs, passwords, preferences or personal ownership. Transient sessions are
not restored at reboot. Both live and installed kernels use the same decoder and
account service; no separate live-only account database is introduced.
