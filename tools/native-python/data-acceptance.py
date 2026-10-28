"""Run this same saved source across native invocations and detached cold boots."""
import os
import errno
import sqlite3
import sys

phase = sys.argv[1]
assert errno.ESTALE == 133  # Native CPython exposes the same pinned Newlib errno ABI as the checkpoint adapter.
assert os.path.isdir("/data")
assert os.getcwd() == "/data"
db = sqlite3.connect("application.sqlite")
assert db.execute("PRAGMA journal_mode=DELETE").fetchone()[0] == "delete"
db.execute("PRAGMA synchronous=FULL")
db.execute("PRAGMA cache_size=1")

if phase == "write":
    db.execute("CREATE TABLE IF NOT EXISTS records (id INTEGER PRIMARY KEY, value BLOB NOT NULL)")
    db.execute("DELETE FROM records")
    db.execute("INSERT INTO records VALUES (1, ?)", (b"A" * 131072,))
    db.commit()
    with open("/data/note.txt", "wb") as stream:
        stream.write(b"native durable file\n")
        stream.flush()
        os.fsync(stream.fileno())
    db.close()
elif phase == "commit-crash":
    db.execute("UPDATE records SET value=? WHERE id=1", (b"B" * 131072,))
    db.commit()
    assert not os.path.exists("/data/application.sqlite-journal")
    os._exit(19)  # Bypasses Python cleanup and the launcher's successful-exit checkpoint.
elif phase == "pending-crash":
    db.execute("BEGIN IMMEDIATE")
    db.execute("UPDATE records SET value=? WHERE id=1", (b"C" * 131072,))
    assert os.path.exists("/data/application.sqlite-journal")
    os._exit(23)  # Hot-journal recovery must preserve the previous committed B value.
elif phase in ("read-a", "read-b"):
    expected = b"A" if phase == "read-a" else b"B"
    assert db.execute("SELECT value FROM records WHERE id=1").fetchone()[0] == expected * 131072
    assert db.execute("PRAGMA integrity_check").fetchone()[0] == "ok"
    assert db.execute("SELECT count(*) FROM records").fetchone()[0] == 1
    with open("/data/note.txt", "rb") as stream:
        assert stream.read() == b"native durable file\n"
    db.close()
else:
    raise AssertionError(phase)
