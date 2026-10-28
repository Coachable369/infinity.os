"""Behavioral deployment checks using an isolated SSH transport, never production."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
FILES = (
    "index.html whitepaper.html invest.html styles.css whitepaper.css navigation.js app.js art.js "
    "emblem-geometry.js scene-math.mjs assets/emblem-outline.json "
    "assets/brand.webp assets/infinity-emblem.webp assets/infinity-detail.webp "
    "assets/screenshots/sign-in-960.webp assets/screenshots/sign-in-1600.webp assets/screenshots/sign-in-full.webp "
    "assets/screenshots/desktop-ai-960.webp assets/screenshots/desktop-ai-1600.webp assets/screenshots/desktop-ai-full.webp "
    "assets/screenshots/app-library-960.webp assets/screenshots/app-library-1600.webp assets/screenshots/app-library-full.webp "
    "assets/Inter-Variable.ttf assets/vendor/three.module.min.js assets/vendor/three.core.min.js "
    "assets/vendor/BufferGeometryUtils.js assets/OFL-inter.txt assets/LICENSE-lucide.txt assets/LICENSE-three.txt"
).split()
TRANSPORT = '''#!/usr/bin/env python3
import io,json,os,sys,tarfile
from pathlib import Path
log=Path(os.environ["TRANSPORT_LOG"])
events=json.loads(log.read_text()) if log.exists() else []
data=sys.stdin.buffer.read()
event={"args":sys.argv[1:]}
if data[:2]==b"\\x1f\\x8b":
    with tarfile.open(fileobj=io.BytesIO(data),mode="r:gz") as archive:
        event["files"]={member.name:archive.extractfile(member).read().decode() for member in archive if member.isfile()}
events.append(event)
log.write_text(json.dumps(events))
sys.exit(23 if len(events)==int(os.environ.get("FAIL_STEP","0")) else 0)
'''


class DeployTest(unittest.TestCase):
    # ------------------------=
    # FUNC: setUp
    # DESC: Creates repository-local fixtures and replaces only the external SSH transport.
    # ------------------=
    def setUp(self):
        work = ROOT / "builds/validation/deploy"
        work.mkdir(parents=True, exist_ok=True)
        self.temp = tempfile.TemporaryDirectory(dir=work)
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.site = self.root / "meetinfinityos.com"
        self.site.mkdir()
        shutil.copy2(ROOT / "deploy.sh", self.root / "deploy.sh")
        shutil.copy2(ROOT / "meetinfinityos.com/deploy.sh", self.site / "deploy.sh")
        for name in FILES + ["private.env", "tests/private.txt", "design/private.txt"]:
            target = self.site / name
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text(name)
        binary = self.root / "bin"
        binary.mkdir()
        transport = binary / "sshpass"
        transport.write_text(TRANSPORT)
        transport.chmod(0o755)
        self.log = self.root / "transport.json"
        self.env = dict(os.environ, PATH=f"{binary}:{os.environ['PATH']}",
                        SSHPASS="test-only-not-a-credential", TRANSPORT_LOG=str(self.log),
                        INFINITY_DEPLOY_HOST="test-user@test.invalid")

    # ------------------------=
    # FUNC: run_script
    # DESC: Executes the public root entry point from an unrelated working directory.
    # ------------------=
    def run_script(self, *args):
        return subprocess.run([str(self.root / "deploy.sh"), *args], cwd=self.root / "bin",
                              env=self.env, capture_output=True, timeout=20)

    # ------------------------=
    # FUNC: test_local_modes_never_connect
    # DESC: Verifies help, local preflight and invalid-option handling without any SSH side effects.
    # ------------------=
    def test_local_modes_never_connect(self):
        for args, status in [(('--help',), 0), (('--check',), 0), (('--unknown',), 2), (('--check', 'extra'), 2)]:
            self.assertEqual(self.run_script(*args).returncode, status)
            self.assertFalse(self.log.exists())

    # ------------------------=
    # FUNC: test_missing_file_fails_before_connecting
    # DESC: Ensures a missing public asset prevents remote backup and publication.
    # ------------------=
    def test_missing_file_fails_before_connecting(self):
        (self.site / FILES[-1]).unlink()
        for args in [(), ('--check',)]:
            self.assertEqual(self.run_script(*args).returncode, 1)
            self.assertFalse(self.log.exists())

    # ------------------------=
    # FUNC: test_symlink_rejected
    # DESC: Prevents an allowlisted filename from including a linked private file.
    # ------------------=
    def test_symlink_rejected(self):
        (self.site / "index.html").unlink()
        (self.site / "index.html").symlink_to(self.site / "private.env")
        self.assertEqual(self.run_script('--check').returncode, 1)
        self.assertFalse(self.log.exists())

    # ------------------------=
    # FUNC: test_exact_public_payload
    # DESC: Inspects the real tar stream, connection destination and successful backup/upload/publish sequence.
    # ------------------=
    def test_exact_public_payload(self):
        result = self.run_script()
        self.assertEqual(result.returncode, 0, result.stderr.decode())
        events = json.loads(self.log.read_text())
        self.assertEqual(len(events), 3)
        self.assertEqual(events[1]['files'], {name: name for name in FILES})
        for event in events:
            self.assertIn('test-user@test.invalid', event['args'])
            self.assertNotIn(self.env['SSHPASS'], event['args'])

    # ------------------------=
    # FUNC: test_transport_failures_propagate
    # DESC: Ensures backup, upload and final verification failure stop further deployment stages.
    # ------------------=
    def test_transport_failures_propagate(self):
        for step in range(1, 4):
            self.log.unlink(missing_ok=True)
            self.env['FAIL_STEP'] = str(step)
            result = self.run_script()
            self.assertNotEqual(result.returncode, 0)
            self.assertEqual(len(json.loads(self.log.read_text())), step)

    # ------------------------=
    # FUNC: test_real_local_assets
    # DESC: Checks the actual checkout's executable wrapper and public asset preflight without network access.
    # ------------------=
    def test_real_local_assets(self):
        result = subprocess.run([str(ROOT / 'deploy.sh'), '--check'], cwd=self.root,
                                env=self.env, capture_output=True, timeout=20)
        self.assertEqual(result.returncode, 0, result.stderr.decode())
        self.assertFalse(self.log.exists())


if __name__ == '__main__':
    unittest.main()
