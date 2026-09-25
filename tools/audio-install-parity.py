"""Check the actual installed kernel and audio-discovering loader packaged in ARM media."""
from pathlib import Path
import struct
import subprocess
import sys

# ------------------------=
# FUNC: main
# DESC: Compares binary installed kernel and loader payloads instead of accepting source or diagnostic strings.
# ------------------=
def main():
    root = Path(__file__).resolve().parent.parent
    installed = (root / "build/aarch64/installed-kernel.elf").read_bytes()
    assert installed[:6] == b"\x7fELF\x02\x01" and int.from_bytes(installed[18:20], "little") == 183
    # Check actual immutable binary speech models, not source/log text or symbols.
    speech_root = root / "build/voice-pocketsphinx-src/model/en-us"
    model_bytes = 0
    for relative in ("en-us.lm.bin", "en-us/mdef", "en-us/means", "en-us/variances", "en-us/sendump", "en-us/transition_matrices"):
        data = (speech_root / relative).read_bytes()
        assert len(data) > 0 and installed.find(data) >= 0
        model_bytes += len(data)
    iso = Path(sys.argv[1]) if len(sys.argv) == 2 else root / "builds/InfinityOS-aarch64.iso"
    # Decode the ISO's El Torito catalog, then address its FAT image directly.
    with iso.open("rb") as file:
        file.seek(17 * 2048)
        record = file.read(2048)
        assert record[0] == 0 and record[1:6] == b"CD001"
        catalog_sector = struct.unpack_from("<I", record, 71)[0]
        file.seek(catalog_sector * 2048)
        catalog = file.read(64)
        assert catalog[30:32] == b"\x55\xaa" and catalog[32] == 0x88
        esp_offset = struct.unpack_from("<I", catalog, 40)[0] * 2048
    media = str(iso) + "@@" + str(esp_offset)
    part_size = 512 * 1024 * 1024
    for part, offset in enumerate(range(0, len(installed), part_size)):
        payload = subprocess.check_output(["mtype", "-i", media, f"::/EFI/INFINITY/PAYLOAD/P1-{part:03}.BIN"])
        assert payload == installed[offset:offset + part_size]
    loader = (root / "build/aarch64/BOOTAA64.EFI").read_bytes()
    for image in (str(root / "build/aarch64/installed-esp.img"), media):
        actual = subprocess.check_output(["mtype", "-i", image, "::/EFI/BOOT/BOOTAA64.EFI"])
        assert actual == loader
    print({"installed_kernel_bytes": len(installed), "loader_bytes": len(loader), "speech_model_bytes": model_bytes, "audio_kernel_loader_parity": True})

if __name__ == "__main__":
    main()
