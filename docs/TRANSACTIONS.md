# Infinity Object Store transactions v2

All metadata mutations enter a non-reentrant transaction boundary. Content and a complete new metadata bank are written first, then flushed. Only after validation does InfinityOS write and flush the alternate 512-byte generation root. That final root write is the durable commit boundary.

```text
allocate new extent -> write immutable content -> write inactive metadata bank
-> flush -> write alternate checksummed root -> flush -> committed
```

Roots occupy object-store-relative sectors 0 and 1. Metadata banks begin at relative sectors 8 and 40. Generations alternate roots and banks. Boot validates both root CRCs and referenced bank CRCs, chooses the highest fully valid generation, and falls back to the previous valid generation when the newest root or bank is corrupt. It never formats automatically after a mount failure.

Copy-on-write updates allocate a fresh content extent and append a VersionRecord. Restore reads and validates the selected historical version, then creates a new current version; it never rewinds or deletes newer history. Namespace changes commit through the same generation mechanism.

Crash tests simulate orphan writes before root commit and corruption of the newest root. Recovery returns either the prior committed generation or the complete new generation. Content CRC failures return `CorruptContent`; metadata-sector failures return `CorruptMetadata`.

V2 validates roots, allocation sectors, sorted object manifests, version extents, namespace targets, relationship targets, and every content read. Its deterministic explicit collector reclaims only tombstoned objects with no namespace or relationship references; there is no aggressive asynchronous GC. The metadata banks remain bounded snapshots, content is limited to one contiguous 16 KiB extent per version, and the bitmap covers 61.5 MiB. Future persistent trees and multi-extent manifests can replace these internals without changing `ObjectRef` or typed operation contracts.
