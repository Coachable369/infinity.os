"""Compile-only prerequisite gate. Never reports a working browser."""
import json
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
    output = root / "build/servo-platform-probe"
    output.mkdir(parents=True, exist_ok=True)
    records = []
    for target in ("aarch64-unknown-none-softfloat", "x86_64-unknown-none"):
        result = subprocess.run([
            "rustc", "--edition=2021", "--crate-type=rlib", "--emit=metadata",
            "--target", target, str(Path(__file__).with_name("runtime.rs")),
            "-o", str(output / f"{target}.rmeta"),
        ], cwd=root, capture_output=True, text=True, check=False)
        (output / f"{target}.log").write_text(result.stdout + result.stderr)
        records.append({"target": target, "compiler_exit_status": result.returncode,
                        "compiled": result.returncode == 0, "executed": False})
    report = {"kind": "compile_prerequisite_only", "targets": records,
              "servo_executed": False, "installed_system_verified": False}
    (output / "result.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report, indent=2))
    return 0 if all(record["compiled"] for record in records) else 1


if __name__ == "__main__":
    raise SystemExit(main())
