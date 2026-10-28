"""Behavioral probe input identity checks; never executes candidate Python."""
from pathlib import Path
import hashlib
import importlib.util
import os
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[2]

# ------------------------=
# FUNC: rejected
# DESC: Checks typed helper rejection while leaving its diagnostic wording outside the test oracle.
# ------------------=
def rejected(function, *arguments):
    try:
        function(*arguments)
    except ValueError:
        return
    raise AssertionError("invalid probe snapshot operation accepted")

# ------------------------=
# FUNC: main
# DESC: Verifies exact copied bytes, original-file independence, bounds and tamper detection using repository-local disposable inputs.
# ------------------=
def main():
    assert os.environ.get("INFINITY_BUILD_KIT_ACTIVE") == "1"
    spec = importlib.util.spec_from_file_location("native_probe", Path(__file__).with_name("run.py"))
    probe = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(probe)
    parent = ROOT / "build/behavior-tests/python-source-snapshot"
    parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="case-", dir=parent) as directory:
        work = Path(directory)
        source = work / "candidate.py"
        marker = work / "must-not-execute"
        original = f"open({str(marker)!r}, 'w').write('executed')\n".encode() + b"\x00\xff"
        source.write_bytes(original)
        snapshot = probe.snapshot_python_source(source, work)
        assert snapshot["path"] != source
        assert snapshot["path"].is_relative_to(work)
        assert snapshot["path"].read_bytes() == original
        assert snapshot["size"] == len(original)
        assert snapshot["sha256"] == hashlib.sha256(original).hexdigest()
        assert snapshot["path"].stat().st_mode & 0o222 == 0
        assert source.read_bytes() == original
        source.write_bytes(b"replaced while guest runs\n")
        probe.verify_python_snapshot(snapshot)
        assert snapshot["path"].read_bytes() == original
        source.unlink()
        probe.verify_python_snapshot(snapshot)
        assert not marker.exists()

        snapshot["path"].chmod(0o644)
        snapshot["path"].write_bytes(b"x" * snapshot["size"])
        rejected(probe.verify_python_snapshot, snapshot)

        for size in (0, probe.PYTHON_SOURCE_LIMIT):
            source.write_bytes(b"z" * size)
            bounded = probe.snapshot_python_source(source, work)
            assert bounded["size"] == size
            assert bounded["path"].read_bytes() == b"z" * size
            probe.verify_python_snapshot(bounded)
        before = set(work.iterdir())
        source.write_bytes(b"z" * (probe.PYTHON_SOURCE_LIMIT + 1))
        rejected(probe.snapshot_python_source, source, work)
        assert set(work.iterdir()) == before
        rejected(probe.snapshot_python_source, Path("/dev/null"), work)
        rejected(probe.snapshot_python_source, source, Path("/"))
        assert not marker.exists()
        before = set(work.iterdir())
        for option in ("--validate-virtualbox-serial", "--validate-virtualbox-compiler-serial"):
            result = subprocess.run(
                [sys.executable, str(Path(__file__).with_name("run.py")),
                 "--python-source", str(source), option, str(work / "uncaptured.bin")],
                capture_output=True, check=False)
            assert result.returncode == 2
            assert set(work.iterdir()) == before
            assert not marker.exists()

if __name__ == "__main__":
    main()
