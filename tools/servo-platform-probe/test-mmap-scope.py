"""Exercise the staged native access macros without claiming fault recovery."""
import os
from pathlib import Path
import subprocess

# ------------------------=
# FUNC: main
# DESC: Compiles and runs native access-scope behavior under the build kit.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Run through build-kit")
    root = Path(__file__).resolve().parents[2]
    headers = root / "build/servo-native-deps/mozjs_sys-153.3.0-0/mozjs/mozglue/misc"
    binary = root / "build/servo-platform-probe/mmap-scope-test"
    subprocess.run(["/opt/homebrew/opt/llvm/bin/clang++", "-std=c++20", "-Wall", "-Wextra", "-Werror",
                    "-D__INFINITYOS__=1", "-I" + str(headers),
                    str(Path(__file__).with_name("mmap-scope-test.cpp")), "-o", str(binary)], check=True)
    subprocess.run([str(binary)], check=True)

if __name__ == "__main__":
    main()
