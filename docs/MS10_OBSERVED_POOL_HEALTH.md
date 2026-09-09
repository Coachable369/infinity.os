# Observed shared Pool health

`pool health obj:<id>` submits the existing typed PoolInspect operation with a
nonzero shared ObjectId, value 1, and offset 0. It runs the same fresh owner and
repair quorum barriers and reader authorization as shared inspection. Results
use the ordinary owned asynchronous request/result channel. Unshared objects
are not supported by this targeted selector; existing Pool list remains intact.

The 64-byte summary carries ObjectId at 0, content hash at 16, desired replicas
at 48, verified unique nodes at 49, offline at 50, stale at 51, corrupt at 52,
under-protected at 53, staging at 54, and little-endian content length at 56.
Response headers carry the effective version, manifest generation, and authority.

Settings and this query use one read-only health derivation. A matching resource
explicitly offline or expired makes a verified placement observed-offline.
Unknown resources do not invent a failure. Wrong-version/hash placements are
stale; duplicate verified nodes do not inflate protection counts. Signed
ObjectInspect bytes remain unchanged. Events and GUI caches are not authority.

Host verification extends the existing shared projection test with actual
resource expiry, matching summary counts, and an unchanged signed placement.
Installed parity requires rebuilding and installing the new System Generation.
