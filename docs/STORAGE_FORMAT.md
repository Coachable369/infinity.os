# InfinityOS Milestone 3A storage format

All integers are unsigned little-endian. Native records occupy one 512-byte
logical block, encode padding explicitly as zero, and store an IEEE CRC-32
(polynomial `0xedb88320`) at byte 508. CRC calculation covers all 512 bytes with
the checksum field zeroed. UUID fields are 16 bytes; this milestone derives
RFC-4122 variant/version bits deterministically from device geometry and a type
tag. A future entropy service will replace this prototype UUID source.

## Physical layout

| Region | LBA | Size/alignment |
|---|---:|---|
| Protective MBR | 0 | one block |
| Primary GPT header | 1 | one block |
| Primary GPT entries | 2-33 | 128 entries x 128 bytes |
| EFI System Partition | 2048-18431 | 8 MiB, 1 MiB aligned |
| Infinity Container | 18432 through last usable LBA | 1 MiB aligned |
| Backup GPT entries | disk end - 32 through disk end - 1 | 32 blocks |
| Backup GPT header | disk end | one block |

GPT uses a protective `0xee` MBR entry, revision 1.0 headers, primary and backup
entry arrays, header CRCs, entry-array CRCs, 128 entries, and the standard EFI
System Partition type GUID. The Infinity partition type is
`496e6669-6e69-7479-5354-4f5241474531` (on-disk bytes spell the format identity).

The EFI region is a firmware-readable FAT volume containing both
`EFI/BOOT/BOOTX64.EFI` and `EFI/InfinityOS/infinity.efi`. FAT is not InfinityFS.

## InfinityContainerHeader (`INFCONT1`, version 1)

| Offset | Size | Field |
|---:|---:|---|
| 0 | 8 | ASCII magic `INFCONT1` |
| 8 | 4 | format version, `1` |
| 12 | 4 | header bytes, `512` |
| 16 | 4 | install state: 1 provisioning, 2 installing, 3 verifying, 4 complete |
| 20 | 4 | logical block bytes, `512` |
| 24 | 8 | container total blocks |
| 32 | 8 | pool record relative LBA, `1` |
| 40 | 8 | Space table relative LBA, `2` |
| 48 | 8 | System kernel relative LBA, `2048` |
| 56 | 8 | exact System kernel byte length |
| 64 | 16 | container UUID |
| 80 | 16 | pool UUID |
| 96 | 412 | reserved zero |
| 508 | 4 | CRC-32 |

The System kernel begins 1 MiB into the container. This is an explicitly
temporary contiguous bootstrap representation; its location is discovered from
metadata and is not a loader constant.

## InfinityPoolHeader (`INFPOOL1`, version 1)

| Offset | Size | Field |
|---:|---:|---|
| 0 | 8 | ASCII magic `INFPOOL1` |
| 8 | 4 | format version, `1` |
| 12 | 4 | record bytes, `512` |
| 16 | 16 | pool UUID |
| 32 | 16 | member container UUID |
| 48 | 8 | total capacity blocks |
| 56 | 8 | first allocation cursor, currently System kernel LBA `2048` |
| 64 | 4 | member count, `1` |
| 68 | 440 | reserved zero |
| 508 | 4 | CRC-32 |

Separate pool and member UUIDs deliberately avoid encoding one-pool-equals-one-
disk as a permanent invariant.

## Space table (`INFSPACE`, version 1)

The table header stores magic at 0, version at 8, record bytes at 12, count `4`
at 16, and reserved zeros through 31. Four 96-byte entries begin at offsets 32,
128, 224, and 320. Bytes 416-507 are reserved; CRC-32 is at 508.

Each entry contains UUID (0, 16 bytes), zero-padded ASCII name (16, 16 bytes),
type (32, u32), policy (36, u32), optional minimum blocks (40, u64), optional
quota blocks (48, u64), state (56, u32), 4 reserved bytes, content relative LBA
(64, u64), content byte length (72, u64), and 16 reserved bytes. Type values 1-4
are System, Personal, Applications, and Recovery. Policy `1` means shared dynamic
capacity. State `1` means initialized. Minimum and quota are zero/unbounded. Only
the System entry has content in this milestone; it points to the installed kernel.
# Boot and native-object separation

Milestone 7 reserves the first 32 MiB of the Infinity Container for the
installed kernel and future boot-critical growth. Object-store metadata starts
at relative LBA 65536. Provisioning rejects a kernel that crosses this boundary
instead of allowing kernel and object writes to overlap silently.
