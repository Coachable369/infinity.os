"""Behavioral memory primitive checks; this is not installed audio acceptance."""
from pathlib import Path
import os
import subprocess
import unittest

ROOT = Path(__file__).resolve().parents[2]


class NativeMemoryTests(unittest.TestCase):
    # ------------------------=
    # FUNC: test_alignment_overlap_and_sentinels
    # DESC: Builds the actual native primitive source with sanitizers and checks every byte against libc.
    # ------------------=
    def test_alignment_overlap_and_sentinels(self):
        self.assertEqual(os.environ.get("INFINITY_BUILD_KIT_ACTIVE"), "1", "Run through build-kit")
        output = ROOT / "build/voice-kokoro"
        output.mkdir(parents=True, exist_ok=True)
        common = ["clang", "-O2", "-fsanitize=address,undefined", "-fno-builtin"]
        subprocess.run(common + ["-Dmemcpy=native_memcpy", "-Dmemmove=native_memmove",
            "-Dmemset=native_memset", "-c", str(ROOT / "tools/voice-kokoro/memory.c"),
            "-o", str(output / "memory-test.o")], check=True)
        subprocess.run(common + [str(ROOT / "tools/voice-kokoro/memory-test.c"),
            str(output / "memory-test.o"), "-o", str(output / "memory-test")], check=True)
        subprocess.run([str(output / "memory-test")], check=True)


if __name__ == "__main__":
    unittest.main()
