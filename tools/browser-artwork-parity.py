"""Check browser artwork bytes in installed System Generation ESP templates.

This proves asset packaging only, not browser availability or installed execution.
"""
import json
import mmap
import os
from pathlib import Path
import struct
import subprocess
import tempfile


# ------------------------=
# FUNC: verify_embedded_fonts
# DESC: Checks exact native Inter raster and metric bytes inside each shipped installed kernel.
# ------------------=
def verify_embedded_fonts(kernel, root):
    records = []
    with kernel.open("rb") as stream, mmap.mmap(stream.fileno(), 0, access=mmap.ACCESS_READ) as image:
        # The native desktop currently selects only 1x/2x UI scale. Higher
        # source atlases are unreachable and legitimately dead-stripped.
        for size in (14, 28):
            for extension, length in (("atlas", size * (size + 6) * 95), ("metrics", 95), ("kern", 95 * 95)):
                asset = root / "assets/fonts" / f"InfinityBrowser-Regular-{size}.{extension}"
                expected = asset.read_bytes()
                assert len(expected) == length, (asset, len(expected), length)
                offset = image.find(expected)
                assert offset >= 0, (kernel, asset)
                records.append({"asset": asset.name, "bytes": length, "kernel_offset": offset})
    return records


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
        "infinity-world-shift-icon-v1-source.png": (1254, 1254),
        "infinity-holographic-desktop-icon-v1-source.png": (1254, 1254),
    }
    records = []
    with tempfile.TemporaryDirectory(prefix="browser-artwork-", dir=root / "build/tmp") as temporary:
        for architecture in ("x86_64", "aarch64"):
            records.append({"architecture": architecture, "embedded_fonts": verify_embedded_fonts(
                root / "build" / architecture / "installed-kernel.elf", root)})
            image = root / "build" / architecture / "installed-esp.img"
            for name, dimensions in assets.items():
                expected = (root / "assets/apps" / name).read_bytes()
                assert expected[:8] == b"\x89PNG\r\n\x1a\n"
                assert struct.unpack(">II", expected[16:24]) == dimensions
                assert expected[24:26] == bytes((8, 6)), "Expected 8-bit RGBA source"
                output = Path(temporary) / name
                output.unlink(missing_ok=True)
                subprocess.run(["mcopy", "-o", "-i", str(image),
                    "::/EFI/InfinityOS/Applications/" + name, str(output)], check=True)
                assert output.read_bytes() == expected, f"Stale or altered asset: {architecture}/{name}"
                records.append({"architecture": architecture, "asset": name,
                    "bytes": len(expected), "dimensions": dimensions, "rgba": True})
            for name in ("infinity-browser-icon-v1.bmp","infinity-browser-navigation-v1.bmp",
                         "infinity-world-shift-icon-v1.bmp", "infinity-holographic-desktop-icon-v1.bmp", "app.infinity.browser.manifest"):
                expected=(root/"assets/apps"/name).read_bytes()
                output=Path(temporary)/name
                output.unlink(missing_ok=True)
                subprocess.run(["mcopy","-o","-i",str(image),
                    "::/EFI/InfinityOS/Applications/"+name,str(output)],check=True)
                assert output.read_bytes()==expected,(architecture,name)
                records.append({"architecture":architecture,"asset":name,"bytes":len(expected)})
    print(json.dumps({"artwork_parity": records, "browser_installed": False}, indent=2))


if __name__ == "__main__":
    main()
