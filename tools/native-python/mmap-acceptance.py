"""Real anonymous mapping semantics inside an isolated native Python process."""
import sys
import mmap

assert sys.platform == "infinityos"
assert mmap.__spec__.origin == "built-in"
for flags in (mmap.MAP_SHARED, mmap.MAP_PRIVATE):
    with mmap.mmap(-1, 8192, flags=flags) as region:
        assert len(region) == 8192 and region[:] == bytes(8192)
        assert region.write(b"native\nmap") == 10
        assert region.tell() == 10
        region.seek(0)
        assert region.readline() == b"native\n"
        assert region.read(3) == b"map"
        region[4096:4100] = b"page"
        region.move(20, 4096, 4)
        assert region[20:24] == b"page"
        assert region.find(b"page", 30) == 4096
        assert region.flush() is None
        assert region.flush(4096, 4096) is None
        exported = memoryview(region)
        exported[30] = 42
        assert region[30] == 42
        try:
            region.close()
        except BufferError:
            pass
        else:
            raise AssertionError("live exported buffer was unmapped")
        exported.release()
    assert region.closed
with mmap.mmap(-1, 4096, access=mmap.ACCESS_READ) as readonly:
    assert readonly[:] == bytes(4096)
    try:
        readonly[0] = 1
    except TypeError:
        pass
    else:
        raise AssertionError("read-only mapping became writable")
for _ in range(64):
    with mmap.mmap(-1, 4096) as recycled:
        assert recycled[0] == 0
        recycled[0] = 255
try:
    mmap.mmap(-1, 2 * 1024 * 1024)
except OSError:
    pass
else:
    raise AssertionError("native committed-page limit was bypassed")
