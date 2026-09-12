"""Fast native execution bootstrap: NOT an installed-system or native-compiler acceptance test."""
import json
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]

# ------------------------=
# FUNC: main
# DESC: Boots the trusted C/ObjectStore probe and asserts its binary VM exit status.
# ------------------=
def main():
    with tempfile.TemporaryDirectory(prefix="infinity-native-c-") as temporary:
        work = Path(temporary)
        volume = work / "boot-volume"
        (volume / "EFI/BOOT").mkdir(parents=True)
        (volume / "EFI/INFINITY").mkdir(parents=True)
        shutil.copyfile(ROOT / "build/x86_64/BOOTX64.EFI", volume / "EFI/BOOT/BOOTX64.EFI")
        shutil.copyfile(ROOT / "build/native-c/probe.elf", volume / "EFI/INFINITY/KERNEL.ELF")
        with (work / "qemu.log").open("wb") as log:
            process = subprocess.Popen([
                "qemu-system-x86_64", "-machine", "pc", "-m", "512M",
                "-drive", "if=pflash,format=raw,readonly=on,file=/opt/homebrew/share/qemu/edk2-x86_64-code.fd",
                "-drive", f"format=raw,file=fat:rw:{volume}", "-boot", "order=c", "-net", "none",
                "-device", "isa-debug-exit,iobase=0xf4,iosize=0x04",
                "-display", "none", "-serial", "stdio", "-no-reboot"], stdout=log, stderr=subprocess.STDOUT)
            try:
                result = process.wait(timeout=45)
            finally:
                if process.poll() is None:
                    process.kill()
                    process.wait()
        proof = {"guest_exit_code": result, "cross_compiled_with_clang": True,
                 "object_store_round_trip": result == 33, "trusted_native_app_execution": result == 33,
                 "on_device_compiler": False, "hardware_isolation": False, "installed_acceptance": False}
        (ROOT / "build/native-c/proof.json").write_text(json.dumps(proof, indent=2) + "\n")
        if result != 33:
            print((work / "qemu.log").read_text(errors="replace")[-3000:])
            raise SystemExit(f"Native C probe failed with exit code {result}")
        print(json.dumps(proof))

if __name__ == "__main__":
    main()
