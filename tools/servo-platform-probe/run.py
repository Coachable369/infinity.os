"""Compile-only prerequisite gate. Never reports a working browser."""
import json
import argparse
import os
from pathlib import Path
import subprocess


# ------------------------=
# FUNC: main
# DESC: Records freestanding target std availability through the repository build kit.
# ------------------=
def main():
    root = Path(__file__).resolve().parents[2]
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Use ./build-kit run python3 tools/servo-platform-probe/run.py")
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--build-std", action="store_true",
                        help="Compile Rust std from source instead of requiring a prebuilt std")
    parser.add_argument("--native-overlay", action="store_true",
                        help="Use the staged Infinity std source adapters (compile only)")
    options = parser.parse_args()
    output = root / "build/servo-platform-probe"
    output.mkdir(parents=True, exist_ok=True)
    records = []
    for target in ("aarch64-unknown-none-softfloat", "x86_64-unknown-none"):
        command = [
            "rustc", "--edition=2021", "--crate-type=rlib", "--emit=metadata",
            "--target", target, str(Path(__file__).with_name("runtime.rs")),
            "-o", str(output / f"{target}.rmeta"),
        ]
        environment = os.environ.copy()
        mode = "prebuilt"
        if options.build_std or options.native_overlay:
            mode = "build-std"
            environment["RUSTC_BOOTSTRAP"] = "1"
            command = ["cargo", "build", "-Z", "build-std=std,panic_abort",
                       "--target", target, "--manifest-path",
                       str(Path(__file__).with_name("Cargo.toml"))]
        if options.native_overlay:
            mode = "native-overlay"
            environment["__CARGO_TESTS_ONLY_SRC_ROOT"] = str(root / "build/servo-rust-src")
            environment["RUSTFLAGS"] = "--cfg infinity_native"
        result = subprocess.run(command, cwd=root, env=environment,
                                capture_output=True, text=True, check=False)
        (output / f"{target}-{mode}.log").write_text(result.stdout + result.stderr)
        records.append({"target": target, "compiler_exit_status": result.returncode,
                        "command": command,
                        "compiled": result.returncode == 0, "executed": False})
    report = {"kind": "compile_prerequisite_only", "mode": mode, "targets": records,
              "servo_executed": False, "installed_system_verified": False}
    (output / f"{mode}-result.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report, indent=2))
    return 0 if all(record["compiled"] for record in records) else 1


if __name__ == "__main__":
    raise SystemExit(main())
