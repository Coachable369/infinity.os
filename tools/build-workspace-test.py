"""Behavioral tests for bounded, repository-local build cleanup."""
import importlib.util
from pathlib import Path
import unittest

SPEC = importlib.util.spec_from_file_location(
    "workspace", Path(__file__).with_name("build-workspace.py"))
workspace = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(workspace)


class BuildWorkspaceTests(unittest.TestCase):
    # ------------------------=
    # FUNC: test_clean_removes_only_managed_outputs
    # DESC: Proves cleanup removes prior products while preserving source, releases and models.
    # ------------------=
    def test_clean_removes_only_managed_outputs(self):
        fixture = workspace.PROJECT_ROOT / "build/workspace-clean-test"
        fixture.mkdir(parents=True, exist_ok=True)
        (fixture / "Cargo.toml").write_text("[workspace]\n")
        (fixture / "Makefile").write_text("all:\n\t@true\n")
        for relative in workspace.MANAGED_DIRECTORIES:
            generated = fixture / relative / "generated.bin"
            generated.parent.mkdir(parents=True, exist_ok=True)
            generated.write_bytes(b"generated")
        source = fixture / "kernel/source.rs"
        release = fixture / "builds/InfinityOS.iso"
        model = fixture / "model-cache/model.gguf"
        for durable in (source, release, model):
            durable.parent.mkdir(parents=True, exist_ok=True)
            durable.write_bytes(b"durable")

        removed = workspace.clean(fixture)

        self.assertEqual({entry["path"] for entry in removed}, set(workspace.MANAGED_DIRECTORIES))
        self.assertTrue(all(durable.read_bytes() == b"durable" for durable in (source, release, model)))
        self.assertTrue((fixture / "build/tmp").is_dir())

    # ------------------------=
    # FUNC: test_clean_rejects_symlinked_root
    # DESC: Proves cleanup cannot be redirected outside its validated project boundary.
    # ------------------=
    def test_clean_rejects_symlinked_root(self):
        fixture = workspace.PROJECT_ROOT / "build/workspace-clean-test"
        fixture.mkdir(parents=True, exist_ok=True)
        (fixture / "Cargo.toml").write_text("[workspace]\n")
        (fixture / "Makefile").write_text("all:\n\t@true\n")
        link = workspace.PROJECT_ROOT / "build/workspace-clean-link"
        link.unlink(missing_ok=True)
        link.symlink_to(fixture, target_is_directory=True)
        with self.assertRaises(ValueError):
            workspace.clean(link)
        link.unlink()

    # ------------------------=
    # FUNC: tearDown
    # DESC: Removes only the test fixture through its validated managed parent.
    # ------------------=
    def tearDown(self):
        fixture = workspace.PROJECT_ROOT / "build/workspace-clean-test"
        link = workspace.PROJECT_ROOT / "build/workspace-clean-link"
        link.unlink(missing_ok=True)
        if fixture.exists():
            import shutil
            shutil.rmtree(fixture)


if __name__ == "__main__":
    unittest.main()
