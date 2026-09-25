"""Behavioral FAT staging tests using real mtools images and byte extraction."""
import importlib.util
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch
import shutil

spec = importlib.util.spec_from_file_location('staging', Path(__file__).with_name('iso-staging.py'))
staging = importlib.util.module_from_spec(spec)
spec.loader.exec_module(staging)

class StagingTests(unittest.TestCase):
    # ------------------------=
    # FUNC: test_real_fat_roundtrip
    # DESC: Verifies bytes survive consumption, sizing and nested FAT-directory creation.
    # ------------------=
    def test_real_fat_roundtrip(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            tree = root / 'tree'
            folder = tree / 'EFI' / 'BOOT'
            folder.mkdir(parents=True)
            data = bytes(range(256)) * 4096
            source = folder / 'BOOTAA64.EFI'
            source.write_bytes(data)
            image = root / 'fat.img'
            size = staging.allocate(tree, image)
            self.assertEqual(image.stat().st_size, size)
            self.assertGreaterEqual(size - len(data), 128 * staging.MIB)
            subprocess.run(['mformat', '-F', '-i', str(image), '::'], check=True)
            staging.consume(tree, image)
            self.assertFalse(source.exists())
            self.assertEqual(subprocess.check_output(['mtype', '-i', str(image),
                             '::/EFI/BOOT/BOOTAA64.EFI']), data)

    # ------------------------=
    # FUNC: test_failed_copy_preserves_source
    # DESC: Ensures copy failure cannot delete the only private source file.
    # ------------------=
    def test_failed_copy_preserves_source(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            tree = root / 'tree'
            tree.mkdir()
            source = tree / 'data.bin'
            source.write_bytes(b'private staging test')
            with self.assertRaises(subprocess.CalledProcessError):
                staging.consume(tree, root / 'absent.img')
            self.assertEqual(source.read_bytes(), b'private staging test')

    # ------------------------=
    # FUNC: test_capacity_guard
    # DESC: Rejects insufficient capacity through structured disk state before allocation.
    # ------------------=
    def test_capacity_guard(self):
        state = type(shutil.disk_usage('.'))(100, 90, 10)
        with patch.object(staging.shutil, 'disk_usage', return_value=state):
            staging.require_space('.', 10)
            with self.assertRaises(RuntimeError):
                staging.require_space('.', 11)

if __name__ == '__main__':
    unittest.main()
