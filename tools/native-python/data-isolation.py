"""An independently saved source must never inherit another source's data."""
import os
import sys

assert os.path.isdir("/data")
assert not os.path.exists("/data/application.sqlite")
assert not os.path.exists("/data/note.txt")
for protected in (sys.argv[0], "/packages/unapproved-data-write.py"):
    try:
        with open(protected, "wb") as stream:
            stream.write(b"unauthorized")
    except PermissionError:
        pass
    else:
        raise AssertionError(protected)
with open("/data/other-source.txt", "wb") as stream:
    stream.write(b"isolated source")
