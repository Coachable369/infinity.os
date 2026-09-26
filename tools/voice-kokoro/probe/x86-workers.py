"""Guest behavioral proof of native x86 AP startup after firmware exit."""
from pathlib import Path
import os
import subprocess
import shutil

ROOT = Path(__file__).resolve().parents[3]

# ------------------------=
# FUNC: main
# DESC: Compiles the production bootstrap into an isolated EFI guest and asserts its binary result.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Use ./build-kit run python3 tools/voice-kokoro/probe/x86-workers.py")
    work = ROOT / "build/voice-kokoro/x86-workers"
    esp = work / "esp/EFI/BOOT"
    esp.mkdir(parents=True, exist_ok=True)
    subprocess.run(["/opt/homebrew/opt/llvm/bin/clang", "--target=x86_64-pc-windows-msvc",
                    "-ffreestanding", "-fshort-wchar", "-fno-stack-protector", "-mno-red-zone",
                    "-O2", "-Wall", "-Wextra", "-Werror", "-c",
                    str(ROOT / "tools/voice-kokoro/probe/x86-workers.c"), "-o", str(work / "probe.obj")], check=True)
    subprocess.run(["/opt/homebrew/opt/lld/bin/lld-link", "/subsystem:efi_application", "/entry:efi_main",
                    "/nodefaultlib", "/machine:x64", "/out:" + str(esp / "BOOTX64.EFI"),
                    str(work / "probe.obj"), str(ROOT / "build/x86_64/handoff.obj"),
                    str(ROOT / "build/x86_64/workers.obj")], check=True)
    shutil.copyfile("/opt/homebrew/share/qemu/edk2-i386-vars.fd", work / "vars.fd")
    result = subprocess.run(["qemu-system-x86_64", "-machine", "q35", "-accel", "tcg", "-smp", "4",
                             "-m", "512M", "-drive", "if=pflash,format=raw,readonly=on,file=/opt/homebrew/share/qemu/edk2-x86_64-code.fd",
                             "-drive", "if=pflash,format=raw,file=" + str(work / "vars.fd"),
                             "-drive", "format=raw,file=fat:rw:" + str(work / "esp"),
                             "-device", "isa-debug-exit,iobase=0xf4,iosize=0x04", "-display", "none",
                             "-serial", "file:" + str(work / "serial.log"),
                             "-debugcon", "file:" + str(work / "stages.bin"),
                             "-monitor", "none", "-no-reboot"], timeout=45)
    if result.returncode != 33:
        raise RuntimeError(f"Native worker guest failed with binary exit status {result.returncode}")
    print("Native x86 workers: three independent APs, floating point, firmware-exit and repeat-start checks passed")

if __name__ == "__main__":
    main()
