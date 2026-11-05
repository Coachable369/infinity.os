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
    with tarfile.open(ROOT / "builds/InfinitySDK.tar.gz") as tar:
        manifest = json.load(tar.extractfile("InfinitySDK/manifest.json"))
        assert manifest["root"] == "/InfinitySDK"
        expected = {"InfinitySDK/manifest.json"}
        for record in manifest["files"]:
            name = "InfinitySDK/" + record["path"]
            expected.add(name)
            data = tar.extractfile(name).read()
            assert len(data) == record["size"]
            assert hashlib.sha256(data).hexdigest() == record["sha256"]
            assert data == (ROOT / record["source"]).read_bytes()
            assert data == (ROOT / "build/tests/sdk-export" / record["path"]).read_bytes()
        assert set(tar.getnames()) == expected
        catalog = json.load(tar.extractfile("InfinitySDK/Examples/catalog.json"))
        assert catalog["version"] == 1 and catalog["root"] == manifest["root"]
        assert "InfinitySDK/" + catalog["instructions"] in expected
        identities = set()
        languages = set()
        for example in catalog["examples"]:
            assert example["id"] not in identities
            identities.add(example["id"])
            languages.add(example["language"])
            assert example["language"] in ("python", "c")
            assert example["permission"] in ("none", "camera", "network")
            assert example["capabilities"] and example["expected"]
            for field in ("source", "instructions", "reference"):
                assert "InfinitySDK/" + example[field] in expected
                assert tar.extractfile("InfinitySDK/" + example[field]).read()
            assert example["source"].endswith(".py" if example["language"] == "python" else ".c")
        assert languages == {"python", "c"}
        assert len(identities) == 11
    print(f"PASS: {len(manifest['files'])} runtime/archive/source byte parity checks")

if __name__ == "__main__":
    main()
