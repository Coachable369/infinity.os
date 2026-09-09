# Native shared metadata runtime

`PoolMetadata` (0xe050) uses the existing authenticated IOP envelope and exact
peer-issued operation grants. The bounded runtime publishes the actual signed
manifest, namespace and reader policy, not a pointer to owner-local metadata.
Three explicitly configured members use R2 observations and W2 writeback.
One request/window progresses per pump; no UI callback waits for networking.

Console admission:

- `pool metadata-authority peer=node:<id> grant=<exact handle> lease=3600 confirm=true`
- `pool share obj:<id> path=/Shared/Example confirm=true`
- `pool read /Shared/Example offset=0 length=64`
- `pool result request=<returned id>`

Sharing explicitly delegates the native storage service on the configured
members. This is a **service principal**, not a grant to every local application
or user. The current human broker additionally requires the exact active
privileged user/session. Requests and results are owned by that session.

The ordinary asynchronous application broker is `storage_client::submit`;
shared reads select sources internally. A locally cached manifest or a historical
certificate is never sufficient to complete the fresh read barrier. Local
recipient bytes are checked against the owner-bound immutable content hashes.
Namespace hints refresh one bounded catalog slot per second and are not an
authorization cache.

Owner updates, policy changes and tombstones use a fresh barrier before mutation
and acknowledge only after successor publication. The native write-ahead intent
retains the exact request, signed base, selected repair digest and applied result.
Cold recovery repeats the quorum barrier; finalization retains an exact receipt
so a lost acknowledgement is idempotent. Fresh repair-overlay convergence precedes
owner mutation; certified placement restoration preserves replacement replicas
without allowing a generic generation jump. Ordinary synchronous shared mutation
bypasses remain rejected. A copy creates a separate local identity and does not
publish a new record on the source's chain.

The automatic repair trigger has a separate service-only read barrier. Admission
uses the actual installed storage service and requires an owner-signed live repair
grant naming that local node before quorum processing and at result consumption.
It does not manufacture an interactive user session. Native actions remain subject
to exact peer grants. Each operation has a bounded deadline and a single result
mailbox; inactive or expired authority fails closed.

Current verification: focused runtime cadence/admission test and signed quorum
state-machine tests pass on the host. This document is not installed owner-loss,
autonomous repair, or milestone-completion evidence. Those require the installed
multi-node acceptance run and the separately authorized repair overlay path.
