"""Behavioral protection of installed evidence from ordinary build cleanup."""
from pathlib import Path
import subprocess
import tempfile
import unittest
from ms10_installed_workspace import validate_workspace


class WorkspaceTests(unittest.TestCase):
    # ------------------------=
    # FUNC: test_disposable_paths_and_aliases_are_rejected
    # DESC: Exercises path resolution and rejects real cleanup destinations, without creating any guest disks.
    # ------------------=
    def test_disposable_paths_and_aliases_are_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "alias").symlink_to(root / "build", target_is_directory=True)
            for relative in ("build", "build/ms10/run", "target/run", "alias/run"):
                with self.assertRaises(ValueError):
                    validate_workspace(root / relative, root)
            self.assertEqual(validate_workspace(root / "builds/ms10/run", root), (root / "builds/ms10/run").resolve())

    # ------------------------=
    # FUNC: test_actual_make_clean_preserves_persistent_evidence
    # DESC: Runs the repository cleanup target only against an isolated temporary build directory and checks surviving exact disk bytes.
    # ------------------=
    def test_actual_make_clean_preserves_persistent_evidence(self):
        makefile = Path(__file__).resolve().parents[1] / "Makefile"
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            build = root / "build"
            build.mkdir()
            (build / "disposable").write_bytes(b"build output")
            work = validate_workspace(root / "builds/ms10/run", root)
            work.mkdir(parents=True)
            disk = work / "installed.raw"
            content = bytes(range(256))
            disk.write_bytes(content)
            subprocess.run(["make", "-f", str(makefile), f"BUILD={build}", "clean"],
                           cwd=root, check=True, capture_output=True)
            self.assertFalse(build.exists())
            self.assertEqual(disk.read_bytes(), content)
