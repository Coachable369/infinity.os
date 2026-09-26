"""Behavioral tests for bounded InfinityOS log retention."""
import importlib.util
from pathlib import Path
import shutil
import unittest

SPEC = importlib.util.spec_from_file_location(
    "log_retention", Path(__file__).with_name("log-retention.py"))
log_retention = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(log_retention)


class LogRetentionTests(unittest.TestCase):
    # ------------------------=
    # FUNC: setUp
    # DESC: Creates repository-shaped log evidence beneath managed scratch storage.
    # ------------------=
    def setUp(self):
        self.root = log_retention.PROJECT_ROOT / "build/log-retention-test-project"
        self.root.mkdir(parents=True)
        (self.root / "build-kit.toml").write_text("version = 1\n")
        (self.root / "Makefile").write_text("all:\n\t@true\n")

    # ------------------------=
    # FUNC: tearDown
    # DESC: Removes the isolated fixture after each test.
    # ------------------=
    def tearDown(self):
        if self.root.exists():
            shutil.rmtree(self.root)

    # ------------------------=
    # FUNC: test_prune_removes_only_valueless_logs
    # DESC: Proves empty logs and crash dumps are removed while non-empty evidence survives.
    # ------------------=
    def test_prune_removes_only_valueless_logs(self):
        empty = self.root / "builds/run/empty.log"
        evidence = self.root / "builds/run/evidence.log"
        unrelated = self.root / "builds/run/empty.json"
        orphan = self.root / "builds/orphan/empty.log"
        crash = self.root / "rustc-ice-fixture.txt"
        for path, content in (
                (empty, b""), (evidence, b"timing data"),
                (unrelated, b""), (orphan, b""), (crash, b"crash")):
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(content)

        removed = log_retention.prune(self.root, {
            "remove_empty_release_logs": True,
            "root_crash_globs": ["rustc-ice-*.txt"],
        })

        self.assertEqual({entry["path"] for entry in removed}, {
            "builds/orphan/empty.log", "builds/run/empty.log",
            "rustc-ice-fixture.txt"})
        self.assertFalse(empty.exists())
        self.assertTrue(empty.parent.exists())
        self.assertFalse(orphan.parent.exists())
        self.assertFalse(crash.exists())
        self.assertEqual(evidence.read_bytes(), b"timing data")
        self.assertTrue(unrelated.exists())

    # ------------------------=
    # FUNC: test_unsafe_pattern_is_rejected
    # DESC: Proves configuration cannot expand cleanup beyond root-level crash files.
    # ------------------=
    def test_unsafe_pattern_is_rejected(self):
        with self.assertRaises(ValueError):
            log_retention.candidates(self.root, {"root_crash_globs": ["*"]})


if __name__ == "__main__":
    unittest.main()
