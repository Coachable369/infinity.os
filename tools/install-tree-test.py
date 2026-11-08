#!/usr/bin/env python3
"""Behavioral checks for recursive guest-root staging discovery."""
import importlib.util
from pathlib import Path
import tempfile

spec = importlib.util.spec_from_file_location("package_sdk", Path(__file__).with_name("package-sdk.py"))
package = importlib.util.module_from_spec(spec)
spec.loader.exec_module(package)

# ------------------------=
# FUNC: main
# DESC: Exercises nested binary/text files, empty directories, edits, removals and invalid staging inputs.
# ------------------=
def main():
    scratch = Path(__file__).resolve().parents[1] / "build/tmp"
    scratch.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(dir=scratch) as folder:
        root = Path(folder)
        stage = root / "runtime/install"
        (stage / "System/Tools/empty").mkdir(parents=True)
        (stage / "Applications/Demo").mkdir(parents=True)
        target = stage / "System/Tools/example.py"
        target.write_bytes(b"print(42)\n")
        (stage / "Applications/Demo/data.bin").write_bytes(bytes(range(256)))
        rows = {path: data for path, source, data in package.install_tree(root)}
        assert rows["/System/Tools/empty"] is None
        assert rows["/System/Tools/example.py"] == b"print(42)\n"
        assert rows["/Applications/Demo/data.bin"] == bytes(range(256))
        assert len(rows) == 7
        target.write_bytes(b"print(43)\n")
        assert dict((p, d) for p, _, d in package.install_tree(root))["/System/Tools/example.py"] == b"print(43)\n"
        target.unlink()
        assert "/System/Tools/example.py" not in {p for p, _, _ in package.install_tree(root)}
        link = stage / "linked"
        link.symlink_to(root)
        try:
            package.install_tree(root)
        except ValueError:
            pass
        else:
            raise AssertionError("Symlink accepted")
        link.unlink()
        (stage / ("x" * 96)).write_bytes(b"x")
        try:
            package.install_tree(root)
        except ValueError:
            pass
        else:
            raise AssertionError("Oversize guest path accepted")
    print("PASS: recursive installed tree discovery")

if __name__ == "__main__":
    main()
