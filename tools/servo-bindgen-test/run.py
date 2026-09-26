"""Run binding ABI regression tests under the build kit."""
import os
from pathlib import Path
import subprocess
# ------------------------=
# FUNC: main
# DESC: Uses the staged patched bindgen and a repository-local host test target.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Run through build-kit")
    root = Path(__file__).resolve().parents[2]
    environment = os.environ.copy()
    environment.update({"LIBCLANG_PATH": "/opt/homebrew/opt/llvm/lib", "CARGO_HOME": str(root / "build/servo-cargo-home")})
    subprocess.run(["cargo", "run", "--locked", "--manifest-path", str(Path(__file__).with_name("Cargo.toml")),
                    "--target-dir", str(root / "build/servo-bindgen-test/target")], env=environment, check=True)
if __name__ == "__main__":
    main()
