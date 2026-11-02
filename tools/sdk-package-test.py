#!/usr/bin/env python3
"""Verify archive contents against canonical source bytes and native catalog export."""
import hashlib
import json
from pathlib import Path
import tarfile

ROOT = Path(__file__).resolve().parents[1]

# ------------------------=
# FUNC: main
# DESC: Validates every released file and compares runtime-exported bytes to archive members.
# ------------------=
def main():
    with tarfile.open(ROOT / "builds/InfintySDK.tar.gz") as tar:
        manifest = json.load(tar.extractfile("InfintySDK/manifest.json"))
        assert manifest["root"] == "/InfintySDK"
        expected = {"InfintySDK/manifest.json"}
        for record in manifest["files"]:
            name = "InfintySDK/" + record["path"]
            expected.add(name)
            data = tar.extractfile(name).read()
            assert len(data) == record["size"]
            assert hashlib.sha256(data).hexdigest() == record["sha256"]
            assert data == (ROOT / record["source"]).read_bytes()
            assert data == (ROOT / "build/tests/sdk-export" / record["path"]).read_bytes()
        assert set(tar.getnames()) == expected
    print(f"PASS: {len(manifest['files'])} runtime/archive/source byte parity checks")

if __name__ == "__main__":
    main()
