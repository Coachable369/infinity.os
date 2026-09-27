"""Stage an experimental browser kernel for real desktop boot, not a release ISO."""
import os
from pathlib import Path
import shutil
import subprocess

# ------------------------=
# FUNC: main
# DESC: Creates private UEFI media from the linked native kernel and existing OS assets without replacing release products.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Run through build-kit")
    root = Path(__file__).resolve().parents[2]
    work = root / "build/servo-platform-probe/desktop"
    stage = work / "stage"
    boot = stage / "EFI/BOOT"
    system = stage / "EFI/INFINITY"
    boot.mkdir(parents=True, exist_ok=True)
    system.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(root / "build/aarch64/BOOTAA64.EFI", boot / "BOOTAA64.EFI")
    subprocess.run(["/opt/homebrew/opt/llvm/bin/llvm-objcopy", "--strip-debug",
        str(work.parent / "kernel-aarch64/qemu-kernel.elf"), str(system / "KERNEL.ELF")], check=True)
    for name in ("FONTS", "FONT-LICENSES", "INFINITYUI"):
        shutil.copytree(root / "build/fat-aarch64/EFI/INFINITY" / name, system / name, dirs_exist_ok=True)
    image = work / "desktop.img"
    with image.open("wb") as output:
        output.truncate(4 * 1024**3)
    subprocess.run(["mformat", "-F", "-i", str(image), "::"], check=True)
    subprocess.run(["mcopy", "-i", str(image), "-s", str(stage / "EFI"), "::"], check=True)
    print(image)

if __name__ == "__main__":
    main()
