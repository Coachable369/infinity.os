"""Behavioral native mapping-capacity, cooperative-yield and fatal-dispatch checks."""
import json
import os
from pathlib import Path
import subprocess


# ------------------------=
# FUNC: main
# DESC: Executes the real memory adapter and staged SpiderMonkey crash macro through repository-local build products.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Run through build-kit")
    root = Path(__file__).resolve().parents[2]
    output = root / "build/servo-platform-probe/native-memory"
    output.mkdir(parents=True, exist_ok=True)
    compiler = "/opt/homebrew/opt/llvm/bin/clang"
    memory = output / "memory-test"
    subprocess.run([compiler, "-std=c11", "-D_POSIX_C_SOURCE=200112L", "-Wall", "-Wextra", "-Werror",
                    str(Path(__file__).with_name("c-memory-host-test.c")), "-o", str(memory)], check=True)
    subprocess.run([str(memory)], check=True)
    subprocess.run(["python3", str(Path(__file__).with_name("prepare-mozjs.py"))], check=True)
    headers = root / "build/servo-native-deps/mozjs_sys-153.3.0-0/mozjs/mfbt"
    include = output / "mozilla"
    if not include.exists():
        include.symlink_to(headers, target_is_directory=True)
    crash = output / "crash-test"
    subprocess.run([compiler + "++", "-std=c++20", "-D__INFINITYOS__=1", "-I" + str(output),
                    str(Path(__file__).with_name("native-crash-test.cpp")), "-o", str(crash)], check=True)
    result = subprocess.run([str(crash)], check=False)
    if result.returncode != 73:
        raise AssertionError({"expected_fatal_callback_exit": 73, "actual_exit": result.returncode})
    receipt = dict(passed=True, concurrent_mappings=128, reclaim_cycles=3,
                   cooperative_allocation_interleavings=2, allocation_failure_paths=2,
                   native_crash_callback=True, installed_browser=False, captcha_tested=False)
    (output / "result.json").write_text(json.dumps(receipt, indent=2) + "\n")
    print(json.dumps(receipt, indent=2))


if __name__ == "__main__":
    main()
