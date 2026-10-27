"""Behavioral payload comparison for the default Infinity UI Design Kit."""
import os
import subprocess
import tempfile
import argparse
import mmap
import json
import struct
import importlib.util
from pathlib import Path

assert os.environ.get("INFINITY_BUILD_KIT_ACTIVE"), "Use ./build-kit run"
root = Path(__file__).resolve().parents[1]
kit = root / "assets/ui-design-kit/default"
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--arch", action="append", choices=("aarch64", "x86_64"))
parser.add_argument("--kernel", action="append", type=Path, default=[])
args = parser.parse_args()
for arch in args.arch or ("aarch64", "x86_64"):
    with tempfile.TemporaryDirectory(dir=root / "build") as directory:
        for source in [*kit.iterdir(), kit.parent / "uikit.infinityui", kit.parent / "uikit-catalog.json"]:
            target = Path(directory) / source.name
            subprocess.run(["mcopy", "-i", str(root / f"build/{arch}/installed-esp.img"),
                            f"::/EFI/InfinityOS/InfinityUI/DesignKit/Default/{source.name}", str(target)], check=True)
            assert target.read_bytes() == source.read_bytes(), (arch, source.name)
document = json.loads((kit.parent / "uikit.infinityui").read_text())
expected_tokens = b"IUIKIT01" + struct.pack("<HH13I", 1, 13, *document["colors"])
assert (kit / "uikit-tokens.bin").read_bytes() == expected_tokens
spec = importlib.util.spec_from_file_location("uikit_compiler", root / "tools/compile-uikit.py")
compiler = importlib.util.module_from_spec(spec)
spec.loader.exec_module(compiler)
assert (kit / "uikit-styles.bin").read_bytes() == compiler.style_bytes(document)
for kernel in args.kernel:
    with kernel.open("rb") as stream, mmap.mmap(stream.fileno(), 0, access=mmap.ACCESS_READ) as binary:
        # The face atlas remains Studio artwork; native animation embeds rigid geometry.
        for source in [*(p for p in kit.glob("*.rgba") if p.name != "face-digital-turn-v1.rgba"), kit / "face-canonical-v1.mesh", kit / "browser-welcome-hero-v1.bmp", kit / "eclipse-access-wallpaper-v1.bmp", kit / "uikit-tokens.bin", kit / "uikit-styles.bin"]:
            assert binary.find(source.read_bytes()) >= 0, (kernel, source.name)
print("Selected installed design-kit payloads and kernel sprites match their source bytes")
