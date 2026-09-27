"""Check browser artwork bytes in installed System Generation ESP templates.

This proves asset packaging only, not browser availability or installed execution.
"""
import json
import os
from pathlib import Path
import struct
import subprocess
import tempfile


# ------------------------=
# FUNC: main
# DESC: Compares exact PNG payloads extracted from both installed ESPs with their approved sources.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Run through ./build-kit run python3 tools/browser-artwork-parity.py")
    root = Path(__file__).resolve().parents[1]
    assets = {
        "infinity-browser-icon-v1-source.png": (1254, 1254),
        "infinity-browser-navigation-v1-source.png": (1774, 887),
    }
    records = []
    with tempfile.TemporaryDirectory(prefix="browser-artwork-", dir=root / "build/tmp") as temporary:
        for architecture in ("x86_64", "aarch64"):
            image = root / "build" / architecture / "installed-esp.img"
            for name, dimensions in assets.items():
                expected = (root / "assets/apps" / name).read_bytes()
                assert expected[:8] == b"\x89PNG\r\n\x1a\n"
                assert struct.unpack(">II", expected[16:24]) == dimensions
                assert expected[24:26] == bytes((8, 6)), "Expected 8-bit RGBA source"
                output = Path(temporary) / name
                subprocess.run(["mcopy", "-o", "-i", str(image),
                    "::/EFI/InfinityOS/Applications/" + name, str(output)], check=True)
                assert output.read_bytes() == expected, f"Stale or altered asset: {architecture}/{name}"
                records.append({"architecture": architecture, "asset": name,
                    "bytes": len(expected), "dimensions": dimensions, "rgba": True})
            for name in ("infinity-browser-icon-v1.bmp","infinity-browser-navigation-v1.bmp",
                         "app.infinity.browser.manifest"):
                expected=(root/"assets/apps"/name).read_bytes()
                output=Path(temporary)/name
                subprocess.run(["mcopy","-o","-i",str(image),
                    "::/EFI/InfinityOS/Applications/"+name,str(output)],check=True)
                assert output.read_bytes()==expected,(architecture,name)
                records.append({"architecture":architecture,"asset":name,"bytes":len(expected)})
    print(json.dumps({"artwork_parity": records, "browser_installed": False}, indent=2))


if __name__ == "__main__":
    main()
