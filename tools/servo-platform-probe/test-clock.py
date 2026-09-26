"""Exercise the actual native-clock conversion in C and C++ with error vectors."""
import os
from pathlib import Path
import subprocess


# ------------------------=
# FUNC: main
# DESC: Builds and executes bounded host adapter tests; it does not claim a guest engine run.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Run through build-kit")
    root = Path(__file__).resolve().parents[2]
    output = root / "build/servo-platform-probe"
    output.mkdir(parents=True, exist_ok=True)
    for language, standard in (("c", "c11"), ("c++", "c++20")):
        binary = output / ("clock-test-" + standard)
        subprocess.run(["/opt/homebrew/opt/llvm/bin/clang", "-x", language, "-std=" + standard,
                        "-Wall", "-Wextra", "-Werror", "-I" + str(root / "sdk/servo-std/include"),
                        str(Path(__file__).with_name("clock-test.c")), "-o", str(binary)], check=True)
        subprocess.run([str(binary)], check=True)
    print("Native clock conversion: C and C++ behavior passed")


if __name__ == "__main__":
    main()
