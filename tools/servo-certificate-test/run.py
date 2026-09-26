"""Run native-root verifier behavior against authenticated recorded certificates."""
import os
from pathlib import Path
import subprocess


# ------------------------=
# FUNC: main
# DESC: Stages the production verifier and tests its actual native-root branch on the build host.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Run through build-kit")
    root = Path(__file__).resolve().parents[2]
    subprocess.run(["python3", str(root / "tools/servo-platform-probe/prepare-certificates.py")], check=True)
    environment = os.environ.copy()
    environment["CARGO_HOME"] = str(root / "build/servo-cargo-home")
    environment["RUSTFLAGS"] = "--cfg infinity_certificate_test"
    return subprocess.run(["cargo", "test", "--manifest-path", str(Path(__file__).with_name("Cargo.toml")), "--lib"],
                          cwd=root, env=environment).returncode


if __name__ == "__main__":
    raise SystemExit(main())
