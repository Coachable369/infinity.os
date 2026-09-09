# Pool metadata quorum foundation

Status: IMPLEMENTED AND HOST TESTED primitives only. NOT integrated with installed
Pool reads. This does not close the owner-offline acceptance gap.

The fixed group has three explicitly configured NodeIds and public keys, one
writer, and a membership epoch. Metadata records bind the ObjectId, generation,
previous digest, version, manifest digest, namespace digest, policy digest,
revocation generation and tombstone. Records are canonical and owner-signed.

Two distinct signed durable staging receipts form a certificate. A writer may
acknowledge publication only after two distinct durable publication receipts.
A reader observes two members through fresh authenticated, correlated operations
and writes the highest valid certified generation back to two members before
exposing it. No member count is inferred from discovery. A single reachable
member cannot establish freshness. There is no owner election or writer promotion.

Read delegation is separately owner-signed and bound to the group, object,
reader node, authenticated local principal, policy generation and expiry.
Possessing replica bytes or a metadata certificate grants no application access.
Deleted objects and superseded grants fail closed.

The primitives use fixed records and bounded state. Native backing persists an
8992-byte, eight-object catalog of signed staged/committed records and fixed group
configuration through ObjectStore transactions. Persistence commits before state
advances or a receipt is issued. Recovery validates canonical bytes and signatures;
capacity exhaustion returns `ResourceLimit`. Six focused behavioral tests cover
quorum rules, group binding, grants, canonical recovery and sector-cut durability.
Referenced manifest/namespace/policy payloads and installed protocol use are not
integrated by this backing module.

## Required before installed use

- Connect the native staging/certificate backing to immutable referenced metadata,
  atomic application publication and bounded reclamation.
- Explicit operator-approved group configuration and read delegations, persisted
  as native objects; no implicit grants during pairing or discovery.
- Registered typed IOP operations over existing bounded secure transport, with
  live capability checks and request/session/correlation replay fencing.
- Ordinary namespace and ObjectRead integration, distinct from physical replica
  transfer operations. Local execution-context authentication remains mandatory.
- Commit ordering with existing object updates, policy changes and deletion;
  failure/partition/reboot tests before acknowledging writes or returning reads.
- Installed A-offline/B+C reads preserving path, ObjectId, version and integrity,
  plus denial when freshness or read authority cannot be established.

Until these are implemented and verified, owner-offline normal reads remain
UNSUPPORTED. The existing owner-mediated verified remote-read path is separate.
