"""Reject incomplete native bundles or stale live/installed Precision artwork in an ISO."""

from __future__ import annotations

import hashlib
from pathlib import Path
import subprocess
import struct
import sys
import tempfile
from kernel_layout import HeaderSink, validate as validate_kernel_layout
from iso_fat import fat_image


ROOT = Path(__file__).resolve().parent.parent
CHUNK_BYTES = 512 * 1024 * 1024
MODELS = (
    (3, 2_147_023_008, "9ed150d4367e68df0ac8e1540f6ddc65b42d0ee26378329d1ecbca60f93fc5f8"),
    (4, 2_019_373_888, "91776fe0f6cd7483d9d5e06162fdd1f8f0262c15ced269791b4d96a655e8a5a2"),
)
MEDIA = {
    "InfinityOS-aarch64.iso": ("aarch64", "virtualbox"),
    "InfinityOS-x86_64.iso": ("x86_64", "virtualbox"),
    "InfinityOS-aarch64-qemu-models.bootmedia": ("aarch64", "qemu"),
}
PRECISION_ATLAS = ROOT / "assets/ui-design-kit/default/eclipse-precision-v1.rgba"
INSTALLED_DESIGN_KIT = "::/EFI/InfinityOS/InfinityUI/DesignKit/Default/"


class BinaryMatchSink(HeaderSink):
    # ------------------------=
    # FUNC: __init__
    # DESC: Retains a bounded ELF prefix and at most one artwork-sized boundary window, never the kernel image.
    # ------------------=
    def __init__(self, expected: bytes):
        super().__init__()
        if not expected:
            raise ValueError("empty binary artwork is not a valid parity target")
        self.expected = expected
        self.tail = b""
        self.matched = False

    # ------------------------=
    # FUNC: write
    # DESC: Matches exact binary artwork across read blocks and payload shards, releasing the window after a match.
    # ------------------=
    def write(self, data):
        super().write(data)
        if not self.matched:
            window = self.tail + data
            self.matched = self.expected in window
            self.tail = b"" if self.matched or len(self.expected) == 1 else window[-(len(self.expected) - 1):]
        return len(data)

    # ------------------------=
    # FUNC: require_match
    # DESC: Rejects a missing or byte-modified atlas without interpreting source code or diagnostic prose.
    # ------------------=
    def require_match(self):
        if not self.matched:
            raise RuntimeError("kernel does not contain the exact current Precision atlas")


# ------------------------=
# FUNC: media_target
# DESC: Resolves only declared release and isolated test-media targets without inferring unknown architectures.
# ------------------=
def media_target(iso: Path) -> tuple[str, str]:
    try:
        return MEDIA[iso.name.removesuffix(".partial")]
    except KeyError as error:
        raise RuntimeError(f"unrecognized media target: {iso.name}") from error


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
# FUNC: verify_boot_loader
# DESC: Proves live and freshly installed boot paths contain the exact current native worker adapter.
# ------------------=
def verify_boot_loader(iso: Path, live_esp: Path, installed_esp: Path, work: Path) -> None:
    # The publisher validates before its atomic rename. Apply identical byte
    # checks to that private candidate and the final published filename.
    architecture, _ = media_target(iso)
    name = "BOOTAA64.EFI" if architecture == "aarch64" else "BOOTX64.EFI"
    expected = (ROOT / "build" / architecture / name).read_bytes()
    if not expected:
        raise RuntimeError("native boot loader is empty")
    if architecture == "aarch64":
        pe = struct.unpack_from("<I", expected, 0x3C)[0]
        assert expected[pe:pe+4] == b"PE\0\0"
        assert struct.unpack_from("<H", expected, pe+4)[0] == 0xAA64
        optional = pe + 24
        base = struct.unpack_from("<Q", expected, optional+24)[0]
        size = struct.unpack_from("<I", expected, optional+56)[0]
        assert 0x8000000 <= base and size > 0 and base + size <= 0x10000000
    if architecture == "x86_64":
        pe = struct.unpack_from("<I", expected, 0x3C)[0]
        assert expected[pe:pe+4] == b"PE\0\0"
        assert struct.unpack_from("<H", expected, pe+4)[0] == 0x8664
        optional = pe + 24
        assert struct.unpack_from("<H", expected, optional)[0] == 0x20B
        base = struct.unpack_from("<Q", expected, optional+24)[0]
        size = struct.unpack_from("<I", expected, optional+56)[0]
        assert 0x100000 <= base and size > 0 and base + size <= 0x4000000
    for image, member in (
        (live_esp, f"::/EFI/BOOT/{name}"),
        (installed_esp, f"::/EFI/BOOT/{name}"),
        (installed_esp, "::/EFI/InfinityOS/infinity.efi"),
    ):
        actual = subprocess.check_output(["mtype", "-i", str(image), member])
        if actual != expected:
            raise RuntimeError(f"stale or mismatched native boot loader: {member}")
    external = work / name
    subprocess.run(
        ["xorriso", "-osirrox", "on", "-indev", str(iso), "-extract", f"/EFI/BOOT/{name}", str(external)],
        check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
    )
    if external.read_bytes() != expected:
        raise RuntimeError("ISO boot loader differs from the installed boot loader")


# ------------------------=
# FUNC: verify_precision_artwork
# DESC: Checks current artwork in both final-media kernels and the freshly installed ESP, excluding the removed source sheet.
# ------------------=
def verify_precision_artwork(live_esp: Path, installed_esp: Path,
                             architecture: str, platform: str) -> None:
    atlas = PRECISION_ATLAS.read_bytes()
    for installed in (False, True):
        sink = BinaryMatchSink(atlas)
        part = 0
        previous_length = CHUNK_BYTES
        while True:
            member = (f"::/EFI/INFINITY/PAYLOAD/P1-{part:03}.BIN" if installed
                      else "::/EFI/INFINITY/KERNEL.ELF")
            present, length, _ = stream_member(live_esp, member, sink)
            if not present:
                if part == 0 or length:
                    raise RuntimeError(f"missing or unreadable kernel artifact: {member}")
                break
            if not length or (installed and (length > CHUNK_BYTES or previous_length != CHUNK_BYTES)):
                raise RuntimeError(f"invalid kernel artifact extent: {member}")
            previous_length = length
            part += 1
            if not installed:
                break
        validate_kernel_layout(sink.header, platform)
        if struct.unpack_from("<H", sink.header, 18)[0] != {"aarch64": 183, "x86_64": 62}[architecture]:
            raise RuntimeError("kernel architecture does not match the selected media target")
        sink.require_match()
    present, length, digest = stream_member(
        installed_esp, INSTALLED_DESIGN_KIT + PRECISION_ATLAS.name, HeaderSink())
    if not present or length != len(atlas) or digest != hashlib.sha256(atlas).hexdigest():
        raise RuntimeError("installed design kit does not contain the exact current Precision atlas")
    deprecated = subprocess.run(
        ["mtype", "-i", str(installed_esp), INSTALLED_DESIGN_KIT + "eclipse-precision-v1-source.png"],
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
    )
    if deprecated.returncode == 0:
        raise RuntimeError("installed design kit still contains the removed Precision source sheet")


# ------------------------=
# FUNC: verify_iso
# DESC: Proves that final media embeds verified models and exact live/installed Precision artwork.
# ------------------=
def verify_iso(iso: Path) -> None:
    architecture, platform = media_target(iso)
    if not iso.is_file() or iso.stat().st_size == 0:
        raise RuntimeError(f"missing ISO: {iso}")
    temporary_root = ROOT / "build" / "tmp"
    temporary_root.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="full-bundle-", dir=temporary_root) as directory:
        work = Path(directory)
        # Read the actual candidate's FAT extent in place. Extracting it again
        # duplicates the entire ISO while the installed image is reassembled.
        live_esp = fat_image(iso)
        installed_esp = work / "installed-efi.img"
        reassemble_installed_esp(live_esp, installed_esp)
        verify_precision_artwork(live_esp, installed_esp, architecture, platform)
        verify_boot_loader(iso, live_esp, installed_esp, work)
        for model in MODELS:
            verify_model(installed_esp, *model)
    print(f"PASS: complete native model bundle and live/installed Precision artwork verified in {iso}")


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
