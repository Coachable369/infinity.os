#!/usr/bin/env python3
import hashlib
import struct
from pathlib import Path

root = Path(__file__).resolve().parents[1]
names = ["worldshift-hero-v1.bmp"] + [f"spatial-world-{index}.bmp" for index in range(4)]
digests = set()
for name in names:
    payload = (root / "assets" / "desktop" / name).read_bytes()
    assert payload[:2] == b"BM"
    width, height = struct.unpack_from("<ii", payload, 18)
    bits_per_pixel = struct.unpack_from("<H", payload, 28)[0]
    assert width >= 1600 and abs(height) >= 900
    assert bits_per_pixel == 24
    assert len(payload) > 4_000_000
    digests.add(hashlib.sha256(payload).digest())
assert len(digests) == len(names)
print("World Shift HD artwork artifacts: PASS")
