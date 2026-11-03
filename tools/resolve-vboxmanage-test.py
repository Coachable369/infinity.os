"""Behavioral executable resolution without contacting a real VM."""
import os
from pathlib import Path
import subprocess
import tempfile

# ------------------------=
# FUNC: main
# DESC: Exercise PATH, app-bundle fallback, explicit override, and missing executable results using isolated executables.
# ------------------=
def main():
    root = Path(__file__).resolve().parent.parent
    with tempfile.TemporaryDirectory() as temporary:
        work = Path(temporary)
        binary = work / "bin"
        binary.mkdir()
        app = work / "VirtualBox.app"
        bundled = app / "Contents/MacOS/VBoxManage"
        bundled.parent.mkdir(parents=True)
        bundled.write_text("#!/bin/sh\nexit 42\n")
        bundled.chmod(0o700)
        path_binary = binary / "VBoxManage"
        path_binary.write_text("#!/bin/sh\nexit 43\n")
        path_binary.chmod(0o700)
        for override, path, bundle, expected in (
            ("", str(binary), str(app), 43),
            ("", str(work), str(app), 42),
            (str(bundled), str(binary), str(app), 42),
            (str(work / "missing"), str(binary), str(app), 1),
            ("", str(work), str(work / "missing.app"), 1),
        ):
            result = subprocess.run(["/bin/sh", "-c",
                '. "$1"; executable=$(resolve_vboxmanage "$2" "$3") || exit 1; "$executable"',
                "resolve-test", str(root / "tools/resolve-vboxmanage.sh"), override, bundle],
                env=dict(os.environ, PATH=path), capture_output=True)
            assert result.returncode == expected, (override, path, result.stderr)

if __name__ == "__main__":
    main()
