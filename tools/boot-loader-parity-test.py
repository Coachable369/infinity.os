#!/usr/bin/env python3
"""Behavioral coverage for binary live/installed boot-loader parity rejection."""
import importlib.util
import os
import struct
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parent.parent
SPEC = importlib.util.spec_from_file_location("bundle", ROOT / "tools/full-bundle-iso-test.py")
BUNDLE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(BUNDLE)


class LoaderParityTests(unittest.TestCase):
    # ------------------------=
    # FUNC: setUp
    # DESC: Creates bounded binary artifact fixtures inside the repository workspace.
    # ------------------=
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(dir=ROOT / "build/tmp", prefix="loader-parity-")
        self.work = Path(self.temporary.name)
        image = bytearray(bytes(range(256)) * 4)
        image[:2] = b"MZ"
        struct.pack_into("<I", image, 0x3C, 0x80)
        image[0x80:0x84] = b"PE\0\0"
        struct.pack_into("<H", image, 0x84, 0x8664)
        struct.pack_into("<H", image, 0x98, 0x20B)
        struct.pack_into("<Q", image, 0xB0, 0x2000000)
        struct.pack_into("<I", image, 0xD0, 0x24000)
        self.expected = bytes(image)

    # ------------------------=
    # FUNC: tearDown
    # DESC: Removes only this test's generated artifact fixture directory.
    # ------------------=
    def tearDown(self):
        self.temporary.cleanup()

    # ------------------------=
    # FUNC: verify
    # DESC: Exercises the production comparator with binary extraction results for all four boot paths.
    # ------------------=
    def verify(self, architecture, copies, suffix=""):
        name = "BOOTAA64.EFI" if architecture == "aarch64" else "BOOTX64.EFI"
        loader = self.work / "build" / architecture / name
        loader.parent.mkdir(parents=True, exist_ok=True)
        loader.write_bytes(self.expected)
        extracted = iter(copies[:3])

        # ------------------------=
        # FUNC: extract_fat
        # DESC: Supplies each isolated FAT extraction's exact binary artifact bytes.
        # ------------------=
        def extract_fat(_command):
            return next(extracted)

        # ------------------------=
        # FUNC: extract_iso
        # DESC: Supplies the top-level boot artifact without creating an incomplete ISO.
        # ------------------=
        def extract_iso(command, **_options):
            Path(command[-1]).write_bytes(copies[3])

        with patch.object(BUNDLE, "ROOT", self.work), \
                patch.object(BUNDLE.subprocess, "check_output", side_effect=extract_fat), \
                patch.object(BUNDLE.subprocess, "run", side_effect=extract_iso):
            BUNDLE.verify_boot_loader(
                self.work / f"InfinityOS-{architecture}.iso{suffix}", self.work / "live.img",
                self.work / "installed.img", self.work,
            )

    # ------------------------=
    # FUNC: test_identical_boot_paths
    # DESC: Accepts complete byte-identical native boot artifacts for both supported architectures.
    # ------------------=
    def test_identical_boot_paths(self):
        for architecture in ("aarch64", "x86_64"):
            for suffix in ("", ".partial"):
                with self.subTest(architecture=architecture, suffix=suffix):
                    self.verify(architecture, [self.expected] * 4, suffix)

    # ------------------------=
    # FUNC: test_rejects_conflicting_loader_extent
    # DESC: Rejects matching binaries whose preferred allocation overlaps the kernel, the speech arena, or low firmware memory.
    # ------------------=
    def test_rejects_conflicting_loader_extent(self):
        original = self.expected
        for base, size in ((0, 0x24000), (0x140000000, 0x24000),
                           (0x3FFF000, 0x24000), (0x2000000, 0)):
            with self.subTest(base=base, size=size):
                image = bytearray(original)
                struct.pack_into("<Q", image, 0xB0, base)
                struct.pack_into("<I", image, 0xD0, size)
                self.expected = bytes(image)
                with self.assertRaises(AssertionError):
                    self.verify("x86_64", [self.expected] * 4)

    # ------------------------=
    # FUNC: test_rejects_each_stale_boot_path
    # DESC: Rejects a one-byte mutation or empty artifact at every independent live/installed boot path.
    # ------------------=
    def test_rejects_each_stale_boot_path(self):
        for architecture in ("aarch64", "x86_64"):
            for suffix in ("", ".partial"):
                for index in range(4):
                    for corrupt in (self.expected[:-1] + b"\0", b""):
                        with self.subTest(architecture=architecture, suffix=suffix, copy=index, length=len(corrupt)):
                            copies = [self.expected] * 4
                            copies[index] = corrupt
                            with self.assertRaises(RuntimeError):
                                self.verify(architecture, copies, suffix)

    # ------------------------=
    # FUNC: test_rejects_unknown_release_names
    # DESC: Rejects invalid architecture and filename suffixes even when all supplied boot artifacts match.
    # ------------------=
    def test_rejects_unknown_release_names(self):
        for architecture, suffix in (("arm64", ""), ("aarch64", ".partial.partial"), ("x86_64", ".backup")):
            with self.subTest(architecture=architecture, suffix=suffix):
                with self.assertRaises(RuntimeError):
                    self.verify(architecture, [self.expected] * 4, suffix)


if __name__ == "__main__":
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Use ./build-kit run python3 tools/boot-loader-parity-test.py")
    unittest.main()
