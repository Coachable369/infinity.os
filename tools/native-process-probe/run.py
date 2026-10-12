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
# FUNC: compiler_result
# DESC: Checks structured compiler exits, generated ELF artifacts, actual child results and memory reclamation independently of diagnostic text.
# ------------------=
def compiler_result(data, work, args):
    assert len(data) >= 56, data.hex()
    marker, status, written, calls, free, initial, diagnostic_length = struct.unpack_from("<7Q", data)
    output_offset = 56 + (diagnostic_length + 7) // 8 * 8
    assert diagnostic_length <= 65536 and len(data) >= output_offset + 8
    output_length, = struct.unpack_from("<Q", data, output_offset)
    assert output_length <= 65536 and len(data) == output_offset + 16 + (output_length + 7) // 8 * 8
    app_status, = struct.unpack_from("<Q", data, len(data) - 8)
    output = data[output_offset + 8:output_offset + 8 + output_length]
    diagnostic = data[56:56 + diagnostic_length]
    (work / "compiler-console.log").write_bytes(diagnostic)
    if not (args.compile_source or args.link_object) or status:
        print(diagnostic.decode("utf-8", errors="replace"))
    assert marker == 0x434c414e47454c30 and status == 0 and written > 0 and calls > 1 and free == initial
    proof = dict(architecture=args.arch, real_native_toolchain=True, hardware_user_mode=True,
        exit_status=status, console_bytes=written, service_calls=calls, memory_reclaimed=True,
        compiled_source=args.compile_source, installed_acceptance=False)
    machine = 183 if args.arch == "aarch64" else 62
    if args.compile_source:
        assert output[:6] == b"\x7fELF\x02\x01"
        assert struct.unpack_from("<HH", output, 16) == (1, machine)
        (work / "compiled.o").write_bytes(output)
    if args.link_object:
        assert output[:6] == b"\x7fELF\x02\x01"
        assert struct.unpack_from("<HH", output, 16) == (3, machine)
        (work / "linked.elf").write_bytes(output)
        proof["native_linked"] = True
        assert app_status == 54
        proof["guest_linked_app_exit"] = app_status
    stage = "link" if args.link_object else "compile" if args.compile_source else "version"
    (work / f"{stage}-evidence.json").write_text(json.dumps(proof, indent=2) + "\n")
    print(json.dumps(proof))

# ------------------------=
# FUNC: main
# DESC: Builds and boots one disposable x86 VM, asserting binary privilege/fault/preemption outcomes.
# ------------------=
def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--arch", choices=["x86_64", "aarch64"], default="x86_64")
    parser.add_argument("--gic", choices=[2, 3], type=int, default=2)
    parser.add_argument("--split-eoi", action="store_true")
    parser.add_argument("--firmware", action="store_true", help="Use production ARM UEFI discovery and handoff")
    parser.add_argument("--memory", choices=["512M", "1G"], default="1G", help="Firmware guest RAM for low-memory boot coverage")
    parser.add_argument("--compiler", type=Path, help="Run a native Clang/LLD image in isolated hardware user mode")
    parser.add_argument("--compile-source", action="store_true", help="Compile real C from a bounded native input stream to an ELF object")
    parser.add_argument("--link-object", type=Path, help="Run native LLD with this object exposed as a read-only guest fixture")
    args = parser.parse_args()
    arm = args.arch == "aarch64"
    assert not args.firmware or (arm and not args.compiler and not args.split_eoi)
    target = "aarch64-unknown-none-softfloat" if arm else "x86_64-unknown-none"
    work = ROOT / "build/native-process-probe" / (args.arch + (f"-gic{args.gic}" if arm else "") + ("-firmware" if args.firmware else "") + ("-compiler" if args.compiler else ""))
    work.mkdir(parents=True, exist_ok=True)
    clang = "/opt/homebrew/opt/llvm/bin/clang"
    linker = "/opt/homebrew/opt/lld/bin/ld.lld"
    c_flags = [clang, f"--target={args.arch}-none-elf", "-ffreestanding", "-fno-stack-protector",
        "-fno-unwind-tables", "-fno-asynchronous-unwind-tables", "-fPIE",
        "-mno-outline-atomics" if arm else "-mno-red-zone",
        "-Isdk/c/include", "-Isdk/compiler/include", "-isystem",
        str(ROOT / ("build/native-c/aarch64/sysroot/aarch64-unknown-elf/include" if arm else
                    "build/native-c/sysroot/x86_64-unknown-elf/include")), "-Os"]
    # Release builds already prepare the same pinned Newlib for native speech.
    # Reuse its real headers/configuration when the optional compiler build is absent.
    newlib = ROOT / "build/newlib-4.6.0.20260123/newlib/libc/include"
    newlib_config = ROOT / f"build/voice-newlib-{args.arch}/{args.arch}-none-elf/newlib"
    if newlib.exists() and (newlib_config / "newlib.h").exists():
        c_flags += ["-isystem", str(newlib_config), "-isystem", str(newlib)]
    subprocess.run(c_flags + ["-c", "tools/native-process-probe/user.c", "-o", str(work / "user.o")], check=True)
    subprocess.run(c_flags + ["-c", "sdk/compiler/isolated.c", "-o", str(work / "isolated.o")], check=True)
    subprocess.run([linker, "-static", "-pie", "--strip-all", "--no-dynamic-linker",
        "--build-id=none", "-z", "max-page-size=4096", "-T", "sdk/c/application.ld",
        str(work / "user.o"), str(work / "isolated.o"), "-o", str(work / "user.elf")], check=True)
    env = dict(os.environ, RUSTC_BOOTSTRAP="1", CARGO_TARGET_DIR=str(work / "cargo"),
        INFINITY_PROCESS_IMAGE=str(work / "user.elf"), INFINITY_PROCESS_GIC=str(args.gic))
    env.pop("INFINITY_PROCESS_SPLIT_EOI", None)
    if args.split_eoi:
        assert arm and args.gic == 3
        env["INFINITY_PROCESS_SPLIT_EOI"] = "1"
    env.pop("INFINITY_COMPILER_PROBE", None)
    if args.compiler:
        env.update(INFINITY_COMPILER_PROBE="1", INFINITY_PROCESS_IMAGE=str(args.compiler.resolve()))
    env.pop("INFINITY_COMPILER_SOURCE", None)
    env.pop("INFINITY_LINK_INPUT", None)
    env["INFINITY_LINK_INPUT_BYTES"] = str((args.link_object or (work / "user.elf")).resolve())
    if args.link_object:
        assert args.compiler and not args.compile_source
        env["INFINITY_LINK_INPUT"] = "1"
    if args.compile_source:
        assert args.compiler
        env["INFINITY_COMPILER_SOURCE"] = "1"
    subprocess.run(["cargo", "build", "--release", "-Z", "build-std=core", "--target",
        target, "--manifest-path", "tools/native-process-probe/Cargo.toml"], env=env, check=True)
    subprocess.run([linker, "-nostdlib", "-static", "-T",
        "tools/native-process-probe/arm.ld" if arm else "linker/x86_64.ld", "-o", str(work / "probe.elf"),
        str(work / "cargo" / target / "release/libinfinity_native_process_probe.a")] +
        (["-e", "infinity_kernel_entry"] if args.firmware else []), check=True)
    if arm:
        boot_args=["-kernel",str(work / "probe.elf")]
        if args.firmware:
            esp=work / "esp"
            (esp / "EFI/BOOT").mkdir(parents=True,exist_ok=True)
            (esp / "EFI/INFINITY").mkdir(parents=True,exist_ok=True)
            shutil.copyfile(ROOT / "build/aarch64/BOOTAA64.EFI",esp / "EFI/BOOT/BOOTAA64.EFI")
            shutil.copyfile(work / "probe.elf",esp / "EFI/INFINITY/KERNEL.ELF")
            boot_args=["-bios","/opt/homebrew/share/qemu/edk2-aarch64-code.fd",
                "-drive",f"if=none,id=esp,format=raw,file=fat:rw:{esp}","-device","virtio-blk-pci,drive=esp","-net","none"]
        with (work / "qemu.log").open("wb") as log:
            subprocess.run(["qemu-system-aarch64", "-machine", f"virt,gic-version={args.gic}", "-cpu", "max",
                "-m", args.memory if args.firmware else "512M", "-display", "none", "-monitor", "none",
                "-qmp", f"unix:{work / 'monitor.sock'},server=on,wait=off", "-serial", f"file:{work / 'result.bin'}",
                *boot_args], stdout=log, stderr=subprocess.STDOUT, check=True, timeout=120 if args.compiler or args.firmware else 30)
        data = (work / "result.bin").read_bytes()
        if args.firmware:
            data=data[-(12*32+8):]  # Only the fixed binary result trailer is acceptance evidence.
        if args.compiler:
            compiler_result(data, work, args)
            return
        assert len(data) == 12 * 32 + 8, data.hex()
        assert struct.unpack_from("<Q", data, 12 * 32)[0] == 0x49534f4c41544544
        rows = list(struct.iter_unpack("<4Q", data[:-8]))
        assert [r[2] >> 26 if r[1] == 0 else r[2] for r in rows] == [21, 36, 21, 32, 21, 30, 21, 0, 21, 21, 21, 21]
        proof = dict(architecture=args.arch, firmware_controller_handoff=args.firmware, hardware_user_mode=True, supervisor_write_blocked=True,
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
            "-display", "none", "-serial", "stdio", "-no-reboot"], stdout=log, stderr=subprocess.STDOUT, timeout=120 if args.compiler else 60)
    data = (work / "result.bin").read_bytes()
    assert run.returncode == 33, (run.returncode, data.hex(), (work / "qemu.log").read_text()[-2000:])
    if args.compiler:
        compiler_result(data, work, args)
        return
    rows = list(struct.iter_unpack("<4Q", data))
    assert len(rows) == 12
    assert [row[1] for row in rows] == [128, 14, 128, 14, 128, 32, 128, 13, 128, 128, 128, 128]
    proof = dict(firmware_controller_handoff=True, hardware_user_mode=True, supervisor_write_blocked=True, nx_enforced=True,
        infinite_loop_preempted=True, privileged_io_blocked=True, memory_reclaimed=True,
        kernel_survived_each_case=True, repeated_resume_tls_simd=True, installed_acceptance=False, cases=rows)
    (work / "evidence.json").write_text(json.dumps(proof, indent=2) + "\n")
    print(json.dumps(proof))

if __name__ == "__main__":
    main()
