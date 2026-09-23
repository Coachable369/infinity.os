"""Exercise real Make ISO recipes and inspect their emitted binary artifacts."""
from pathlib import Path
import subprocess
import tempfile


# ------------------------=
# FUNC: main
# DESC: Executes packaging recipes on tiny fixtures and verifies output location and extracted bytes.
# ------------------=
def main():
    root = Path(__file__).resolve().parent.parent
    cases = (
        ("x86_64", "InfinityOS-x86_64.iso"),
        ("aarch64", "InfinityOS-aarch64-bootstrap-test.iso"),
        ("aarch64-qemu", "InfinityOS-aarch64-qemu-test.iso"),
    )
    with tempfile.TemporaryDirectory(prefix="infinity-iso-output-") as directory:
        work = Path(directory)
        for architecture, name in cases:
            fat = "fat" if architecture == "x86_64" else f"fat-{architecture}"
            for entry in ("BOOT", "INFINITY"):
                folder = work / "build" / fat / "EFI" / entry
                folder.mkdir(parents=True)
                (folder / "fixture.bin").write_bytes(bytes(range(256)))
            image = f"build/infinity-{architecture}.img"
            (work / image).write_bytes(bytes(range(256)) * 16)
            subprocess.run(
                ["make", "-f", str(root / "Makefile"), "-o", image, f"builds/{name}"],
                cwd=work, check=True, stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
            artifact = work / "builds" / name
            assert artifact.is_file() and artifact.stat().st_size > 0
            extracted = work / f"{architecture}.bin"
            subprocess.run(
                ["xorriso", "-osirrox", "on", "-indev", str(artifact),
                 "-extract", "/EFI/BOOT/fixture.bin", str(extracted)],
                check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
            )
            assert extracted.read_bytes() == bytes(range(256))
        assert not list((work / "build").rglob("*.iso"))
        assert len(list((work / "builds").glob("*.iso"))) == len(cases)


if __name__ == "__main__":
    main()
