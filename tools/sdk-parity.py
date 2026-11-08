#!/usr/bin/env python3
"""Assert every SDK resource is embedded in each requested live/installed kernel."""
import json
import mmap
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[1]

# ------------------------=
# FUNC: main
# DESC: Compares binary SDK payloads against canonical sources in live and installed release kernels.
# ------------------=
def main():
    manifest_bytes = (ROOT / "build/sdk/manifest.json").read_bytes()
    manifest = json.loads(manifest_bytes)
    if not sys.argv[1:]:
        raise SystemExit("Provide live and installed kernel ELF paths")
    for name in sys.argv[1:]:
        with (ROOT / name).open("rb") as stream, mmap.mmap(stream.fileno(), 0, access=mmap.ACCESS_READ) as binary:
            assert binary.find(manifest_bytes) >= 0, (name, "SDK manifest absent or stale")
            for record in manifest["files"]:
                payload = (ROOT / record["source"]).read_bytes()
                assert binary.find(payload) >= 0, (name, record["path"])
            for tool in [*(p for p in (ROOT / "runtime/install").rglob("*") if p.is_file()), *(p for p in (ROOT / "sdk/user-template").rglob("*") if p.is_file())]:
                assert binary.find(tool.read_bytes()) >= 0, (name, tool.name, "system payload absent or stale")
        print(f"PASS: {name}: {len(manifest['files'])} SDK binary resources and manifest")

if __name__ == "__main__":
    main()
