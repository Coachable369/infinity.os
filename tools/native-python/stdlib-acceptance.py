"""Native 3.13 library semantics; executed only inside the isolated guest."""
import sys
assert sys.platform == "infinityos"
assert sys.version_info[:2] == (3, 13)
import _infinity_modules
import abc, argparse, array, ast, base64, binascii, bisect
import calendar, cmath, collections, contextlib, contextvars, copy, csv
import dataclasses, datetime, decimal, difflib, enum, fnmatch, fractions
import functools, gc, getopt, hashlib, heapq, hmac, html, io, ipaddress
import itertools, json, keyword, math, operator, pathlib, pickle, pprint
import random, re, secrets, shlex, statistics, string, struct, textwrap
import time, types, typing, unicodedata, urllib.parse, uuid, warnings, weakref
import xml.etree.ElementTree as ET
import zipfile, zlib, bz2, lzma, tarfile, _queue, cProfile, zoneinfo, sqlite3, tkinter

assert decimal.Decimal("0.1") + decimal.Decimal("0.2") == decimal.Decimal("0.3")
assert fractions.Fraction(1, 3) + fractions.Fraction(2, 3) == 1
assert json.loads(json.dumps({"answer": 42})) == {"answer": 42}
assert hashlib.sha256(b"abc").digest() == bytes.fromhex(
    "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad")
assert zlib.decompress(zlib.compress(b"native" * 1024)) == b"native" * 1024
payload = bytes(range(256)) * 256
compressed = bz2.compress(payload)
assert bz2.decompress(compressed) == payload
for format in (lzma.FORMAT_XZ, lzma.FORMAT_ALONE):
    encoded = lzma.compress(payload, format=format, preset=1)
    assert lzma.decompress(encoded) == payload
    stream = lzma.LZMADecompressor(format=format)
    assert stream.decompress(encoded[:9]) + stream.decompress(encoded[9:]) == payload
    assert stream.eof
try:
    lzma.decompress(b"invalid compressed stream")
except lzma.LZMAError:
    pass
else:
    raise AssertionError("Invalid LZMA stream accepted")
decoder = bz2.BZ2Decompressor()
decoded = b"".join(decoder.decompress(compressed[index:index + 17])
    for index in range(0, len(compressed), 17))
assert decoded == payload and decoder.eof
try:
    bz2.decompress(b"not a bzip2 stream")
except OSError:
    pass
else:
    raise AssertionError("invalid bzip2 stream accepted")
archive = io.BytesIO()
with tarfile.open(fileobj=archive, mode="w:bz2") as bundle:
    entry = tarfile.TarInfo("native.txt")
    entry.size = len(payload)
    bundle.addfile(entry, io.BytesIO(payload))
archive.seek(0)
with tarfile.open(fileobj=archive, mode="r:bz2") as bundle:
    assert bundle.extractfile("native.txt").read() == payload
assert ET.fromstring(b"<root><value>42</value></root>").findtext("value") == "42"
assert statistics.NormalDist().inv_cdf(0.5) == 0
queue = _queue.SimpleQueue()
queue.put(42)
assert queue.get_nowait() == 42 and queue.empty()
assert uuid.uuid4().version == 4
for text, codec in (("中文", "gb18030"), ("日本語", "shift_jis"),
                    ("日本語", "iso2022_jp"), ("한국어", "euc_kr"),
                    ("中文", "big5"), ("中文", "big5hkscs")):
    assert text.encode(codec).decode(codec) == text
zone_bytes = b"TZif\0" + bytes(15) + struct.pack(
    ">6l", 0, 0, 0, 0, 1, 4) + struct.pack(
    ">lBB", 3600, 0, 0) + b"ONE\0"
zone = zoneinfo.ZoneInfo.from_file(io.BytesIO(zone_bytes), key="Native/Fixture")
assert datetime.datetime(2026, 1, 1, tzinfo=zone).utcoffset().total_seconds() == 3600
chicago = zoneinfo.ZoneInfo("America/Chicago")
assert datetime.datetime(2026, 1, 1, tzinfo=chicago).utcoffset().total_seconds() == -21600
assert datetime.datetime(2026, 7, 1, tzinfo=chicago).utcoffset().total_seconds() == -18000
tokyo = zoneinfo.ZoneInfo("Asia/Tokyo")
assert datetime.datetime(2026, 7, 1, tzinfo=tokyo).utcoffset().total_seconds() == 32400
assert {"America/Chicago", "Asia/Tokyo", "UTC"} <= zoneinfo.available_timezones()
from importlib.metadata import version
assert version("tzdata") == "2026.5"
profile = cProfile.Profile()
assert profile.runcall(sum, range(10)) == 45
assert len(profile.getstats()) > 0
database = sqlite3.connect(":memory:")
database.execute("create table items (id integer primary key, name text unique, payload blob)")
database.executemany("insert into items(name,payload) values (?,?)",
    [("one", b"\x00\xff"), ("two", bytes(range(16)))])
database.commit()
assert database.execute("select count(*) from items").fetchone() == (2,)
assert database.execute("select payload from items where name=?", ("one",)).fetchone() == (b"\x00\xff",)
try:
    with database:
        database.execute("insert into items(name) values ('three')")
        database.execute("insert into items(name) values ('one')")
except sqlite3.IntegrityError:
    pass
else:
    raise AssertionError("SQLite uniqueness violation was ignored")
assert database.execute("select count(*) from items").fetchone() == (2,)
database.create_function("double", 1, lambda number: number * 2)
assert database.execute("select double(21), json_extract('{\"a\":7}', '$.a')").fetchone() == (42, 7)
database.execute("create virtual table terms using fts5(text)")
database.execute("insert into terms values ('native python database')")
assert database.execute("select count(*) from terms where terms match 'native'").fetchone() == (1,)
database.commit()  # Flush virtual-table transaction buffers before exporting the database image.
image = database.serialize()
copy_database = sqlite3.connect(":memory:")
copy_database.deserialize(image)
assert copy_database.execute("select name from items order by id").fetchall() == [("one",), ("two",)]
integrity = copy_database.execute("pragma integrity_check").fetchall()
assert integrity == [("ok",)], integrity
database.close()
copy_database.close()
# The real upstream interpreter is exercised without requiring a display grant.
# Widget and native input acceptance is independently performed after installation.
tcl = tkinter.Tcl()
assert int(tcl.eval("expr {6 * 7}")) == 42
tcl.setvar("native_value", 19)
assert tcl.getint(tcl.getvar("native_value")) == 19
assert tcl.splitlist(tcl.call("list", "one", "two words", "three")) == ("one", "two words", "three")
tcl.tk.createcommand("native_multiply", lambda value: int(value) * 3)
assert tcl.getint(tcl.call("native_multiply", 14)) == 42
tcl.tk.deletecommand("native_multiply")
independent_tcl = tkinter.Tcl()
assert independent_tcl.getint(independent_tcl.call("info", "exists", "native_value")) == 0
try:
    tcl.eval("expr {1 / 0}")
except tkinter.TclError:
    pass
else:
    raise AssertionError("Tcl arithmetic failure was not delivered")
del independent_tcl, tcl
assert _infinity_modules.validate_source("import json\nimport math\n", "native.py") == ("json", "math")
try:
    _infinity_modules.validate_source("import not_installed_infinity\n", "native.py")
except _infinity_modules.DependencyError as error:
    assert error.issues[0]["module"] == "not_installed_infinity"
else:
    raise AssertionError("missing generated dependency accepted")
inventory = _infinity_modules.inventory()
assert inventory["schema_version"] == 1
for name in ("_elementtree", "_queue", "_zoneinfo", "_lsprof", "_codecs_cn", "_bz2", "_sqlite3", "_lzma", "_tkinter"):
    assert any(item["name"] == name and item["kind"] == "builtin" for item in inventory["modules"])
print("Native standard library behavior completed")
sys.exit(0)
