# Infinity Object Store format v3

Milestone 6.5 extends stable base types with Collection, Project, Model,
IdentityData, and DeviceData and persistent relationship IDs with ContainedBy,
RelatedTo, VersionOf, OwnedBy, SharedWith, Favorite, and Pinned. Semantic
subtypes remain above the Object Store. `INFOORG1` is the checksummed bootstrap
taxonomy System object.

InfinityOS persists objects, not files. A namespace path is a relationship to a 128-bit `ObjectId`; paths, sectors, and extents never participate in identity. No POSIX file primitive, inode, descriptor, or symbolic link backs this format. All integer fields use explicit little-endian encoding, all offsets below are byte offsets, and no Rust memory layout is serialized.

The store begins at container-relative LBA 49,152. The Milestone 3A boot image remains at relative LBA 2,048 as a documented, read-only compatibility boundary until firmware can consume System Space objects.

## Regions and commit roots

| Relative sector | Structure | Size | Integrity |
|---:|---|---:|---|
| 0, 1 | alternating generation roots | 512 bytes each | CRC-32/ISO-HDLC at 508 |
| 8..39, 40..71 | alternating metadata banks | 32 sectors each | CRC-32 per sector |
| 80 onward | immutable content extents | 4 KiB allocation blocks | CRC in VersionRecord |

Every metadata sector has an 8-byte magic at 0, format `u32` at 8, reserved bytes that readers ignore, and CRC at 508 computed with the CRC field zero. Unsupported versions return `UnsupportedFormat`; invalid CRCs return `CorruptMetadata`.

`INFOROOT`: generation `u64` at 16 and bank-relative LBA `u64` at 24. Only bank 8 or 40 is accepted.

Each bank contains:

| Bank offset | Magic | Payload |
|---:|---|---|
| 0 | `INFOSTAT` | generation `u64@16`, next-ID state `u64@24`, capacity blocks `u32@32`, bitmap bytes `u32@36` |
| 1..4 | `INFOALC2` | 492 allocation-bitmap bytes per sector from byte 16 |
| 5..10 | `INFOOBJ2` | four 120-byte ObjectRecords per sector |
| 11..14 | `INFOVER2` | eight 60-byte VersionRecords per sector |
| 15..22 | `INFONSP2` | four 120-byte NamespaceRecords per sector |
| 23..24 | `INFOREL2` | eight 60-byte RelationshipRecords per sector |
| 25..26 | `INFOOBJ2` | two v3 expansion sectors, four ObjectRecords each |
| 27..31 | reserved | zero/ignored for future compatible metadata indexes |

## ObjectRecord (120 bytes)

| Offset | Width | Field |
|---:|---:|---|
| 0 | 1 | used |
| 1 | 1 | tombstone |
| 2 | 1 | extensible object type |
| 3 | 1 | Space |
| 4 | 16 | ObjectId |
| 20 | 4 | current version |
| 24 | 8 | creation generation |
| 32 | 8 | modification generation |
| 40 | 1 | display-name UTF-8 length |
| 41 | 47 | display-name bytes |
| 88 | 16 | owner ObjectId; zero is the temporary bootstrap identity |
| 104 | 4 | flags/policy extension bits |
| 108 | 2 | typed content type |
| 110 | 1 | tag bytes length |
| 111 | 8 | compact UTF-8 bootstrap tags |
| 119 | 1 | reserved |

The object table is persistently sorted by ObjectId, enabling binary lookup. Type values are Blob, Text, NamespaceNode, SystemComponent, ApplicationData, and Metadata. Space values are System, Personal, Applications, and Recovery. Unknown active values are corruption, while a future incompatible layout requires a new format version.

## VersionRecord (60 bytes)

Used is at 0; ObjectId at 4; version at 20; extent start at 24; block count at 28; logical size at 32; content CRC at 36; parent version at 40; creation generation at 44. Bytes 52..59 are reserved. A content manifest currently supports one contiguous extent of up to four 4 KiB blocks (16 KiB). The extent format deliberately admits future extent lists, content hashes, compression, encryption, sparse content, and remote references without making location part of identity.

## RelationshipRecord (60 bytes)

Used is at 0; relationship type at 2; source ObjectId at 4; target ObjectId at 20; metadata flags at 36; creation generation at 40. Bytes 48..59 are reserved. Initial types are member-of, derived-from, references, generated-by, and belongs-to-project. Namespace relationships remain separately indexed because they carry bounded UTF-8 paths.

## Allocation and accounting

The allocator exposes 4 KiB logical allocation blocks over eight 512-byte transfer sectors. Its 1,968-byte checksummed bitmap addresses 15,744 blocks (61.5 MiB), uses deterministic checked first-fit contiguous extents, and is reconstructed from disk on every mount. Allocation requests carry a Space; usage is calculated from each retained VersionRecord and its owning object's Space. Spaces share one pool and are not fixed partitions. Policy hooks are reserved in object flags and Space-aware allocation APIs.

The object and namespace tables remain bounded bootstrap indexes (32 objects,
32 versions, 32 namespace references, 16 relationships). Format v3 consumes
two sectors that were reserved in each v2 bank, allowing native AI System
objects without sacrificing the user-object acceptance budget. These are
implementation limits, not public API semantics. The 16 KiB content ceiling,
CRC-32 rather than a cryptographic content hash, and deterministic UUID-shaped
generator are temporary structures to replace with persistent trees,
multi-extent manifests, a platform entropy provider, and a modern content hash
in later storage milestones.
