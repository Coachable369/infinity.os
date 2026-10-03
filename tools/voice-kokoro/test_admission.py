"""Behavioral native-engine admission verification; not installed speech acceptance."""
from pathlib import Path
import os
import subprocess
import unittest

ROOT = Path(__file__).resolve().parents[2]


class NativeAdmissionTests(unittest.TestCase):
    # ------------------------=
    # FUNC: test_concurrent_native_ownership
    # DESC: Executes the production atomic admission helper under concurrent callers and verifies protected buffer ownership.
    # ------------------=
    def test_concurrent_native_ownership(self):
        self.assertEqual(os.environ.get("INFINITY_BUILD_KIT_ACTIVE"), "1", "Run through build-kit")
        output = ROOT / "build/voice-kokoro/admission-test"
        output.parent.mkdir(parents=True, exist_ok=True)
        subprocess.run(["clang", "-std=c11", "-O2", "-Wall", "-Wextra", "-Werror",
                        "-fsanitize=address,undefined", "-pthread",
                        str(ROOT / "tools/voice-kokoro/admission-test.c"), "-o", str(output)], check=True)
        subprocess.run([str(output)], check=True)


if __name__ == "__main__":
    unittest.main()
