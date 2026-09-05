# InfinityOS System Manifest format

Records are little-endian, independently versioned, zero-filled in reserved fields, and protected by an IEEE CRC-32 in their final four bytes. Boot Catalog and InfinitySystemManifest records are 512 bytes. The component table is a bounded multi-sector record. Readers reject unknown required versions, out-of-range counts/references, architecture mismatch, invalid lifecycle state, or checksum mismatch. Structures are encoded field-by-field; Rust/C struct memory is never persisted.

## Boot Catalog `INFBOOT1`, version 1

| Offset | Width | Meaning |
|---:|---:|---|
| 0 | 8 | magic |
| 8 | 4 | format version |
| 12 | 4 | record bytes (512) |
| 16 | 4 | active flag |
| 20 | 4 | architecture |
| 24 | 8 | active generation ID |
| 32 | 8 | manifest relative LBA |
| 40 | 8 | previous generation hook |
| 48 | 8 | recovery generation hook |
| 56 | 4 | bounded generation count |
| 64 | 16 | system/container UUID |

## InfinitySystemManifest `INFSYSM1`, version 1

Offsets 16/20 are lifecycle state and architecture; 24 is the 64-bit generation ID; 32 is build ID; 40/48/56 are kernel relative LBA, byte length, and CRC-32; 60/64 are bounded component count and component-table relative LBA; 72 is required runtime version; 80..95 is system UUID; 96 is a deterministic created timestamp (zero means unavailable). Remaining bytes are reserved for compatible extension.

## Component table `INFCOMP1`, version 2

Version 2 occupies two contiguous 512-byte sectors and protects bytes 0..1023
with its CRC-32 at bytes 1020..1023. The header stores total record bytes,
component count, 48-byte entry size, and sector count. This removes the former
single-sector capacity limit while retaining fixed bounds. Each entry contains 32-bit
component ID, type, version, architecture (`0` means neutral), install class,
required flags, content checksum, reference kind, then 64-bit content reference
and byte length. Version 2 installs eleven required CORE declarations: kernel,
EFI/bootstrap, required drivers, runtime, console, Intent Runtime, storage
runtime, recovery metadata, the native AI/local-ML/voice/agent suite,
InfinityUI/identity/session services, and the native networking suite.
Statically linked components deliberately share the verified installed-kernel
extent; this declares composition without pretending they are separate binaries
or files.

Install classes are `CORE (1)`, `SYSTEM OPTIONAL (2)`, and `POST-INSTALL (3)`. Only CORE is selected now. The central `SYSTEM_COMPONENT_REGISTRY` is the single future registration point.
