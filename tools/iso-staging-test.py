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
    # FUNC: test_ui_reset_is_exact_and_preserves_other_artifacts
    # DESC: Exercises both managed roots while proving only the two replaceable UI trees are removed.
    # ------------------=
    def test_ui_reset_is_exact_and_preserves_other_artifacts(self):
        temporary_root = staging.ROOT / 'build/tmp'
        temporary_root.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(dir=temporary_root) as temporary:
            root = Path(temporary)
            with patch.object(staging, 'ROOT', root):
                for name in ('installed-fat', 'installed-fat-aarch64'):
                    tree = root / 'build' / name
                    ui = tree / 'EFI/InfinityOS/InfinityUI'
                    for folder in ('Installer/nested', 'DesignKit/Default'):
                        asset = ui / folder / 'obsolete.bin'
                        asset.parent.mkdir(parents=True)
                        asset.write_bytes(b'obsolete')
                    retained = ui / 'Icons/current.bin'
                    retained.parent.mkdir()
                    retained.write_bytes(b'keep')
                    model = tree / 'EFI/INFINITY/PAYLOAD/P4-000.BIN'
                    model.parent.mkdir(parents=True)
                    model.write_bytes(b'model')
                    staging.reset_ui_stage(tree)
                    self.assertFalse((ui / 'Installer').exists())
                    self.assertFalse((ui / 'DesignKit').exists())
                    self.assertEqual(retained.read_bytes(), b'keep')
                    self.assertEqual(model.read_bytes(), b'model')
                    staging.reset_ui_stage(tree)
                for invalid in (root, root / 'build', root / 'build/installed-fat/..',
                                root / 'build/installed-fat/EFI', root / 'outside'):
                    with self.assertRaises(ValueError):
                        staging.reset_ui_stage(invalid)

    # ------------------------=
    # FUNC: test_ui_reset_rejects_symlinks_before_deleting_any_target
    # DESC: Rejects redirected ancestors and nested links while leaving all managed and external bytes unchanged.
    # ------------------=
    def test_ui_reset_rejects_symlinks_before_deleting_any_target(self):
        temporary_root = staging.ROOT / 'build/tmp'
        temporary_root.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(dir=temporary_root) as temporary:
            root = Path(temporary)
            external = root / 'outside'
            external.mkdir()
            protected = external / 'keep.bin'
            protected.write_bytes(b'outside')
            tree = root / 'build/installed-fat'
            ui = tree / 'EFI/InfinityOS/InfinityUI'
            installer = ui / 'Installer/keep.bin'
            installer.parent.mkdir(parents=True)
            installer.write_bytes(b'installed')
            kit = ui / 'DesignKit/Default'
            kit.mkdir(parents=True)
            link = kit / 'redirect'
            link.symlink_to(external, target_is_directory=True)
            with patch.object(staging, 'ROOT', root):
                with self.assertRaises(ValueError):
                    staging.reset_ui_stage(tree)
                self.assertEqual(installer.read_bytes(), b'installed')
                self.assertEqual(protected.read_bytes(), b'outside')
                link.unlink()
                redirected = root / 'build/installed-fat-aarch64'
                redirected.symlink_to(tree, target_is_directory=True)
                with self.assertRaises(ValueError):
                    staging.reset_ui_stage(redirected)
                self.assertEqual(installer.read_bytes(), b'installed')
                self.assertEqual(protected.read_bytes(), b'outside')

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
