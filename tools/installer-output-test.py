"""Exercise real Make ISO recipes and inspect their emitted binary artifacts."""
from pathlib import Path
import os
import subprocess
import tempfile


# ------------------------=
# FUNC: main
# DESC: Executes packaging recipes on tiny fixtures and verifies output location and extracted bytes.
# ------------------=
def main():
    root = Path(__file__).resolve().parent.parent
    cases = (
        ("x86_64", "InfinityOS-x86_64.bootmedia"),
        ("aarch64", "InfinityOS-aarch64.bootmedia"),
        ("aarch64-qemu", "InfinityOS-aarch64-qemu.bootmedia"),
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
            result = subprocess.run(
                # This fixture tests ISO packaging only. Native model parity is
                # exercised separately against actual linked kernel artifacts.
                ["make", "-f", str(root / "Makefile"), "-o", image,
                 "-o", "x86-native-speech-parity", f"build/test-media/{name}"],
                cwd=work, stdout=subprocess.DEVNULL,
                stderr=subprocess.PIPE, text=True,
                env={**os.environ, "INFINITY_ISO_BUILD_AUTHORITY": "build.sh"},
            )
            assert result.returncode == 0, result.stderr[-4000:]
            artifact = work / "build" / "test-media" / name
            assert artifact.is_file() and artifact.stat().st_size > 0
            extracted = work / f"{architecture}.bin"
            subprocess.run(
                ["xorriso", "-osirrox", "on", "-indev", str(artifact),
                 "-extract", "/EFI/BOOT/fixture.bin", str(extracted)],
                check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
            )
            assert extracted.read_bytes() == bytes(range(256))
        guarded = work / "build" / "test-media" / cases[0][1]
        guarded.unlink()
        denied = subprocess.run(
            ["make", "-f", str(root / "Makefile"), "-o", "build/infinity-x86_64.img",
             "-o", "x86-native-speech-parity", f"build/test-media/{cases[0][1]}"],
            cwd=work,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            env={key: value for key, value in os.environ.items()
                 if key != "INFINITY_ISO_BUILD_AUTHORITY"},
        )
        assert denied.returncode != 0 and not guarded.exists()
        invalid = work / "builds" / "Invalid.iso"
        invalid.parent.mkdir(parents=True)
        invalid.write_bytes((work / "build" / "test-media" / cases[1][1]).read_bytes())
        rejected = subprocess.run(
            ["python3", str(root / "tools/full-bundle-iso-test.py"), str(invalid)],
            cwd=root,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        assert rejected.returncode != 0
        invalid.unlink()
        assert not list((work / "build").rglob("*.iso"))
        assert not list((work / "builds").glob("*.iso"))


if __name__ == "__main__":
    main()
