"""Fast native execution bootstrap: NOT an installed-system or native-compiler acceptance test."""
import json
from pathlib import Path
import shutil
import subprocess
import tempfile
import argparse

ROOT = Path(__file__).resolve().parents[2]

# ------------------------=
# FUNC: main
# DESC: Boots the trusted C/ObjectStore probe and asserts its binary VM exit status.
# ------------------=
def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--locale", action="store_true", help="Run the linked Newlib/libc++ locale probe")
    options = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix="infinity-native-c-") as temporary:
        work = Path(temporary)
        volume = work / "boot-volume"
        (volume / "EFI/BOOT").mkdir(parents=True)
        (volume / "EFI/INFINITY").mkdir(parents=True)
        shutil.copyfile(ROOT / "build/x86_64/BOOTX64.EFI", volume / "EFI/BOOT/BOOTX64.EFI")
        kernel = "locale-probe.elf" if options.locale else "probe.elf"
        shutil.copyfile(ROOT / "build/native-c" / kernel, volume / "EFI/INFINITY/KERNEL.ELF")
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
                 "native_c_stdio_object_io": result == 33,
                 "native_serial_sync_abi": result == 33,
                 "on_device_compiler": False, "hardware_isolation": False, "installed_acceptance": False}
        if options.locale:
            proof = {"guest_exit_code": result, "native_newlib_ctype": result == 33,
                     "native_libcxx_regex_classes": result == 33,
                     "on_device_compiler": False, "installed_acceptance": False}
        report = "locale-proof.json" if options.locale else "proof.json"
        (ROOT / "build/native-c" / report).write_text(json.dumps(proof, indent=2) + "\n")
        if result != 33:
            print((work / "qemu.log").read_text(errors="replace")[-3000:])
            raise SystemExit(f"Native C probe failed with exit code {result}")
        print(json.dumps(proof))

if __name__ == "__main__":
    main()
