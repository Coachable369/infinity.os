#!/usr/bin/env python3
"""Behavioral launcher checks with isolated build/LaunchServices substitutes."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]


class StudioLauncherTests(unittest.TestCase):
    # ------------------------=
    # FUNC: setUp
    # DESC: Creates repository-local isolated command substitutes and a Studio bundle.
    # ------------------=
    def setUp(self):
        self.fixture = tempfile.TemporaryDirectory(prefix="studio launcher ", dir=ROOT / "build/tmp")
        self.root = Path(self.fixture.name)
        shutil.copy2(ROOT / "run-infinitystudio.sh", self.root)
        self.bin = self.root / "bin"
        self.bin.mkdir()
        self.events = self.root / "events.jsonl"
        stub = (
            f"#!{sys.executable}\n"
            "import json,os,pathlib,sys\n"
            "name=pathlib.Path(sys.argv[0]).name\n"
            "with open(os.environ['STUDIO_TEST_EVENTS'],'a') as stream:\n"
            " stream.write(json.dumps([name,sys.argv[1:],os.getcwd()])+'\\n')\n"
            "if name=='uname': print(os.environ.get('STUDIO_TEST_OS','Darwin'))\n"
            "sys.exit(int(os.environ.get('STUDIO_TEST_BUILD_EXIT','0')) if name=='build-kit' else 0)\n"
        )
        for path in (self.root / "build-kit", self.bin / "open", self.bin / "uname"):
            path.write_text(stub)
            path.chmod(0o755)
        self.app = self.root / "builds/InfinityOS Installer Studio.app"
        self.executable = self.app / "Contents/MacOS/InfinityInstallerStudio"
        self.executable.parent.mkdir(parents=True)
        self.executable.touch()
        self.executable.chmod(0o755)
        self.env = dict(os.environ, PATH=f"{self.bin}:{os.environ['PATH']}",
                        STUDIO_TEST_EVENTS=str(self.events))

    # ------------------------=
    # FUNC: tearDown
    # DESC: Removes only the test-owned fixture directory.
    # ------------------=
    def tearDown(self):
        self.fixture.cleanup()

    # ------------------------=
    # FUNC: run_launcher
    # DESC: Executes the actual Bash launcher from outside its repository root.
    # ------------------=
    def run_launcher(self, *arguments):
        result = subprocess.run(["/bin/bash", str(self.root / "run-infinitystudio.sh"), *arguments],
                                cwd=ROOT, env=self.env, capture_output=True)
        events = [json.loads(line) for line in self.events.read_text().splitlines()] if self.events.exists() else []
        return result.returncode, events

    # ------------------------=
    # FUNC: test_build_then_launch_from_foreign_directory
    # DESC: Checks ordered clean-profile invocation and exact app path including spaces.
    # ------------------=
    def test_build_then_launch_from_foreign_directory(self):
        status, events = self.run_launcher()
        self.assertEqual(status, 0)
        self.assertEqual(events[1:], [
            ["build-kit", ["studio-clean"], str(self.root)],
            ["open", [str(self.app)], str(self.root)],
        ])

    # ------------------------=
    # FUNC: test_failed_build_never_opens_stale_app
    # DESC: Verifies nonzero build status propagates despite an existing app bundle.
    # ------------------=
    def test_failed_build_never_opens_stale_app(self):
        self.env["STUDIO_TEST_BUILD_EXIT"] = "23"
        status, events = self.run_launcher()
        self.assertEqual(status, 23)
        self.assertEqual([event[0] for event in events], ["uname", "build-kit"])

    # ------------------------=
    # FUNC: test_missing_app_fails_without_launch
    # DESC: Rejects a successful build command that did not produce its executable.
    # ------------------=
    def test_missing_app_fails_without_launch(self):
        self.executable.unlink()
        status, events = self.run_launcher()
        self.assertEqual(status, 1)
        self.assertEqual([event[0] for event in events], ["uname", "build-kit"])

    # ------------------------=
    # FUNC: test_no_launch
    # DESC: Builds successfully without invoking LaunchServices when requested.
    # ------------------=
    def test_no_launch(self):
        status, events = self.run_launcher("--no-launch")
        self.assertEqual(status, 0)
        self.assertEqual([event[0] for event in events], ["uname", "build-kit"])

    # ------------------------=
    # FUNC: test_rejects_invalid_arguments_and_non_macos
    # DESC: Ensures unsupported invocations never clean or build anything.
    # ------------------=
    def test_rejects_invalid_arguments_and_non_macos(self):
        status, events = self.run_launcher("--invalid")
        self.assertEqual((status, events), (2, []))
        self.env["STUDIO_TEST_OS"] = "Linux"
        status, events = self.run_launcher()
        self.assertEqual(status, 1)
        self.assertEqual([event[0] for event in events], ["uname"])


if __name__ == "__main__":
    unittest.main()
