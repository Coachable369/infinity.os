"""Build a minimal canonical ARM boot probe without model/installer payloads."""
from pathlib import Path
import subprocess
from kernel_layout import validate


# ------------------------=
# FUNC: main
# DESC: Packages the real installed ARM kernel and EFI loader for a disposable firmware allocation test.
# ------------------=
def main():
    root = Path(__file__).resolve().parents[1]
    output = root / "builds/validation/kernel-layout"
    output.mkdir(parents=True, exist_ok=True)
    kernel = root / "build/aarch64/installed-kernel.elf"
    with kernel.open("rb") as source:
        validate(source.read(65536))
    image = output / "efi.img"
    with image.open("wb") as target:
        target.truncate(1280 * 1024 * 1024)
    subprocess.run(["mformat", "-F", "-i", str(image), "::"], check=True)
    for directory in ("::/EFI", "::/EFI/BOOT", "::/EFI/INFINITY"):
        subprocess.run(["mmd", "-i", str(image), directory], check=True)
    subprocess.run(["mcopy", "-i", str(image), str(root / "build/aarch64/BOOTAA64.EFI"),
                    "::/EFI/BOOT/BOOTAA64.EFI"], check=True)
    subprocess.run(["mcopy", "-i", str(image), str(kernel), "::/EFI/INFINITY/KERNEL.ELF"], check=True)
    # Keep diagnostic media distinct from the two canonical release ISO names.
    subprocess.run(["xorriso", "-as", "mkisofs", "-R", "-e", "efi.img", "-no-emul-boot",
                    "-o", str(output / "kernel.bootmedia"), "-graft-points",
                    "efi.img=" + str(image)], check=True)


if __name__ == "__main__":
    main()
