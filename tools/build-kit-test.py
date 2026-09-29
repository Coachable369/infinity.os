"""Behavioral tests for the InfinityOS build authority."""
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import unittest

SPEC = importlib.util.spec_from_file_location(
    "build_kit", Path(__file__).with_name("build-kit.py"))
build_kit = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(build_kit)


class BuildKitTests(unittest.TestCase):
    # ------------------------=
    # FUNC: setUp
    # DESC: Creates an isolated project-shaped fixture beneath the managed repository build tree.
    # ------------------=
    def setUp(self):
        self.root = build_kit.PROJECT_ROOT / "build/build-kit-test-project"
        self.root.mkdir(parents=True)
        (self.root / "Cargo.toml").write_text("[workspace]\n")
        (self.root / "Makefile").write_text("all:\n\t@true\n")
        tools = self.root / "tools"
        tools.mkdir()
        shutil.copy2(build_kit.PROJECT_ROOT / "tools/build-workspace.py", tools)
        shutil.copy2(build_kit.PROJECT_ROOT / "tools/log-retention.py", tools)
        self.config = self.root / "build-kit.toml"
        self.config.write_text(
            "version = 1\nproject = \"InfinityOS\"\n"
            "[paths]\nscratch = \"build\"\ntemporary = \"build/tmp\"\n"
            "releases = \"builds\"\nmanifests = \"builds/manifests\"\n"
            "models = \"model-cache\"\npatches = \"third_party/patches\"\n"
            "logs = \"build/logs\"\n"
            "[log_retention]\nremove_empty_release_logs = true\n"
            "root_crash_globs = [\"rustc-ice-*.txt\"]\n"
            "[profiles.clean_probe]\ncommand = [\"true\"]\nclean = true\n"
            "[profiles.incremental_probe]\ncommand = [\"true\"]\nclean = false\n")

    # ------------------------=
    # FUNC: tearDown
    # DESC: Removes only the isolated build-kit fixture after each behavioral test.
    # ------------------=
    def tearDown(self):
        if self.root.exists():
            shutil.rmtree(self.root)

    # ------------------------=
    # FUNC: test_run_cleans_contains_locks_and_records
    # DESC: Exercises a real child process and verifies the complete managed build lifecycle.
    # ------------------=
    def test_run_cleans_contains_locks_and_records(self):
        stale = self.root / "build/stale.bin"
        release = self.root / "builds/previous.iso"
        model = self.root / "model-cache/model.gguf"
        empty_log = self.root / "builds/obsolete/empty.log"
        evidence_log = self.root / "builds/evidence/run.log"
        crash_log = self.root / "rustc-ice-fixture.txt"
        for path in (stale, release, model):
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(b"data")
        empty_log.parent.mkdir(parents=True)
        empty_log.write_bytes(b"")
        evidence_log.parent.mkdir(parents=True)
        evidence_log.write_bytes(b"measured evidence")
        crash_log.write_bytes(b"obsolete crash report")
        observed = self.root / "observed.json"
        probe = (
            "import json,os,pathlib;"
            f"pathlib.Path({str(observed)!r}).write_text(json.dumps({{"
            "'cwd':os.getcwd(),'tmp':os.environ['TMPDIR'],"
            "'cargo':os.environ['CARGO_TARGET_DIR'],"
            "'active':os.environ['INFINITY_BUILD_KIT_ACTIVE']}))")

        self.config.write_text(self.config.read_text().replace(
            "command = [\"true\"]\nclean = true",
            f"command = [{json.dumps(sys.executable)}, \"-c\", {json.dumps(probe)}]\nclean = true"))
        result = build_kit.execute("clean_probe", [], self.root, self.config)

        state = json.loads(observed.read_text())
        manifests = tuple((self.root / "builds/manifests").glob("*.json"))
        self.assertEqual(result, 0)
        self.assertFalse(stale.exists())
        self.assertEqual(release.read_bytes(), b"data")
        self.assertEqual(model.read_bytes(), b"data")
        self.assertFalse(empty_log.exists())
        self.assertEqual(evidence_log.read_bytes(), b"measured evidence")
        self.assertFalse(crash_log.exists())
        self.assertEqual(state["cwd"], str(self.root))
        self.assertEqual(state["tmp"], str(self.root / "build/tmp"))
        self.assertEqual(state["cargo"], str(self.root / "build/cargo"))
        self.assertEqual(state["active"], "1")
        self.assertEqual(len(manifests), 1)
        record = json.loads(manifests[0].read_text())
        self.assertEqual(record["result"], "passed")
        self.assertTrue(record["cleaned"])
        self.assertEqual(record["pruned_logs"], 2)
        self.assertFalse((self.root / "builds/.build-kit.lock").exists())

    # ------------------------=
    # FUNC: test_incremental_run_preserves_cache_and_observes_changed_input
    # DESC: Verifies incremental builds retain prior products while processing newly changed inputs.
    # ------------------=
    def test_incremental_run_preserves_cache_and_observes_changed_input(self):
        source = self.root / "source.txt"
        result_path = self.root / "build/result.txt"
        cache = self.root / "build/compiler-cache.bin"
        source.write_text("first")
        cache.parent.mkdir(parents=True)
        cache.write_bytes(b"compiled cache")
        probe = (
            "from pathlib import Path;"
            f"Path({str(result_path)!r}).write_text(Path({str(source)!r}).read_text())")

        first = build_kit.execute(
            "run", [sys.executable, "-c", probe], self.root, self.config)
        source.write_text("second")
        second = build_kit.execute(
            "run", [sys.executable, "-c", probe], self.root, self.config)

        manifests = tuple((self.root / "builds/manifests").glob("*.json"))
        self.assertEqual((first, second), (0, 0))
        self.assertEqual(cache.read_bytes(), b"compiled cache")
        self.assertEqual(result_path.read_text(), "second")
        self.assertEqual(len(manifests), 2)
        for manifest in manifests:
            record = json.loads(manifest.read_text())
            self.assertFalse(record["cleaned"])
            self.assertEqual(record["removed_bytes"], 0)

    # ------------------------=
    # FUNC: test_live_lock_rejects_concurrent_build
    # DESC: Verifies a second build cannot mutate shared outputs while an owner is active.
    # ------------------=
    def test_live_lock_rejects_concurrent_build(self):
        releases = self.root / "builds"
        releases.mkdir()
        (releases / ".build-kit.lock").write_text(json.dumps({
            "build_id": "active-fixture", "pid": os.getpid(), "root": str(self.root)}))
        with self.assertRaises(RuntimeError):
            build_kit.execute("incremental_probe", [], self.root, self.config)

    # ------------------------=
    # FUNC: test_subordinate_builds_reject_unmanaged_execution
    # DESC: Verifies every dedicated build script fails before work without build-kit context.
    # ------------------=
    def test_subordinate_builds_reject_unmanaged_execution(self):
        scripts = (
            "tools/build-hermes.sh",
            "tools/build-icon-themes.sh",
            "tools/build-native-c-toolchain.sh",
            "tools/build-qwen.sh",
            "tools/build-system-sound.sh",
            "tools/installer-designer/build.sh",
            "tools/native-c-probe/build-compiler-runtime.sh",
            "tools/native-c-probe/build-hello.sh",
            "tools/native-c-probe/build-sync.sh",
            "tools/native-c-probe/build-tls.sh",
        )
        environment = os.environ.copy()
        for name in ("INFINITY_BUILD_KIT_ACTIVE", "INFINITY_PROJECT_ROOT", "TMPDIR"):
            environment.pop(name, None)
        for relative in scripts:
            interpreter = "zsh" if relative == "tools/installer-designer/build.sh" else "sh"
            result = subprocess.run(
                [interpreter, str(build_kit.PROJECT_ROOT / relative)],
                cwd=build_kit.PROJECT_ROOT,
                env=environment,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                check=False)
            self.assertEqual(result.returncode, 64, relative)

    # ------------------------=
    # FUNC: test_iso_publisher_requires_build_sh_authority
    # DESC: Verifies managed subordinate execution still cannot publish an ISO outside build.sh.
    # ------------------=
    def test_iso_publisher_requires_build_sh_authority(self):
        environment = os.environ.copy()
        environment.update({
            "INFINITY_BUILD_KIT_ACTIVE": "1",
            "INFINITY_PROJECT_ROOT": str(build_kit.PROJECT_ROOT),
            "TMPDIR": str(build_kit.PROJECT_ROOT / "build/tmp"),
        })
        environment.pop("INFINITY_ISO_BUILD_AUTHORITY", None)
        result = subprocess.run(
            ["sh", str(build_kit.PROJECT_ROOT / "tools/build-qwen.sh")],
            cwd=build_kit.PROJECT_ROOT,
            env=environment,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            check=False,
        )
        self.assertEqual(result.returncode, 2)


if __name__ == "__main__":
    unittest.main()
