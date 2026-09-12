"""Update a stopped VM's private RAW clone, validating boot records before writes."""
import argparse
import os
import zlib
import hashlib
import json

# ------------------------=
# FUNC: integer
# DESC: Reads a little-endian field.
# ------------------=
def integer(data, offset, size=8):
    return int.from_bytes(data[offset:offset + size], 'little')

# ------------------------=
# FUNC: put
# DESC: Changes a bounded little-endian field.
# ------------------=
def put(data, offset, value, size=8):
    data[offset:offset + size] = value.to_bytes(size, 'little')

# ------------------------=
# FUNC: record_crc
# DESC: Verifies or refreshes a trailing record CRC32.
# ------------------=
def record_crc(data, update=False):
    copy = bytearray(data)
    expected = integer(copy, len(copy) - 4, 4)
    put(copy, len(copy) - 4, 0, 4)
    actual = zlib.crc32(copy)
    if update:
        put(data, len(data) - 4, actual, 4)
    else:
        assert actual == expected, 'Record CRC mismatch'

# ------------------------=
# FUNC: main
# DESC: Validates the installed kernel and all referring records before patching a clone; verifies every changed byte.
# ------------------=
def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('clone')
    parser.add_argument('kernel')
    parser.add_argument('--apply', action='store_true')
    parser.add_argument('--export-dir', help='Export validated sector patches for an offline backed-up image')
    args = parser.parse_args()
    with open(args.kernel, 'rb') as stream:
        kernel = stream.read()
    assert kernel[:6] == b'\x7fELF\x02\x01' and integer(kernel, 18, 2) == 183
    with open(args.clone, 'r+b' if args.apply else 'rb') as disk:
        # ------------------------=
        # FUNC: read
        # DESC: Reads an exact extent from the private disk clone.
        # ------------------=
        def read(lba, size=512):
            disk.seek(lba * 512)
            result = bytearray(disk.read(size))
            assert len(result) == size
            return result
        gpt = read(1)
        assert gpt[:8] == b'EFI PART'
        header_size, expected = integer(gpt, 12, 4), integer(gpt, 16, 4)
        put(gpt, 16, 0, 4)
        assert zlib.crc32(gpt[:header_size]) == expected
        count, stride = integer(gpt, 80, 4), integer(gpt, 84, 4)
        assert 0 < count <= 128 and stride == 128
        entries = read(integer(gpt, 72), count * stride)
        assert zlib.crc32(entries) == integer(gpt, 88, 4)
        found = []
        for index in range(count):
            entry = entries[index * stride:(index + 1) * stride]
            first, last = integer(entry, 32), integer(entry, 40)
            if first and read(first)[:8] == b'INFCONT1':
                found.append((first, last))
        assert len(found) == 1
        first, last = found[0]
        stores = []
        for offset in (262144, 257 * 2048):
            for slot in (0, 1):
                root = read(first + offset + slot)
                if root[:8] == b'INFOROOT':
                    record_crc(root)
                    stores.append(offset)
                    break
        assert len(stores) == 1, 'Ambiguous or missing object-store boundary'
        kernel_limit = stores[0]
        header = read(first)
        record_crc(header)
        assert integer(header, 16, 4) == 4
        catalog = read(first + integer(header, 96))
        record_crc(catalog)
        assert catalog[:8] == b'INFBOOT1'
        manifest_lba = first + integer(catalog, 32)
        manifest = read(manifest_lba)
        record_crc(manifest)
        assert manifest[:8] == b'INFSYSM1' and integer(manifest, 16, 4) == 3
        assert integer(manifest, 24) == integer(catalog, 24)
        relative, old_size, old_crc = integer(manifest, 40), integer(manifest, 48), integer(manifest, 56, 4)
        assert relative == integer(header, 48) == 2048
        assert old_size == integer(header, 56) and 0 < old_size <= (kernel_limit - relative) * 512
        assert zlib.crc32(read(first + relative, old_size)) == old_crc
        components_lba = first + integer(manifest, 64)
        components = read(components_lba, 1024)
        record_crc(components)
        assert components[:8] == b'INFCOMP1' and integer(components, 8, 4) == 2
        assert integer(components, 20, 4) == 48
        count = integer(components, 16, 4)
        assert 0 < count <= 20
        crc = zlib.crc32(kernel)
        references = 0
        for index in range(count):
            offset = 32 + index * 48
            if integer(components, offset + 28, 4) == 1:
                assert integer(components, offset + 32) == relative
                assert integer(components, offset + 40) == old_size
                assert integer(components, offset + 24, 4) == old_crc
                put(components, offset + 24, crc, 4)
                put(components, offset + 40, len(kernel))
                references += 1
        assert references
        padded = kernel + bytes((-len(kernel)) % 512)
        assert relative + len(padded) // 512 <= kernel_limit
        assert first + relative + len(padded) // 512 <= last
        put(header, 56, len(kernel))
        put(manifest, 48, len(kernel))
        put(manifest, 56, crc, 4)
        for record in (header, manifest, components):
            record_crc(record, True)
        changes = [(first + relative, padded), (components_lba, components), (manifest_lba, manifest), (first, header)]
        # ------------------------=
        # FUNC: unchanged_digest
        # DESC: Hashes every disk byte outside the exact approved kernel and manifest write extents.
        # ------------------=
        def unchanged_digest():
            digest = hashlib.sha256()
            ranges = sorted((lba * 512, lba * 512 + len(data)) for lba, data in changes)
            position = 0
            for start, end in ranges + [(os.fstat(disk.fileno()).st_size, os.fstat(disk.fileno()).st_size)]:
                assert start >= position
                disk.seek(position)
                while position < start:
                    chunk = disk.read(min(4 * 1024 * 1024, start - position))
                    assert chunk
                    digest.update(chunk)
                    position += len(chunk)
                position = end
            return digest.digest()
        if args.apply:
            unchanged = unchanged_digest()
            for lba, data in changes:
                disk.seek(lba * 512)
                disk.write(data)
            disk.flush()
            os.fsync(disk.fileno())
            for lba, data in changes:
                assert read(lba, len(data)) == data
            assert unchanged_digest() == unchanged, 'Unrelated disk bytes changed'
        if args.export_dir:
            os.mkdir(args.export_dir)
            patches = []
            for index, (lba, data) in enumerate(changes):
                path = os.path.abspath(os.path.join(args.export_dir, f'{index}.bin'))
                with open(path, 'xb') as stream:
                    stream.write(data)
                patches.append(dict(offset=lba * 512, length=len(data), path=path,
                                    sha256=hashlib.sha256(data).hexdigest()))
            with open(os.path.join(args.export_dir, 'patches.json'), 'x') as stream:
                json.dump(patches, stream)
        print(dict(applied=args.apply, kernel_bytes=len(kernel), references=references, container_lba=first))

if __name__ == '__main__':
    main()
