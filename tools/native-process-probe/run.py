"""Real user-mode execution tests; not installed compiler/window acceptance."""
from pathlib import Path
import argparse
import json
import os
import shutil
import struct
import subprocess

ROOT = Path(__file__).resolve().parents[2]

# ------------------------=
# FUNC: main
# DESC: Builds and boots one disposable x86 VM, asserting binary privilege/fault/preemption outcomes.
# ------------------=
def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--arch", choices=["x86_64", "aarch64"], default="x86_64")
    parser.add_argument("--gic", choices=[2, 3], type=int, default=2)
    args = parser.parse_args()
    arm = args.arch == "aarch64"
    target = "aarch64-unknown-none-softfloat" if arm else "x86_64-unknown-none"
    work = ROOT / "build/native-process-probe" / (args.arch + (f"-gic{args.gic}" if arm else ""))
    work.mkdir(parents=True, exist_ok=True)
    clang = "/opt/homebrew/opt/llvm/bin/clang"
    linker = "/opt/homebrew/opt/lld/bin/ld.lld"
    subprocess.run([clang, f"--target={args.arch}-none-elf", "-ffreestanding", "-fno-stack-protector",
        "-fno-unwind-tables", "-fno-asynchronous-unwind-tables", "-fPIE",
        "-mno-outline-atomics" if arm else "-mno-red-zone",
        "-Isdk/c/include", "-Os", "-c", "tools/native-process-probe/user.c", "-o", str(work / "user.o")], check=True)
    subprocess.run([linker, "-static", "-pie", "--strip-all", "--no-dynamic-linker",
        "--build-id=none", "-z", "max-page-size=4096", "-T", "sdk/c/application.ld",
        str(work / "user.o"), "-o", str(work / "user.elf")], check=True)
    env = dict(os.environ, RUSTC_BOOTSTRAP="1", CARGO_TARGET_DIR=str(work / "cargo"),
        INFINITY_PROCESS_IMAGE=str(work / "user.elf"), INFINITY_PROCESS_GIC=str(args.gic))
    subprocess.run(["cargo", "build", "--release", "-Z", "build-std=core", "--target",
        target, "--manifest-path", "tools/native-process-probe/Cargo.toml"], env=env, check=True)
    subprocess.run([linker, "-nostdlib", "-static", "-T",
        "tools/native-process-probe/arm.ld" if arm else "linker/x86_64.ld", "-o", str(work / "probe.elf"),
        str(work / "cargo" / target / "release/libinfinity_native_process_probe.a")], check=True)
    if arm:
        with (work / "qemu.log").open("wb") as log:
            subprocess.run(["qemu-system-aarch64", "-machine", f"virt,gic-version={args.gic}", "-cpu", "max",
                "-m", "512M", "-display", "none", "-monitor", "none", "-serial", f"file:{work / 'result.bin'}",
                "-kernel", str(work / "probe.elf")], stdout=log, stderr=subprocess.STDOUT, check=True, timeout=30)
        data = (work / "result.bin").read_bytes()
        assert len(data) == 11 * 32 + 8, data.hex()
        assert struct.unpack_from("<Q", data, 11 * 32)[0] == 0x49534f4c41544544
        rows = list(struct.iter_unpack("<4Q", data[:-8]))
        assert [r[2] >> 26 if r[1] == 0 else r[2] for r in rows] == [21, 36, 21, 32, 21, 30, 21, 0, 21, 21, 21]
        proof = dict(architecture=args.arch, hardware_user_mode=True, supervisor_write_blocked=True,
            nx_enforced=True, infinite_loop_preempted=True, privileged_instruction_blocked=True,
            memory_reclaimed=True, kernel_survived_each_case=True, repeated_resume_tls_simd=True, installed_acceptance=False, cases=rows)
        (work / "evidence.json").write_text(json.dumps(proof, indent=2) + "\n")
        print(json.dumps(proof))
        return
    esp = work / "esp"
    (esp / "EFI/BOOT").mkdir(parents=True, exist_ok=True)
    (esp / "EFI/INFINITY").mkdir(parents=True, exist_ok=True)
    shutil.copyfile(ROOT / "build/x86_64/BOOTX64.EFI", esp / "EFI/BOOT/BOOTX64.EFI")
    shutil.copyfile(work / "probe.elf", esp / "EFI/INFINITY/KERNEL.ELF")
    with (work / "qemu.log").open("wb") as log:
        run = subprocess.run(["qemu-system-x86_64", "-machine", "pc", "-cpu", "max", "-m", "1G",
            "-drive", "if=pflash,format=raw,readonly=on,file=/opt/homebrew/share/qemu/edk2-x86_64-code.fd",
            "-drive", f"format=raw,file=fat:rw:{esp}", "-boot", "order=c", "-net", "none",
            "-device", "isa-debug-exit,iobase=0xf4,iosize=0x04", "-debugcon", f"file:{work / 'result.bin'}",
            "-display", "none", "-serial", "stdio", "-no-reboot"], stdout=log, stderr=subprocess.STDOUT, timeout=60)
    data = (work / "result.bin").read_bytes()
    assert run.returncode == 33, (run.returncode, data.hex(), (work / "qemu.log").read_text()[-2000:])
    rows = list(struct.iter_unpack("<4Q", data))
    assert len(rows) == 11
    assert [row[1] for row in rows] == [128, 14, 128, 14, 128, 32, 128, 13, 128, 128, 128]
    proof = dict(hardware_user_mode=True, supervisor_write_blocked=True, nx_enforced=True,
        infinite_loop_preempted=True, privileged_io_blocked=True, memory_reclaimed=True,
        kernel_survived_each_case=True, repeated_resume_tls_simd=True, installed_acceptance=False, cases=rows)
    (work / "evidence.json").write_text(json.dumps(proof, indent=2) + "\n")
    print(json.dumps(proof))

if __name__ == "__main__":
    main()
