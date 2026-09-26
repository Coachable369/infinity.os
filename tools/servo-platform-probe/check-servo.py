"""Check the pinned engine; keep source/cache/logs inside the repository."""
import json
import argparse
import os
from pathlib import Path
import subprocess
import shlex


# ------------------------=
# FUNC: main
# DESC: Records an actual minimal-feature Servo compiler attempt without runtime claims.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Run via ./build-kit run python3 tools/servo-platform-probe/check-servo.py")
    root = Path(__file__).resolve().parents[2]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--package", choices=("servo", "fontsan", "aws-lc-sys"), default="servo")
    parser.add_argument("--arch", choices=("aarch64", "x86_64"), default="aarch64")
    parser.add_argument("--codegen", action="store_true", help="Build native engine archives, not just metadata; does not link a browser")
    options = parser.parse_args()
    subprocess.run(["python3", str(Path(__file__).with_name("prepare-deps.py"))], check=True)
    subprocess.run(["python3", str(Path(__file__).with_name("prepare-mio.py")), "--engine"], check=True)
    subprocess.run(["python3", str(Path(__file__).with_name("prepare-async-net.py")), "--engine"], check=True)
    subprocess.run(["python3", str(Path(__file__).with_name("prepare-entropy.py"))], check=True)
    subprocess.run(["python3", str(Path(__file__).with_name("prepare-fonts.py"))], check=True)
    subprocess.run(["python3", str(Path(__file__).with_name("prepare-style.py"))], check=True)
    subprocess.run(["python3", str(Path(__file__).with_name("prepare-webdriver.py"))], check=True)
    subprocess.run(["python3", str(Path(__file__).with_name("prepare-surfman.py"))], check=True)
    subprocess.run(["python3", str(Path(__file__).with_name("prepare-mozjs.py"))], check=True)
    subprocess.run(["python3", str(Path(__file__).with_name("prepare-certificates.py"))], check=True)
    subprocess.run(["python3", str(Path(__file__).with_name("prepare-engine.py"))], check=True)
    subprocess.run(["python3", str(Path(__file__).with_name("prepare-bindgen.py"))], check=True)
    arch = options.arch
    target = "aarch64-unknown-none-softfloat" if arch == "aarch64" else "x86_64-unknown-none"
    target_key = target.replace("-", "_")
    source = root / "build/servo-port-audit"
    revision = subprocess.check_output(["git", "-C", str(source), "rev-parse", "HEAD"], text=True).strip()
    if revision != "d05154e2b4def11a9fefe412898a0a6c8925a9cd":
        raise SystemExit("Unreviewed Servo revision")
    output = root / "build/servo-platform-probe"
    output.mkdir(parents=True, exist_ok=True)
    environment = os.environ.copy()
    for name in ("CPPFLAGS", "CFLAGS", "CXXFLAGS", "CPATH", "C_INCLUDE_PATH", "CPLUS_INCLUDE_PATH", "LIBRARY_PATH"):
        environment.pop(name, None)
    environment.update({"RUSTC_BOOTSTRAP": "1", "CARGO_HOME": str(root / "build/servo-cargo-home"),
                        "RUST_BACKTRACE": "1",
                        "LIBCLANG_PATH": "/opt/homebrew/opt/llvm/lib",
                        "__CARGO_TESTS_ONLY_SRC_ROOT": str(root / "build/servo-rust-src/library"),
                        "RUSTFLAGS": "--cfg infinity_native --check-cfg=cfg(infinity_native) --check-cfg=cfg(infinity_certificate_test)"})
    # Native VFS and mutex callbacks are mandatory at linkage/initialization.
    # Do not select SQLite's Unix filesystem or pthread backend for this target.
    environment["LIBSQLITE3_FLAGS"] = "-DSQLITE_OS_OTHER=1 -DSQLITE_MUTEX_APPDEF=1 -DSQLITE_OMIT_LOAD_EXTENSION=1"
    # Use actual freestanding target headers already built by the native toolchain,
    # never macOS headers. This compiler check does not link the serial voice libc.
    includes = [root / f"build/voice-newlib-{arch}/{arch}-none-elf/newlib/targ-include",
                root / "build/newlib-4.6.0.20260123/newlib/libc/include"]
    if not all(path.is_dir() for path in includes):
        raise SystemExit("Native target C headers unavailable; prepare the native toolchain through build-kit")
    environment[f"CC_{target_key}"] = "/opt/homebrew/opt/llvm/bin/clang"
    environment[f"AR_{target_key}"] = "/opt/homebrew/opt/llvm/bin/llvm-ar"
    environment[f"CPP_{target_key}"] = f"/opt/homebrew/opt/llvm/bin/clang --target={arch}-none-elf -E"
    environment[f"CPPFLAGS_{target_key}"] = ""
    cflags = (
        f"--target={arch}-none-elf -ffreestanding " +
        ("-mstrict-align " if arch == "aarch64" else "-mno-red-zone ") +
        "-I" + str(root / "sdk/servo-std/include") + " " +
        "-include " + str(root / "sdk/servo-std/c-target.h") + " " +
        " ".join("-isystem " + str(path) for path in includes))
    environment[f"CFLAGS_{target_key}"] = cflags
    cxx_headers = root / f"build/voice-kokoro/cxx-{arch}/include/c++/v1"
    if not (cxx_headers / "__config_site").is_file():
        raise SystemExit("Native libc++ headers unavailable; prepare native toolchain through build-kit")
    environment[f"CXX_{target_key}"] = "/opt/homebrew/opt/llvm/bin/clang++"
    # mozjs reads the global spelling; cc-rs reads the target-qualified one.
    environment["CXXSTDLIB"] = "c++"
    environment[f"CXXSTDLIB_{target_key}"] = "c++"
    environment[f"CXXFLAGS_{target_key}"] = (
        "-nostdinc++ -isystem " + str(cxx_headers) + " " + cflags)
    subprocess.run([environment[f"CC_{target_key}"],
                    *shlex.split(cflags), "-std=c11", "-fsyntax-only",
                    str(Path(__file__).with_name("c-abi-probe.c"))], check=True, env=environment)
    command = ["cargo", "build" if options.codegen else "check", "-j", "4", "-Z", "build-std=std,panic_abort",
               "--target", target, "--manifest-path",
               str(source / "components/servo/Cargo.toml"), "--locked", "--message-format=json-render-diagnostics"]
    native_libc = root / "build/servo-native-deps/libc-0.2.189"
    if native_libc.is_dir():
        command += ["--config", 'patch.crates-io.libc.path="' + str(native_libc) + '"']
    command += ["--config", 'patch.crates-io.mio.path="' + str(root / "build/servo-native-deps/mio-1.2.3") + '"']
    command += ["--config", 'patch.crates-io.webdriver.path="' + str(root / "build/servo-native-deps/webdriver-0.54.0") + '"']
    command += ["--config", 'patch.crates-io.imsz.path="' + str(root / "build/servo-native-deps/imsz-0.4.1") + '"']
    command += ["--config", 'patch.crates-io.surfman.path="' + str(root / "build/servo-native-deps/surfman-0.14.0") + '"']
    command += ["--config", 'patch.crates-io.mozjs_sys.path="' + str(root / "build/servo-native-deps/mozjs_sys-153.3.0-0") + '"']
    command += ["--config", 'patch.crates-io.bindgen.path="' + str(root / "build/servo-native-deps/bindgen-0.72.1") + '"']
    command += ["--config", 'patch.crates-io.rustls-platform-verifier.path="' + str(root / "build/servo-native-deps/rustls-platform-verifier-0.7.0") + '"']
    for name, version in (("tokio", "1.53.1"), ("hyper-util", "0.1.20")):
        command += ["--config", 'patch.crates-io.' + name + '.path="' + str(root / "build/servo-native-deps" / (name + "-" + version)) + '"']
    for name, path in json.loads((root / "build/servo-native-deps/stylo/native-patches.json").read_text()).items():
        command += ["--config", 'patch."https://github.com/servo/stylo".' + name + '.path="' + path + '"']
    command += ["--config", 'patch.crates-io.wr_glyph_rasterizer.path="' + str(root / "build/servo-native-deps/wr_glyph_rasterizer-0.70.0") + '"']
    command += ["--config", 'patch.crates-io.freetype-sys.path="' + str(root / "build/servo-native-deps/freetype-sys-0.23.0") + '"']
    for index, version in enumerate(("0.2.17", "0.3.4", "0.4.1")):
        key = 'patch.crates-io.getrandom_native_' + str(index)
        command += ["--config", key + '.package="getrandom"', "--config", key + '.path="' + str(root / "build/servo-native-deps" / ("getrandom-" + version)) + '"']
    if options.package == "servo":
        command += ["--no-default-features", "--features", "bundled,ipc-channel/force-inprocess"]
    else:
        command += ["-p", options.package]
    prefix = options.package + "-" + arch + ("-codegen" if options.codegen else "-check")
    native_paths = set()
    with (output / (prefix + ".log")).open("w") as log:
        process = subprocess.Popen(command, cwd=root, env=environment, stdout=subprocess.PIPE,
                                   stderr=log, text=True)
        for line in process.stdout:
            try:
                event = json.loads(line)
            except ValueError:
                log.write(line)
                continue
            if event.get("reason") == "build-script-executed":
                native_paths.update(event.get("linked_paths", []))
            if event.get("reason") == "compiler-message":
                log.write(event["message"].get("rendered") or "")
            log.flush()
        status = process.wait()
    report = {"servo_revision": revision, "package": options.package, "target": target, "command": command,
              "compiler_exit_status": status, "native_search_paths": sorted(native_paths), "executed": False}
    (output / (prefix + ".json")).write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report, indent=2))
    return status


if __name__ == "__main__":
    raise SystemExit(main())
