"""Behavioral reprovisioning contract: the actual script requests TPM 2.0."""
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

# ------------------------=
# FUNC: main
# DESC: Runs the real reprovisioner against an isolated VirtualBox fixture and asserts numeric TPM configuration plus created disk and unregister state, never human-readable output.
# ------------------=
def main():
    root = Path(__file__).resolve().parent.parent
    with tempfile.TemporaryDirectory(prefix="infinity-tpm-provision-") as directory:
        work = Path(directory).resolve()
        binary, state, vm = (work / name for name in ("bin", "state", "vm"))
        for path in (binary, state, vm):
            path.mkdir()
        for name in ("VBoxManage", "uname", "file"):
            target = binary / name
            shutil.copyfile(root / "tools/test-fixtures/re-provision" / name, target)
            target.chmod(0o700)
        # Execute an isolated repository with media in its canonical builds path.
        (work / "tools").mkdir()
        (work / "builds").mkdir()
        shutil.copyfile(root / "re-provision.sh", work / "re-provision.sh")
        shutil.copyfile(root / "tools/select-install-iso.sh", work / "tools/select-install-iso.sh")
        iso = work / "builds/test.iso"
        # The VirtualBox/file fixtures model ISO structure; the real selector
        # still requires a nonempty artifact before any destructive operation.
        iso.write_bytes(b"infinity-test-iso\x00")
        environment = dict(os.environ, PATH=f"{binary}:{os.environ['PATH']}",
            TEST_STATE_PATH=str(state), TEST_VM_DIRECTORY=str(vm), TEST_ISO_PATH=str(iso),
            TEST_MEDIUM_MODE="orphan", INFINITY_VBOXMANAGE=str(binary / "VBoxManage"))
        for key in ("INFINITY_VM_MEMORY_MB", "INFINITY_VM_CPU_COUNT", "INFINITY_VM_DISK_SIZE_MB"):
            environment.pop(key, None)
        result = subprocess.run(["sh", str(work / "re-provision.sh"), "infinityos-4", str(iso)],
            env=environment, capture_output=True, timeout=20)
        assert result.returncode == 0, result.stderr.decode(errors="replace")
        assert int((state / "tpm-version").read_text()) == 200
        assert int((state / "memory").read_text()) == 22480
        assert (state / "--boot1").read_text().strip() == "disk"
        assert (state / "--boot2").read_text().strip() == "dvd"
        assert (vm / "infinityos-4.vdi").is_file()
        assert not (state / "unregistered").exists()

        for initial, race, failure, succeeds in (
            ("poweroff", False, False, True),
            ("aborted", False, False, True),
            ("saved", False, False, True),
            ("running", False, False, True),
            ("paused", False, False, True),
            ("running", True, False, True),
            ("running", False, True, False),
            ("starting", False, False, False),
            ("inaccessible", False, False, False),
        ):
            shutil.rmtree(state)
            state.mkdir()
            (state / "vm-state").write_text("old\n")
            previous_disk = (vm / "infinityos-4.vdi").read_bytes()
            environment.update(TEST_INITIAL_STATE=initial,
                TEST_STOP_RACE=str(int(race)), TEST_STOP_FAILURE=str(int(failure)))
            result = subprocess.run(["sh", str(work / "re-provision.sh"), "infinityos-4", str(iso)],
                env=environment, capture_output=True, timeout=20)
            assert (result.returncode == 0) == succeeds, (initial, result.stderr.decode(errors="replace"))
            assert (state / "poweroff-attempted").exists() == (initial in ("running", "paused"))
            assert (state / "discarded").exists() == (initial == "saved")
            assert (state / "unregistered").exists() == succeeds
            assert (state / "vm-state").read_text().strip() == ("new" if succeeds else "old")
            if succeeds:
                assert (state / "attached-iso").read_bytes() == iso.read_bytes()
            else:
                assert (vm / "infinityos-4.vdi").read_bytes() == previous_disk

if __name__ == "__main__":
    main()
