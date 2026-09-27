"""Track native browser configuration without invalidating identical builds."""
import os
from pathlib import Path
import sys

# ------------------------=
# FUNC: main
# DESC: Updates the shared live/installed kernel configuration stamp only on a real mode transition.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Run through build-kit")
    if len(sys.argv) != 2 or sys.argv[1] not in ("0", "1"):
        raise SystemExit("Native browser mode must be 0 or 1")
    path = Path(__file__).resolve().parents[1] / "build/browser-mode"
    value = (sys.argv[1] + "\n").encode()
    if not path.exists() or path.read_bytes() != value:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(value)

if __name__ == "__main__":
    main()
