# Native Network Policy

Policy subjects are stable System, ExecutionContext, ServiceIdentity,
ApplicationIdentity, or Session identities. Executable paths are not identity.
Rules include direction, interface, local/remote address and prefix, port,
protocol, action, priority, logging mode, lease, and policy source.

Resolution is deterministic: enabled, unexpired, matching rules are ordered by
priority and then stable rule ID. No match uses an explicit fallback action;
the bootstrap fallback is deny. Allow, Deny, Ask, RateLimit, and AuditOnly are
typed actions. Ask is a denial until Trusted UI commits authority.

`Network.Connect`, `Network.Send`, and `Network.Receive` validate capability at
use. Policy is evaluated at connection creation. Revocation and lease expiry
therefore affect a running holder without restart. Per-application differing
allow/deny scopes, revocation, and lease expiry are **TESTED**. Persistent
policy object encoding and Trusted UI grant presentation are **SCAFFOLDED**.
