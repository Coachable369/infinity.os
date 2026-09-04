# Milestone 3B conformance record

InfinityOS uses object identity as the native persistent primitive. Namespace paths are direct relationships to objects; there is no underlying file, inode, descriptor, POSIX link, or host-filesystem implementation.

## Implemented architecture

- Architecture-neutral 128-bit `ObjectId` and typed `ObjectRef` contracts.
- Explicit little-endian format-v2 encoding; no raw Rust-structure serialization.
- 4 KiB native allocation blocks, checked contiguous extents, a checksummed 61.5 MiB bitmap, and actual per-Space accounting.
- Copy-on-write immutable content, persistent parent-linked version manifests, restore-as-new-version behavior, and content CRC validation.
- Two alternating checksummed metadata banks and roots. The root switch is the durable commit boundary; mount chooses the highest fully valid generation and falls back safely.
- Persistently sorted object index, UTF-8 bounded namespace, multiple direct references, computed `/objects/<id>` view, identity-preserving move, separate detach/destruction, tombstones, and conservative explicit collection.
- Typed metadata (type, owner, Space, content type, flags, compact tags, size, created, modified) and persistent typed object relationships.
- Typed Object/Namespace/Relationship service requests and results behind an authorization policy boundary.
- Native System and Recovery bootstrap metadata objects. The Milestone 3A boot-region kernel remains the documented temporary firmware compatibility boundary.
- Console create/read/write/inspect/history/restore/remove/collect, namespace list/inspect/move/link, storage usage, and deterministic natural-language mappings.

## Verification commands

`./build.sh` builds bootable x86, x86_64 UEFI, and AArch64 UEFI ISOs in `builds/`.

`make object-test` verifies object identity, copy-on-write history, restoration, typed metadata/query, authorization denial, persistent relationships, multi-extent content, Space accounting, conservative GC, transaction rollback, multiple namespace references, identity-preserving moves, reboot reconstruction, unsupported-version rejection, and root/allocation/object/namespace/relationship/content corruption detection. It injects write failures after content, during metadata manifests, before root commit, and at two points during namespace updates.

`make milestone-3b-test` runs that conformance suite, provisions and boots a fresh x86_64 VM disk, then performs the mandatory installed-VM create/write/link/move/reboot/history/restore/reboot acceptance sequence.

`make test` boots x86, x86_64, and AArch64; exercises console/intent behavior on each; checks x86_64 graphical mouse selection and the AArch64 absolute-pointer bridge; and reruns installer safety checks.

## Deliberate bootstrap limits

- Format v3 has bounded tables: 32 objects, 32 versions, 32 namespace references, and 16 semantic relationships. It extends v2 only through formerly reserved bank sectors.
- One version uses one contiguous extent of at most 16 KiB. Multi-extent manifests and persistent tree indexes are later replacements.
- CRC-32/ISO-HDLC provides corruption detection. A modern content hash is reserved for future content addressing.
- Object IDs are UUID-shaped and deterministically mixed from the container seed, generation, and monotonic state. A platform entropy service must replace the generator.
- Case-sensitive UTF-8 lookup is an explicitly temporary policy; Unicode normalization is deferred.
- GC is explicit and conservative; named snapshot management is not exposed, though committed generation roots provide the snapshot/recovery foundation.
- x86_64 installed-disk persistence is VM-tested. AArch64 builds and boots with architecture-neutral object code, but installed-disk persistence is not yet automated there. Legacy x86 builds and boots, but does not mount the native store at runtime.
