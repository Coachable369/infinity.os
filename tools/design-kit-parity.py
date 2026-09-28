"""Behavioral payload comparison for the default Infinity UI Design Kit."""
import os
import subprocess
import tempfile
import argparse
import mmap
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
        for source in kit.iterdir():
            target = Path(directory) / source.name
            subprocess.run(["mcopy", "-i", str(root / f"build/{arch}/installed-esp.img"),
                            f"::/EFI/InfinityOS/InfinityUI/DesignKit/Default/{source.name}", str(target)], check=True)
            assert target.read_bytes() == source.read_bytes(), (arch, source.name)
for kernel in args.kernel:
    with kernel.open("rb") as stream, mmap.mmap(stream.fileno(), 0, access=mmap.ACCESS_READ) as binary:
        for source in kit.glob("*.rgba"):
            assert binary.find(source.read_bytes()) >= 0, (kernel, source.name)
print("Selected installed design-kit payloads and kernel sprites match their source bytes")
