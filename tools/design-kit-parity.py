"""Behavioral payload comparison for the default Infinity UI Design Kit."""
import os
import subprocess
import tempfile
from pathlib import Path

assert os.environ.get("INFINITY_BUILD_KIT_ACTIVE"), "Use ./build-kit run"
root = Path(__file__).resolve().parents[1]
kit = root / "assets/ui-design-kit/default"
for arch in ("aarch64", "x86_64"):
    with tempfile.TemporaryDirectory(dir=root / "build") as directory:
        for source in kit.iterdir():
            target = Path(directory) / source.name
            subprocess.run(["mcopy", "-i", str(root / f"build/{arch}/installed-esp.img"),
                            f"::/EFI/InfinityOS/InfinityUI/DesignKit/Default/{source.name}", str(target)], check=True)
            assert target.read_bytes() == source.read_bytes(), (arch, source.name)
print("Both installed design-kit payloads match their source bytes")
