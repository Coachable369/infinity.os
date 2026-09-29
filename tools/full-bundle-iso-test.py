"""Reject any published InfinityOS ISO that omits the complete native model bundle."""

from __future__ import annotations

import hashlib
from pathlib import Path
import subprocess
import sys
import tempfile


ROOT = Path(__file__).resolve().parent.parent
CHUNK_BYTES = 512 * 1024 * 1024
MODELS = (
    (3, 2_147_023_008, "9ed150d4367e68df0ac8e1540f6ddc65b42d0ee26378329d1ecbca60f93fc5f8"),
    (4, 2_019_373_888, "91776fe0f6cd7483d9d5e06162fdd1f8f0262c15ced269791b4d96a655e8a5a2"),
)


# ------------------------=
# FUNC: stream_member
# DESC: Streams one FAT member into a sink without materializing multi-gigabyte payloads in memory.
# ------------------=
def stream_member(image: Path, member: str, sink) -> tuple[bool, int, str]:
    process = subprocess.Popen(
        ["mtype", "-i", str(image), member],
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
    )
    assert process.stdout is not None
    digest = hashlib.sha256()
    total = 0
    while True:
        block = process.stdout.read(1024 * 1024)
        if not block:
            break
        sink.write(block)
        digest.update(block)
        total += len(block)
    return process.wait() == 0, total, digest.hexdigest()


# ------------------------=
# FUNC: reassemble_installed_esp
# DESC: Reassembles the installer-owned System Generation image from the live ISO payload.
# ------------------=
def reassemble_installed_esp(live_esp: Path, installed_esp: Path) -> None:
    parts = 0
    with installed_esp.open("wb") as target:
        while True:
            member = f"::/EFI/INFINITY/PAYLOAD/P0-{parts:03}.BIN"
            present, length, _ = stream_member(live_esp, member, target)
            if not present:
                break
            if length == 0 or (parts > 0 and length > CHUNK_BYTES):
                raise RuntimeError(f"invalid installed-image shard {parts}")
            parts += 1
    if parts == 0:
        raise RuntimeError("ISO has no installed System Generation payload")


# ------------------------=
# FUNC: verify_model
# DESC: Verifies one complete native model by byte count and full SHA-256 from the installed payload.
# ------------------=
def verify_model(installed_esp: Path, slot: int, expected_bytes: int, expected_hash: str) -> None:
    digest = hashlib.sha256()
    total = 0
    part = 0
    while total < expected_bytes:
        member = f"::/EFI/INFINITY/PAYLOAD/P{slot}-{part:03}.BIN"
        process = subprocess.Popen(
            ["mtype", "-i", str(installed_esp), member],
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
        )
        assert process.stdout is not None
        length = 0
        while True:
            block = process.stdout.read(1024 * 1024)
            if not block:
                break
            digest.update(block)
            length += len(block)
        if process.wait() != 0 or length == 0:
            raise RuntimeError(f"missing native model slot {slot} shard {part}")
        total += length
        part += 1
    if total != expected_bytes or digest.hexdigest() != expected_hash:
        raise RuntimeError(f"native model slot {slot} failed full-content verification")


# ------------------------=
# FUNC: verify_iso
# DESC: Proves that a published ISO embeds both verified models in its fresh-install System Generation.
# ------------------=
def verify_iso(iso: Path) -> None:
    if not iso.is_file() or iso.stat().st_size == 0:
        raise RuntimeError(f"missing ISO: {iso}")
    temporary_root = ROOT / "build" / "tmp"
    temporary_root.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="full-bundle-", dir=temporary_root) as directory:
        work = Path(directory)
        live_esp = work / "live-efi.img"
        installed_esp = work / "installed-efi.img"
        subprocess.run(
            ["xorriso", "-osirrox", "on", "-indev", str(iso), "-extract", "/efi.img", str(live_esp)],
            check=True,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        reassemble_installed_esp(live_esp, installed_esp)
        for model in MODELS:
            verify_model(installed_esp, *model)
    print(f"PASS: complete native model bundle verified in {iso}")


# ------------------------=
# FUNC: main
# DESC: Applies the mandatory full-bundle gate to every requested ISO artifact.
# ------------------=
def main() -> None:
    images = [Path(value).resolve() for value in sys.argv[1:]]
    if not images:
        raise SystemExit("usage: full-bundle-iso-test.py <image.iso> [...]")
    for image in images:
        verify_iso(image)


if __name__ == "__main__":
    main()
