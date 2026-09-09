# Namespace-independent quorum persistence

Pool quorum state and per-object signed metadata payloads use bounded, internal
System/Metadata records named `@pool-quorum-replicas` and
`@pool-quorum/<ObjectId>`. Explicit metadata peer configuration similarly uses
`@pool-metadata-access`. These records do not consume user namespace entries.
Existing named records remain readable and retain their original identities.
Conflicting legacy/reserved identities and duplicate reserved records fail closed.

Quorum catalog and payload publication remains a single native-root transaction.
Read-only lookup never initializes missing state. Shared-object mutation fencing
uses this same reserved/legacy resolver; the internal fresh-quorum permit is still
required. Object count and content-size limits have not increased.

Host verification exercises configuration cold roundtrip, Stage and Publish with all 32 namespace entries used,
every sector-write interruption, cold recovery, and unchanged namespace occupancy.
Existing guarded mutation and returned-owner deletion tests remain enabled, with
the deletion fixture explicitly filling the namespace rather than relying on
incidental internal named-record allocation. These host results do not substitute
for the subsequent fresh-ISO installed Share and owner-offline acceptance pass.
