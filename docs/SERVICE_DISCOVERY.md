# Local Service Discovery

The native discovery manager stores bounded typed advertisements containing
service identity/type, endpoint, metadata schema/hash, health, lease expiry,
and optional future NodeIdentity. Advertisements never confer call authority.

Lease expiry, bounded-cache saturation, health state, and Offline-profile
disablement are **TESTED**. Local wire advertisement/discovery protocols are
**UNSUPPORTED** until a physical adapter exists. Milestone 9 can add peer trust
without changing the record or connection identity contract.
