"""Real user-mode execution tests; not installed compiler/window acceptance."""
from pathlib import Path
import argparse
import importlib.util
import json
import os
import shutil
import struct
import subprocess
import time

ROOT = Path(__file__).resolve().parents[2]

# ------------------------=
# FUNC: pip_result
# DESC: Validates per-process pip lifecycle exits, archive commits and memory reclamation using binary native receipts.
# ------------------=
def pip_result(data, work, args):
    destination = ROOT / "builds/validation/native-python" / args.arch
    run = destination / ("pip-probe-" + str(time.time_ns()))
    run.mkdir(parents=True)
    (run / "result.bin").write_bytes(data)
    proof = dict(architecture=args.arch, native_python_pip=False, hardware_user_mode=True,
                 stages=[], installed_acceptance=False, authenticated_user_isolation=False,
                 reboot_persistence=False)
    (run / "evidence.json").write_text(json.dumps(proof, indent=2) + "\n")
    assert len(data) >= 24
    marker, version, count = struct.unpack_from("<3Q", data)
    assert marker == 0x5049504C49464531 and version == 1 and count == (1 if args.python_pip_imports else 13)
    offset = 24
    stages = []
    names = ["version", "install", "import", "metadata", "reject-platform", "reject-traversal",
             "persisted-import", "isolated-namespace", "upgrade", "upgraded-import", "reject-native", "uninstall", "removed"]
    if args.python_pip_imports:
        names = ["install-uninstall-imports"]
    previous_hash = None
    for index in range(count):
        assert len(data) >= offset + 80
        stage, status, visits, before, after, archive_bytes, digest, committed, stdout, stderr = struct.unpack_from("<10Q", data, offset)
        offset += 80
        assert stage == index and visits > 0 and before == after
        assert archive_bytes > 0 and stdout <= 65536 and stderr <= 65536
        output_size = (stdout + 7) // 8 * 8
        error_size = (stderr + 7) // 8 * 8
        assert len(data) >= offset + output_size + error_size
        output = data[offset:offset + stdout]
        (work / (names[index] + "-stdout.log")).write_bytes(output)
        (run / (names[index] + "-stdout.log")).write_bytes(output)
        offset += output_size
        errors = data[offset:offset + stderr]
        (work / (names[index] + "-stderr.log")).write_bytes(errors)
        (run / (names[index] + "-stderr.log")).write_bytes(errors)
        offset += error_size
        proof["observed_stage"] = dict(name=names[index], exit_status=status, before_pages=before,
                                        after_pages=after, archive_bytes=archive_bytes,
                                        archive_fingerprint=digest, committed=bool(committed))
        proof["stages"] = stages
        (run / "evidence.json").write_text(json.dumps(proof, indent=2) + "\n")
        if index in (4, 5, 10):
            assert 0 < status < 256 and not committed and digest == previous_hash
        else:
            assert status == 0, (names[index], status, (work / (names[index] + "-stderr.log")).read_text(errors="replace"))
        if index in (1, 8, 11):
            assert committed == 1 and digest != previous_hash
        if index in (2, 3, 6, 7, 9, 12):
            assert not committed and digest == previous_hash
        previous_hash = digest
        stages.append(dict(name=names[index], exit_status=status, poll_visits=visits,
                           memory_reclaimed=True, committed=bool(committed), archive_bytes=archive_bytes,
                           archive_fingerprint=digest, stdout_bytes=stdout, stderr_bytes=stderr))
    assert data[offset:] == struct.pack("<Q", 0x504950434F4D504C)
    if args.python_pip_imports:
        proof.update(native_python_pip=False, install_uninstall_imports=True, stages=stages)
        (run / "evidence.json").write_text(json.dumps(proof, indent=2) + "\n")
        print(json.dumps(proof))
        return
    proof = dict(architecture=args.arch, native_python_pip=True, hardware_user_mode=True,
                 actual_upstream_pip=True, archive_remount_between_processes=True,
                 isolated_empty_namespace=True, memory_reclaimed=True, stages=stages,
                 installed_acceptance=False, authenticated_user_isolation=False,
                 reboot_persistence=False)
    (run / "evidence.json").write_text(json.dumps(proof, indent=2) + "\n")
    (destination / "pip-lifecycle.json").write_text(json.dumps(proof, indent=2) + "\n")
    (destination / "pip-lifecycle.bin").write_bytes(data)
    print(json.dumps(proof))

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
    assert marker == 0x434c414e47454c30 and calls > 1 and free == initial
    if args.python_validation:
        # The fixture normally prints and exits 54. Validating it must neither
        # execute those side effects nor misclassify a missing stdlib as syntax.
        assert output_length == 0
        if args.python_validation == "invalid":
            assert status == 65 and diagnostic_length > 0 and written > 0
        else:
            assert status == 0 and diagnostic_length == 0 and written == 0
    else:
        assert written > 0
    if args.reject_invalid_source:
        assert status != 0 and output_length == 0 and diagnostic_length > 0
    elif args.python_validation:
        pass
    elif args.python_script:
        assert status == 54
    else:
        assert status == 0
    proof = dict(architecture=args.arch, real_native_toolchain=True, hardware_user_mode=True,
        exit_status=status, console_bytes=written, poll_visits=calls, memory_reclaimed=True,
        compiled_source=args.compile_source, installed_acceptance=False)
    proof["interactive_input"] = args.interactive
    proof["native_python_script"] = args.python_script
    proof["native_python_extension"] = bool(args.python_extension)
    proof["native_python_validation"] = args.python_validation
    machine = 183 if args.arch == "aarch64" else 62
    if args.reject_invalid_source:
        proof["invalid_source_rejected"] = True
        proof["binary_output_bytes"] = output_length
    elif args.compile_source:
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
    stage = "python-validation-" + args.python_validation if args.python_validation else "python-extension" if args.python_extension else "python-script" if args.python_script else "invalid" if args.reject_invalid_source else "link" if args.link_object else "compile" if args.compile_source else "version"
    (work / f"{stage}-evidence.json").write_text(json.dumps(proof, indent=2) + "\n")
    if args.python_script:
        evidence = ROOT / "builds/validation/native-python" / args.arch
        evidence.mkdir(parents=True, exist_ok=True)
        (evidence / f"{stage}.json").write_text(json.dumps(proof, indent=2) + "\n")
        (evidence / f"{stage}.bin").write_bytes(data)
    if args.interactive:
        evidence = ROOT / "builds/validation/native-apps" / (args.arch + "-interactive")
        evidence.mkdir(parents=True, exist_ok=True)
        (evidence / f"{stage}.json").write_text(json.dumps(proof, indent=2) + "\n")
    print(json.dumps(proof))

# ------------------------=
# FUNC: virtualbox_result
# DESC: Validates fixed binary hardware-isolation records captured from the repository-owned VirtualBox ARM guest.
# ------------------=
def virtualbox_result(path):
    data = path.read_bytes()
    trailer = data[-(12 * 32 + 8):]
    assert len(trailer) == 12 * 32 + 8
    rows = list(struct.iter_unpack("<4Q", trailer[:-8]))
    assert struct.unpack_from("<Q", trailer, 12 * 32)[0] == 0x49534F4C41544544
    assert [r[2] >> 26 if r[1] == 0 else r[2] for r in rows] == [21, 36, 21, 32, 21, 27, 21, 0, 21, 21, 21, 21]
    proof = dict(architecture="aarch64", platform="virtualbox-gicv3",
        firmware_controller_handoff=True, virtual_slice_timer=True, hardware_user_mode=True,
        supervisor_write_blocked=True, nx_enforced=True, infinite_loop_preempted=True,
        privileged_instruction_blocked=True, memory_reclaimed=True,
        kernel_survived_each_case=True, repeated_resume_tls_simd=True,
        installed_acceptance=False, cases=rows)
    evidence = ROOT / "builds/validation/native-apps/aarch64-virtualbox-execution.json"
    evidence.parent.mkdir(parents=True, exist_ok=True)
    evidence.write_text(json.dumps(proof, indent=2) + "\n")
    print(json.dumps(proof))

# ------------------------=
# FUNC: virtualbox_compiler_result
# DESC: Extracts and validates structured native compiler records following the VirtualBox firmware prefix.
# ------------------=
def virtualbox_compiler_result(path, args):
    data = path.read_bytes()
    marker = struct.pack("<Q", 0x434C414E47454C30)
    offset = data.rfind(marker)
    assert offset >= 0
    args.arch = "aarch64"
    work = ROOT / "build/native-process-probe/aarch64-virtualbox-validation"
    work.mkdir(parents=True, exist_ok=True)
    compiler_result(data[offset:], work, args)

# ------------------------=
# FUNC: main
# DESC: Builds and boots one disposable x86 VM, asserting binary privilege/fault/preemption outcomes.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Use ./build-kit run python3 tools/native-process-probe/run.py")
    parser = argparse.ArgumentParser()
    parser.add_argument("--arch", choices=["x86_64", "aarch64"], default="x86_64")
    parser.add_argument("--gic", choices=[2, 3], type=int, default=2)
    parser.add_argument("--split-eoi", action="store_true")
    parser.add_argument("--firmware", action="store_true", help="Use production ARM UEFI discovery and handoff")
    parser.add_argument("--virtualbox", action="store_true", help="Build a production-handoff probe for the repository-owned VirtualBox runner")
    parser.add_argument("--validate-virtualbox-serial", type=Path, help="Validate binary records captured by the VirtualBox ARM probe")
    parser.add_argument("--validate-virtualbox-compiler-serial", type=Path, help="Validate compiler records captured by the VirtualBox ARM probe")
    parser.add_argument("--memory", choices=["512M", "1G"], default="1G", help="Firmware guest RAM for low-memory boot coverage")
    parser.add_argument("--compiler", type=Path, help="Run a native Clang/LLD image in isolated hardware user mode")
    parser.add_argument("--python-script", action="store_true", help="Execute a real Python script using the supplied native interpreter image")
    parser.add_argument("--python-pip", action="store_true", help="Run actual pip install/import/metadata/rejection/uninstall across serialized native package mounts")
    parser.add_argument("--python-pip-imports", action="store_true", help="Diagnose native installer imports before pip's eager-import audit can mask their exceptions")
    parser.add_argument("--python-extension", type=Path, help="Import and call an actual CPython native extension in the guest")
    parser.add_argument("--python-validation", choices=("valid", "invalid"), help="Run the production -c syntax validator without executing the granted Python source")
    parser.add_argument("--compile-source", action="store_true", help="Compile real C from a bounded native input stream to an ELF object")
    parser.add_argument("--interactive", action="store_true", help="Compile and exercise the native interactive calculator fixture")
    parser.add_argument("--reject-invalid-source", action="store_true", help="Verify real compiler failure, diagnostics, no executable output and full memory recovery")
    parser.add_argument("--link-object", type=Path, help="Run native LLD with this object exposed as a read-only guest fixture")
    args = parser.parse_args()
    if args.validate_virtualbox_serial:
        virtualbox_result(args.validate_virtualbox_serial)
        return
    if args.validate_virtualbox_compiler_serial:
        virtualbox_compiler_result(args.validate_virtualbox_compiler_serial, args)
        return
    assert not args.interactive or args.compiler
    assert not args.python_script or (args.compiler and not args.compile_source and not args.link_object)
    assert not args.python_pip or (args.compiler and not args.python_script and not args.compile_source and not args.link_object and not args.firmware and not args.virtualbox)
    assert not args.python_pip_imports or args.python_pip
    assert not args.python_extension or args.python_script
    assert not args.python_validation or (args.python_script and not args.python_extension)
    assert not args.reject_invalid_source or (args.compiler and args.compile_source and not args.link_object)
    arm = args.arch == "aarch64"
    assert not args.firmware or (arm and not args.split_eoi)
    assert not args.virtualbox or (arm and args.firmware)
    target = "aarch64-unknown-none-softfloat" if arm else "x86_64-unknown-none"
    work = ROOT / "build/native-process-probe" / (args.arch + (f"-gic{args.gic}" if arm else "") +
        ("-firmware" if args.firmware else "") +
        ("-virtualbox" if args.virtualbox else "") +
        ("-compiler" if args.compiler else ""))
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
    if args.python_pip:
        spec = importlib.util.spec_from_file_location("pip_fixtures", ROOT / "tools/native-python/package-tests.py")
        fixtures = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(fixtures)
        fixtures.verify_fixtures()
        fixture_dir = work / "pip-fixtures"
        fixtures.write_inputs(fixture_dir)
        env["INFINITY_PIP_FIXTURES"] = str(fixture_dir)
    env.pop("INFINITY_PIP_IMPORT_ONLY", None)
    if args.python_pip_imports:
        env["INFINITY_PIP_IMPORT_ONLY"] = "1"
    # Compiler probes use the actual isolated main() CRT shipped with the SDK.
    env["INFINITY_APPLICATION_CRT"] = str(ROOT / "builds/toolchains" / args.arch / "application-crt.o") if args.compiler else str(work / "user.o")
    env.pop("INFINITY_PROCESS_SPLIT_EOI", None)
    if args.split_eoi:
        assert arm and args.gic == 3
        env["INFINITY_PROCESS_SPLIT_EOI"] = "1"
    env.pop("INFINITY_PROCESS_VIRTUALBOX", None)
    if args.virtualbox:
        env["INFINITY_PROCESS_VIRTUALBOX"] = "1"
    env.pop("INFINITY_COMPILER_PROBE", None)
    env.pop("INFINITY_PYTHON_PROBE", None)
    env.pop("INFINITY_PYTHON_EXTENSION_PROBE", None)
    env.pop("INFINITY_PYTHON_VALIDATION_PROBE", None)
    env.pop("INFINITY_PYTHON_INVALID_PROBE", None)
    if args.python_validation:
        env["INFINITY_PYTHON_VALIDATION_PROBE"] = "1"
        if args.python_validation == "invalid":
            env["INFINITY_PYTHON_INVALID_PROBE"] = "1"
    env["INFINITY_PYTHON_EXTENSION_BYTES"] = str(args.python_extension.resolve()) if args.python_extension else str(work / "user.o")
    if args.python_extension:
        env["INFINITY_PYTHON_EXTENSION_PROBE"] = "1"
    if args.python_script:
        env["INFINITY_PYTHON_PROBE"] = "1"
    env.pop("INFINITY_INTERACTIVE_APP", None)
    if args.interactive:
        env["INFINITY_INTERACTIVE_APP"] = "1"
    env.pop("INFINITY_COMPILER_INVALID_SOURCE", None)
    if args.reject_invalid_source:
        env["INFINITY_COMPILER_INVALID_SOURCE"] = "1"
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
        target, "--manifest-path", "tools/native-process-probe/Cargo.toml"] +
        (["--features", "python-pip-probe"] if args.python_pip else []), env=env, check=True)
    subprocess.run([linker, "-nostdlib", "-static", "-T",
        "tools/native-process-probe/arm.ld" if arm else "linker/x86_64.ld", "-o", str(work / "probe.elf"),
        str(work / "cargo" / target / "release/libinfinity_native_process_probe.a")] +
        (["-e", "infinity_kernel_entry"] if args.firmware else []), check=True)
    if args.virtualbox:
        print(work / "probe.elf")
        return
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
                *boot_args], stdout=log, stderr=subprocess.STDOUT, check=True, timeout=600 if args.python_pip else 120 if args.compiler or args.firmware else 30)
        data = (work / "result.bin").read_bytes()
        if args.python_pip:
            pip_result(data, work, args)
            return
        if args.firmware:
            data=data[-(12*32+8):]  # Only the fixed binary result trailer is acceptance evidence.
        if args.compiler:
            compiler_result(data, work, args)
            return
        assert len(data) == 12 * 32 + 8, data.hex()
        assert struct.unpack_from("<Q", data, 12 * 32)[0] == 0x49534f4c41544544
        rows = list(struct.iter_unpack("<4Q", data[:-8]))
        timer_interrupt = 30
        assert [r[2] >> 26 if r[1] == 0 else r[2] for r in rows] == [21, 36, 21, 32, 21, timer_interrupt, 21, 0, 21, 21, 21, 21]
        proof = dict(architecture=args.arch, firmware_controller_handoff=args.firmware,
            hardware_user_mode=True, supervisor_write_blocked=True,
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
            "-display", "none", "-serial", "stdio", "-no-reboot"], stdout=log, stderr=subprocess.STDOUT, timeout=600 if args.python_pip else 120 if args.compiler else 60)
    data = (work / "result.bin").read_bytes()
    assert run.returncode == 33, (run.returncode, data.hex(), (work / "qemu.log").read_text()[-2000:])
    if args.python_pip:
        pip_result(data, work, args)
        return
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
