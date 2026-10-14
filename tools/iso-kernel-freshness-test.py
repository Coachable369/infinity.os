#!/usr/bin/env python3
"""Behavioral regression for rejecting stale live or installed ISO kernels."""

from __future__ import annotations

import importlib.util
import os
from pathlib import Path
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parent.parent
SPEC = importlib.util.spec_from_file_location("iso_kernel_freshness", ROOT / "tools/iso-kernel-freshness.py")
FRESHNESS = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(FRESHNESS)


class KernelFreshnessTests(unittest.TestCase):
    # ------------------------=
    # FUNC: setUp
    # DESC: Builds a small real FAT and ISO fixture carrying distinct live and installed kernels.
    # ------------------=
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(dir=ROOT / "build/tmp", prefix="kernel-freshness-test-")
        self.work = Path(self.temporary.name)
        self.live = self.work / "live.elf"
        self.installed = self.work / "installed.elf"
        self.live.write_bytes(bytes(range(251)) * 17)
        self.installed.write_bytes(bytes(reversed(range(251))) * 19)
        tree = self.work / "fat"
        (tree / "EFI/INFINITY/PAYLOAD").mkdir(parents=True)
        (tree / "EFI/INFINITY/KERNEL.ELF").write_bytes(self.live.read_bytes())
        (tree / "EFI/INFINITY/PAYLOAD/P1-000.BIN").write_bytes(self.installed.read_bytes()[:2000])
        (tree / "EFI/INFINITY/PAYLOAD/P1-001.BIN").write_bytes(self.installed.read_bytes()[2000:])
        self.efi = self.work / "efi.img"
        subprocess.run(["mkfile", "-n", "16m", str(self.efi)], check=True)
        subprocess.run(["mformat", "-F", "-i", str(self.efi), "::"], check=True)
        subprocess.run(["mcopy", "-i", str(self.efi), "-s", str(tree / "EFI"), "::"], check=True)
        stage = self.work / "iso"
        stage.mkdir()
        (stage / "efi.img").write_bytes(self.efi.read_bytes())
        self.iso = self.work / "InfinityOS-test.iso"
        subprocess.run(
            ["xorriso", "-as", "mkisofs", "-R", "-V", "KERNEL_FRESHNESS", "-e", "efi.img", "-no-emul-boot", "-o", str(self.iso), str(stage)],
            check=True,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )

    # ------------------------=
    # FUNC: tearDown
    # DESC: Removes only the repository-local fixture owned by this test.
    # ------------------=
    def tearDown(self):
        self.temporary.cleanup()

    # ------------------------=
    # FUNC: test_accepts_current_live_and_installed_kernels
    # DESC: Accepts a release only when both current build artifacts match its observable bytes.
    # ------------------=
    def test_accepts_current_live_and_installed_kernels(self):
        FRESHNESS.verify_iso_kernels(self.iso, self.live, self.installed)

    # ------------------------=
    # FUNC: test_rejects_each_stale_kernel_path
    # DESC: Rejects independently stale live and installed kernel expectations.
    # ------------------=
    def test_rejects_each_stale_kernel_path(self):
        for artifact in (self.live, self.installed):
            original = artifact.read_bytes()
            artifact.write_bytes(original[:-1] + bytes([original[-1] ^ 0xFF]))
            with self.subTest(artifact=artifact.name):
                with self.assertRaises(RuntimeError):
                    FRESHNESS.verify_iso_kernels(self.iso, self.live, self.installed)
            artifact.write_bytes(original)


if __name__ == "__main__":
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Use ./build-kit run python3 tools/iso-kernel-freshness-test.py")
    unittest.main()
