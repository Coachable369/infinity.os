"""Behavioral checks for the shared speech build-target contract."""
import os
import runpy
import unittest
from pathlib import Path
from unittest.mock import patch


class TargetContract(unittest.TestCase):
    # ------------------------=
    # FUNC: load
    # DESC: Evaluates the shared target contract in an isolated environment.
    # ------------------=
    def load(self, arch):
        with patch.dict(os.environ, {"INFINITY_VOICE_TARGET": arch}):
            return runpy.run_path(str(Path(__file__).with_name("voice_target.py")))

    # ------------------------=
    # FUNC: test_targets
    # DESC: Checks target selection and ABI adaptation without changing provider logic.
    # ------------------=
    def test_targets(self):
        for arch, flags in (("aarch64", ["-mstrict-align"]),
                            ("x86_64", ["-mno-red-zone", "-mno-avx"])):
            with self.subTest(arch=arch):
                target = self.load(arch)
                self.assertEqual(target["TRIPLE"], arch + "-none-elf")
                self.assertEqual(target["FLAGS"], flags)
                self.assertEqual(target["syscall_aliases"]("read", "open"),
                                 [] if arch == "aarch64" else
                                 ["--defsym=read=_read", "--defsym=open=_open"])

    # ------------------------=
    # FUNC: test_rejects_host_targets
    # DESC: Refuses accidental host-runtime and unsupported cross-build targets.
    # ------------------=
    def test_rejects_host_targets(self):
        for arch in ("", "arm64", "x86_64-apple-darwin", "x86_64-linux-gnu"):
            with self.subTest(arch=arch), self.assertRaises(ValueError):
                self.load(arch)


if __name__ == "__main__":
    unittest.main()
