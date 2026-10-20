"""Binary ELF load-range checks for supported native firmware memory maps."""
import struct


# ------------------------=
# FUNC: validate
# DESC: Validates native load segments against firmware maps and x86 relative-addressing limits.
# ------------------=
def validate(header, platform="virtualbox"):
    if platform not in ("virtualbox", "qemu"):
        raise ValueError("unsupported kernel memory map")
    if len(header) < 64 or header[:6] != b"\x7fELF\x02\x01":
        raise ValueError("invalid ELF64 header")
    machine = struct.unpack_from("<H", header, 18)[0]
    offset = struct.unpack_from("<Q", header, 32)[0]
    stride, count = struct.unpack_from("<HH", header, 54)
    if stride != 56 or count > 64 or offset + stride * count > len(header):
        raise ValueError("invalid program header table")
    ranges = []
    for index in range(count):
        kind, flags, file_offset, virtual, physical, filesz, memsz, align = struct.unpack_from(
            "<IIQQQQQQ", header, offset + index * stride)
        if kind != 1 or not memsz:
            continue
        end = physical + memsz
        if filesz > memsz or virtual != physical or end > 2**64:
            raise ValueError("invalid load range")
        if machine == 183:
            if platform == "virtualbox" and physical < 0x100000000 and end > 0xE0000000:
                raise ValueError("ARM kernel load range overlaps VirtualBox device aperture")
            if platform == "qemu" and not 0x40000000 <= physical < end <= 0x340000000:
                raise ValueError("ARM kernel load range is outside the 12 GiB QEMU guest RAM")
        if machine == 62 and end > 0x80000000:
            if (physical, memsz) not in ((0x100000000, 0x60000000),
                                         (0x160000000, 0x20000000)):
                raise ValueError("x86 load range exceeds relative-addressing range")
            if filesz != 0 or flags != 6:
                raise ValueError("x86 high arena must be zero-initialized non-executable memory")
        ranges.append((physical, end))
    if not ranges:
        raise ValueError("no kernel load ranges")
    ordered = sorted(ranges)
    if any(left[1] > right[0] for left, right in zip(ordered, ordered[1:])):
        raise ValueError("overlapping kernel load ranges")
    if platform == "qemu" and (machine != 183 or min(start for start, _ in ranges) != 0x50000000):
        raise ValueError("QEMU test media requires the ARM native kernel load base")
    return ranges


class HeaderSink:
    # ------------------------=
    # FUNC: __init__
    # DESC: Retains only a bounded ELF header prefix while the complete artifact is streamed.
    # ------------------=
    def __init__(self):
        self.header = bytearray()

    # ------------------------=
    # FUNC: write
    # DESC: Consumes bytes without retaining multi-gigabyte kernel data.
    # ------------------=
    def write(self, data):
        self.header.extend(data[:max(0, 65536 - len(self.header))])
        return len(data)
