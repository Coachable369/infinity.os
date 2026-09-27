"""Execute memory primitives in a disposable freestanding ARM64 guest, not the installed OS."""
import json
import argparse
import os
from pathlib import Path
import struct
import subprocess
import shutil


# ------------------------=
# FUNC: main
# DESC: Builds through the active kit and checks binary guest assertions rather than serial prose.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Run through build-kit")
    root = Path(__file__).resolve().parents[2]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--context-probe", action="store_true")
    parser.add_argument("--executor-probe", action="store_true")
    parser.add_argument("--std-probe", action="store_true")
    parser.add_argument("--mio-probe", action="store_true")
    parser.add_argument("--socket-probe", action="store_true")
    parser.add_argument("--async-probe", action="store_true")
    parser.add_argument("--arch", choices=["aarch64", "x86_64"], default="aarch64")
    options = parser.parse_args()
    if options.async_probe:
        options.socket_probe = True
    if options.socket_probe:
        options.mio_probe = True
    if options.mio_probe:
        options.std_probe = True
    # Cargo resolves optional path dependencies even when their feature is disabled.
    subprocess.run(["python3", str(Path(__file__).with_name("prepare-mio.py"))], check=True)
    subprocess.run(["python3", str(Path(__file__).with_name("prepare-async-net.py"))], check=True)
    subprocess.run(["python3", str(Path(__file__).with_name("prepare-entropy.py"))], check=True)
    if options.std_probe:
        options.executor_probe = True
    if options.executor_probe:
        options.context_probe = True
    output = root / ("build/servo-context-guest" if options.context_probe else "build/servo-memory-guest")
    if options.executor_probe:
        output = root / "build/servo-executor-guest"
    if options.std_probe:
        output = root / "build/servo-std-guest"
    if options.mio_probe:
        output = root / "build/servo-mio-guest"
    if options.socket_probe:
        output = root / "build/servo-socket-guest"
    if options.async_probe:
        output = root / "build/servo-async-guest"
    output = output / options.arch
    output.mkdir(parents=True, exist_ok=True)
    target = output / "target"
    triple = "aarch64-unknown-none" if options.context_probe else "aarch64-unknown-none-softfloat"
    if options.arch == "x86_64":
        triple = "x86_64-unknown-none"
    environment = dict(os.environ, RUSTC_BOOTSTRAP="1", CARGO_TARGET_DIR=str(target))
    if options.std_probe:
        environment["__CARGO_TESTS_ONLY_SRC_ROOT"] = str(root / "build/servo-rust-src/library")
        environment["RUSTFLAGS"] = "--cfg infinity_native --check-cfg=cfg(infinity_native)"
        if options.socket_probe:
            # The socket proof does not exercise TLS. Keep linked HTTP crypto
            # dependencies on their reviewed software paths for freestanding
            # targets which do not enable vector instruction ABI support.
            environment["RUSTFLAGS"] += " --cfg aes_force_soft --cfg polyval_force_soft"
    command = ["cargo", "build", "--manifest-path", str(Path(__file__).with_name("guest") / "Cargo.toml"),
                    "--release", "-Z", "build-std=std,panic_abort" if options.std_probe else "build-std=core", "--target", triple]
    command += ["--config", 'patch.crates-io.mio.path="' + str(root / "build/servo-native-deps/mio-1.2.3") + '"']
    if options.async_probe:
        command += ["--features", "async-probe"]
    elif options.socket_probe:
        command += ["--features", "socket-probe"]
    elif options.mio_probe:
        command += ["--features", "mio-probe"]
    elif options.std_probe:
        command += ["--features", "std-probe"]
    elif options.executor_probe:
        command += ["--features", "executor-probe"]
    elif options.context_probe:
        command += ["--features", "context-probe"]
    subprocess.run(command, env=environment, check=True)
    executable = output / "probe.elf"
    native_objects = []
    if options.std_probe:
        sqlite = root / "build/servo-cargo-home/registry/src/index.crates.io-1949cf8c6b5b557f/libsqlite3-sys-0.36.0/sqlite3"
        for source in (Path(__file__).with_name("c-allocator-test.c"), root / "sdk/servo-std/c-reentrant-allocator.c",
                       Path(__file__).with_name("c-thread-time-test.c"), root / "sdk/servo-std/c-thread-time.c",
                       root / "sdk/servo-std/c-sync.c", Path(__file__).with_name("c-sync-test.c"),
                       root / "sdk/servo-std/c-thread.c", Path(__file__).with_name("c-thread-test.c"),
                       root / "sdk/servo-std/c-memory.c", Path(__file__).with_name("c-memory-test.c"),
                       root / "sdk/servo-std/c-system.c", root / "sdk/servo-std/c-sqlite.c",
                       Path(__file__).with_name("c-sqlite-test.c"), sqlite / "sqlite3.c"):
            obj = output / (source.stem + ".o")
            subprocess.run(["/opt/homebrew/opt/llvm/bin/clang", "--target=" + options.arch + "-none-elf",
                        "-mstrict-align" if options.arch == "aarch64" else "-mno-red-zone",
                        "-ffreestanding", "-fno-builtin", "-O2", "-c",
                        "-I", str(sqlite), "-DSQLITE_OS_OTHER=1", "-DSQLITE_MUTEX_APPDEF=1", "-DSQLITE_OMIT_LOAD_EXTENSION=1",
                        "-include", str(root / "sdk/servo-std/c-target.h"),
                        "-isystem", str(root / ("build/voice-newlib-" + options.arch) / (options.arch + "-none-elf/newlib/targ-include")),
                        "-isystem", str(root / "build/newlib-4.6.0.20260123/newlib/libc/include"),
                        str(source), "-o", str(obj)], check=True)
            native_objects.append(str(obj))
        native_objects += [str(root / ("build/voice-newlib-" + options.arch) / (options.arch + "-none-elf/newlib") / library) for library in ("libc.a", "libm.a")]
    linker = Path(__file__).with_name("guest") / "link.ld" if options.arch == "aarch64" else root / "linker/x86_64.ld"
    subprocess.run(["/opt/homebrew/opt/lld/bin/ld.lld", "--gc-sections", "-nostdlib", "-T",
                    str(linker), "-o", str(executable),
                    str(target / triple / "release/libinfinity_servo_memory_probe.a"), *native_objects], check=True)
    result = output / "result.bin"
    result.unlink(missing_ok=True)
    expected = (2, 0, 2002, 4096) if options.context_probe else (1, 0, 65, 4096)
    if options.executor_probe:
        expected = (3, 0, 2002, 4096)
    if options.std_probe:
        expected = (4, 0, 2002, 4096)
    if options.mio_probe:
        expected = (5, 0, 2002, 4096)
    if options.socket_probe:
        expected = (6, 0, 2002, 4096)
    if options.async_probe:
        expected = (7, 0, 2002, 4096)
    if options.arch == "aarch64":
        subprocess.run(["qemu-system-aarch64", "-machine", "virt", "-accel", "tcg", "-cpu", "max",
                        "-m", "128M", "-display", "none", "-serial", "file:" + str(result),
                        "-monitor", "none", "-kernel", str(executable)], check=True, timeout=30)
        record = struct.unpack("<4Q", result.read_bytes())
    else:
        volume = output / "volume"
        (volume / "EFI/BOOT").mkdir(parents=True, exist_ok=True)
        (volume / "EFI/INFINITY").mkdir(parents=True, exist_ok=True)
        shutil.copyfile(root / "build/x86_64/BOOTX64.EFI", volume / "EFI/BOOT/BOOTX64.EFI")
        shutil.copyfile(executable, volume / "EFI/INFINITY/KERNEL.ELF")
        with (output / "qemu.log").open("wb") as log:
            guest = subprocess.run(["qemu-system-x86_64", "-machine", "pc", "-cpu", "max", "-accel", "tcg",
                "-drive", "if=pflash,format=raw,readonly=on,file=/opt/homebrew/share/qemu/edk2-x86_64-code.fd",
                "-device", "isa-debug-exit,iobase=0xf4,iosize=0x04", "-m", "512M",
                "-drive", "format=raw,file=fat:rw:" + str(volume), "-boot", "order=c",
                "-display", "none", "-serial", "stdio", "-monitor", "none", "-no-reboot"],
                stdout=log, stderr=subprocess.STDOUT, timeout=60)
        if guest.returncode != 33:
            raise RuntimeError("Native guest assertions failed: exit " + str(guest.returncode))
        record = expected
    if record != expected:
        raise RuntimeError("Guest memory assertions failed: " + repr(record))
    evidence = dict(environment="freestanding QEMU guest", architecture=options.arch, operations=record[2], context_probe=options.context_probe,
                    executor_probe=options.executor_probe, std_probe=options.std_probe, mio_probe=options.mio_probe,
                    socket_probe=options.socket_probe, async_probe=options.async_probe,
                    dns_contract_probe=options.std_probe, live_dns=False,
                    arena_bytes=record[3], passed=True, installed_os=False, servo_executed=False)
    (output / "evidence.json").write_text(json.dumps(evidence, indent=2) + "\n")
    print(json.dumps(evidence, indent=2))


if __name__ == "__main__":
    main()
