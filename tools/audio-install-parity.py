"""Check the actual installed kernel and audio-discovering loader packaged in ARM media."""
from pathlib import Path
import mmap
import subprocess

# ------------------------=
# FUNC: main
# DESC: Compares binary installed kernel and loader payloads instead of accepting source or diagnostic strings.
# ------------------=
def main():
    root = Path(__file__).resolve().parent.parent
    installed = (root / "build/aarch64/installed-kernel.elf").read_bytes()
    assert installed[:6] == b"\x7fELF\x02\x01" and int.from_bytes(installed[18:20], "little") == 183
    with (root / "build/aarch64/kernel.elf").open("rb") as file:
        with mmap.mmap(file.fileno(), 0, access=mmap.ACCESS_READ) as installer:
            assert installer.find(installed) >= 0
    loader = (root / "build/aarch64/BOOTAA64.EFI").read_bytes()
    for image in ("build/aarch64/installed-esp.img", "build/infinity-aarch64.img"):
        actual = subprocess.check_output(["mtype", "-i", str(root / image), "::/EFI/BOOT/BOOTAA64.EFI"])
        assert actual == loader
    print({"installed_kernel_bytes": len(installed), "loader_bytes": len(loader), "audio_kernel_loader_parity": True})

if __name__ == "__main__":
    main()
