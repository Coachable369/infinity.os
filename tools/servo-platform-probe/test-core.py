"""Run behavioral checks for engine-independent browser and worker state."""
import os
from pathlib import Path
import subprocess

# ------------------------=
# FUNC: main
# DESC: Compiles and exercises bounded browser state only inside the active build kit.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Run through build-kit")
    root = Path(__file__).resolve().parents[2]
    output = root / "build/servo-platform-probe/browser-core-test"
    output.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run(["rustc", "--edition=2021", "--test", str(root / "sdk/infinity-browser-core/lib.rs"),
                    "-o", str(output)], check=True)
    return subprocess.run([str(output)]).returncode

if __name__ == "__main__":
    raise SystemExit(main())
