"""Run the isolated byte-order implementation as C and C++; host evidence only."""
import os
from pathlib import Path
import subprocess


# ------------------------=
# FUNC: main
# DESC: Builds and executes all byte-order assertions without treating compilation as behavioral proof.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Run through build-kit")
    root = Path(__file__).resolve().parents[2]
    output = root / "build/servo-platform-probe"
    output.mkdir(parents=True, exist_ok=True)
    for language in ("c", "c++"):
        executable = output / ("byteorder-" + language)
        subprocess.run(["/opt/homebrew/opt/llvm/bin/clang", "-x", language, "-Wall", "-Werror",
                        "-I" + str(root / "sdk/servo-std/include"),
                        str(Path(__file__).with_name("byteorder-test.c")), "-o", str(executable)], check=True)
        subprocess.run([str(executable)], check=True)
        print(language + ": byte-order behavior PASS (host execution)")


if __name__ == "__main__":
    main()
