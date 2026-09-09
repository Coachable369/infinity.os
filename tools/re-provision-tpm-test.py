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
        work = Path(directory)
        binary, state, vm = (work / name for name in ("bin", "state", "vm"))
        for path in (binary, state, vm):
            path.mkdir()
        for name in ("VBoxManage", "uname", "file"):
            target = binary / name
            shutil.copyfile(root / "tools/test-fixtures/re-provision" / name, target)
            target.chmod(0o700)
        iso = work / "test.iso"
        iso.touch()
        environment = dict(os.environ, PATH=f"{binary}:{os.environ['PATH']}",
            TEST_STATE_PATH=str(state), TEST_VM_DIRECTORY=str(vm), TEST_ISO_PATH=str(iso),
            TEST_MEDIUM_MODE="orphan", INFINITY_VBOXMANAGE=str(binary / "VBoxManage"))
        for key in ("INFINITY_VM_MEMORY_MB", "INFINITY_VM_CPU_COUNT", "INFINITY_VM_DISK_SIZE_MB"):
            environment.pop(key, None)
        result = subprocess.run([str(root / "re-provision.sh"), "infinityos-4", str(iso)],
            env=environment, capture_output=True, timeout=20)
        assert result.returncode == 0, result.stderr.decode(errors="replace")
        assert int((state / "tpm-version").read_text()) == 200
        assert (vm / "infinityos-4.vdi").is_file()
        assert not (state / "unregistered").exists()

if __name__ == "__main__":
    main()
