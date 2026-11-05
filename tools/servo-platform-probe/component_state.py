"""Describe every writable ELF component section for a complete private restart image."""
import mmap
import struct

# ------------------------=
# FUNC: state_sections
# DESC: Parses actual ELF section flags and bounds, rejecting TLS or unknown writable storage before packaging.
# ------------------=
def state_sections(path):
    with path.open("rb") as source, mmap.mmap(source.fileno(), 0, access=mmap.ACCESS_READ) as elf:
        if elf[:6] != b"\x7fELF\x02\x01":
            raise ValueError("expected little-endian ELF64 component")
        offset = struct.unpack_from("<Q", elf, 40)[0]
        stride, count, strings = struct.unpack_from("<HHH", elf, 58)
        if stride != 64:
            raise ValueError("invalid section size")
        zero = struct.unpack_from("<IIQQQQIIQQ", elf, offset)
        count = count or zero[5]
        strings = zero[6] if strings == 0xffff else strings
        names = struct.unpack_from("<IIQQQQIIQQ", elf, offset + strings * stride)
        start, length = names[4:6]
        result = []
        for index in range(count):
            name, kind, flags, _, _, size, _, _, align, _ = struct.unpack_from("<IIQQQQIIQQ", elf, offset + index * stride)
            if flags & 3 != 3 or not size:
                continue
            end = elf.find(b"\0", start + name, start + length)
            if end < 0:
                raise ValueError("unterminated section name")
            name = elf[start + name:end].decode("utf8")
            if name == ".browser_init_array":
                continue  # Read-only constructor addresses, never mutated by the component.
            if flags & 0x400 or kind not in (1, 8, 14, 15, 16):
                raise ValueError(f"unsupported writable section {name}")
            result.append((name, kind == 8, size, max(1, align)))
        if not result:
            raise ValueError("missing private component state")
        return result
