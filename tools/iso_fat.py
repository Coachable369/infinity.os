"""Read a contiguous ISO9660 EFI image directly with mtools, without extraction."""
import struct
from pathlib import Path


# ------------------------=
# FUNC: fat_image
# DESC: Validates bounded ISO root records and contiguous EFI extents before returning a read-only mtools offset reference.
# ------------------=
def fat_image(iso: Path) -> str:
    size = iso.stat().st_size
    with iso.open("rb") as source:
        root = None
        for sector in range(16, 80):
            source.seek(sector * 2048)
            descriptor = source.read(2048)
            if len(descriptor) != 2048 or descriptor[1:6] != b"CD001":
                raise ValueError("Invalid ISO volume descriptor")
            if descriptor[0] == 1:
                root = struct.unpack_from("<I", descriptor, 158)[0], struct.unpack_from("<I", descriptor, 166)[0]
                break
            if descriptor[0] == 255:
                break
        if root is None or not 0 < root[1] <= 1024 * 1024 or root[0] * 2048 + root[1] > size:
            raise ValueError("Invalid ISO root directory")
        source.seek(root[0] * 2048)
        directory = source.read(root[1])
        offset, start, end, continued = 0, None, None, False
        while offset < len(directory):
            length = directory[offset]
            if not length:
                offset = (offset // 2048 + 1) * 2048
                continue
            record = directory[offset:offset + length]
            if length < 34 or len(record) != length or 33 + record[32] > length:
                raise ValueError("Malformed ISO directory record")
            if record[33:33 + record[32]].upper().split(b";")[0] == b"EFI.IMG":
                at = struct.unpack_from("<I", record, 2)[0] * 2048
                count = struct.unpack_from("<I", record, 10)[0]
                if not count or at + count > size or record[25] & 2:
                    raise ValueError("Invalid EFI extent")
                if start is None:
                    start = at
                elif not continued or at != end:
                    raise ValueError("EFI extents are not contiguous")
                end = at + count
                continued = bool(record[25] & 128)
            offset += length
        if start is None or continued:
            raise ValueError("Incomplete EFI image")
        source.seek(start)
        boot = source.read(512)
        if len(boot) != 512 or boot[510:512] != b"\x55\xaa":
            raise ValueError("Invalid FAT boot sector")
        sector_bytes = struct.unpack_from("<H", boot, 11)[0]
        sectors = struct.unpack_from("<H", boot, 19)[0] or struct.unpack_from("<I", boot, 32)[0]
        if sector_bytes not in (512, 1024, 2048, 4096) or not sectors or sectors * sector_bytes > end - start:
            raise ValueError("FAT volume exceeds EFI extent")
    return f"{iso.resolve()}@@{start}"
