"""Check the pinned engine; keep source/cache/logs inside the repository."""
import json
import os
from pathlib import Path
import subprocess


# ------------------------=
# FUNC: main
# DESC: Records an actual minimal-feature Servo compiler attempt without runtime claims.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Run via ./build-kit run python3 tools/servo-platform-probe/check-servo.py")
    root = Path(__file__).resolve().parents[2]
    source = root / "build/servo-port-audit"
    revision = subprocess.check_output(["git", "-C", str(source), "rev-parse", "HEAD"], text=True).strip()
    if revision != "d05154e2b4def11a9fefe412898a0a6c8925a9cd":
        raise SystemExit("Unreviewed Servo revision")
    output = root / "build/servo-platform-probe"
    output.mkdir(parents=True, exist_ok=True)
    environment = os.environ.copy()
    environment.update({"RUSTC_BOOTSTRAP": "1", "CARGO_HOME": str(root / "build/servo-cargo-home"),
                        "__CARGO_TESTS_ONLY_SRC_ROOT": str(root / "build/servo-rust-src/library"),
                        "RUSTFLAGS": "--cfg infinity_native"})
    # Use actual freestanding ARM headers already built by the native toolchain,
    # never macOS headers. This compiler check does not link the serial voice libc.
    includes = [root / "build/voice-newlib-aarch64/aarch64-none-elf/newlib/targ-include",
                root / "build/newlib-4.6.0.20260123/newlib/libc/include"]
    if not all(path.is_dir() for path in includes):
        raise SystemExit("Native ARM C headers unavailable; prepare the native toolchain through build-kit")
    environment["CC_aarch64_unknown_none_softfloat"] = "/opt/homebrew/opt/llvm/bin/clang"
    environment["CFLAGS_aarch64_unknown_none_softfloat"] = (
        "--target=aarch64-none-elf -ffreestanding -mstrict-align " +
        " ".join("-isystem " + str(path) for path in includes))
    command = ["cargo", "check", "-j", "4", "-Z", "build-std=std,panic_abort",
               "--target", "aarch64-unknown-none-softfloat", "--manifest-path",
               str(source / "components/servo/Cargo.toml"), "--no-default-features",
               "--features", "bundled", "--locked"]
    with (output / "servo-check.log").open("w") as log:
        result = subprocess.run(command, cwd=root, env=environment, stdout=log,
                                stderr=subprocess.STDOUT, check=False)
    report = {"servo_revision": revision, "command": command,
              "compiler_exit_status": result.returncode, "executed": False}
    (output / "servo-check.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report, indent=2))
    return result.returncode


if __name__ == "__main__":
    raise SystemExit(main())
