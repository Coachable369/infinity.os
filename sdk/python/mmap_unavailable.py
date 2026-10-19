"""Explicitly unavailable file mapping boundary for isolated native Python.

The package profile provides bounded streams, not a file-backed mapping service.
Upstream pip imports this module for its HTTP cache wrapper even during offline
installation; constructing a mapping must still fail instead of copying a file
and pretending that the result is a coherent shared memory map.
"""
import errno

ACCESS_DEFAULT = 0
ACCESS_READ = 1
ACCESS_WRITE = 2
ACCESS_COPY = 3
error = OSError


class mmap:
    # ------------------------=
    # FUNC: __new__
    # DESC: Rejects unavailable mappings without allocating or claiming file coherence.
    # ------------------=
    def __new__(cls, *args, **kwargs):
        raise OSError(errno.ENOTSUP, "File-backed memory mapping is unavailable in native Python")
