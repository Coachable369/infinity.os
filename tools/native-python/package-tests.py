#!/usr/bin/env python3
"""Deterministic real-wheel fixtures and behavioral native pip acceptance inputs."""
from pathlib import Path
import argparse
import base64
import csv
import hashlib
import io
import json
import os
import zipfile

ROOT = Path(__file__).resolve().parents[2]
BASE = "infinity_test_base-1.0.0-py3-none-any.whl"
APP = "infinity_test_app-1.0.0-py3-none-any.whl"
UPGRADE = "infinity_test_app-1.1.0-py3-none-any.whl"
TRAVERSAL = "infinity_test_escape-1.0.0-py3-none-any.whl"
PLATFORM = "infinity_test_platform-1.0.0-cp313-cp313-win_amd64.whl"
INVALID_NATIVE = "infinity_invalid_native-1.0.0-py3-none-any.whl"
NATIVE = {arch: "infinity_test_native-1.0.0-cp312-cp312-infinityos_" + arch + ".whl"
          for arch in ("aarch64", "x86_64")}
COMPATIBLE = {arch: "infinity_native_package-1.0.0-cp313-cp313-infinityos_" + arch + ".whl"
              for arch in ("aarch64", "x86_64")}
NATIVE_IMPORT = b'''import infinity_native_test
assert infinity_native_test.__file__ == "/packages/infinity_native_test.so"
assert infinity_native_test.add(19, 23) == 42
try:
    infinity_native_test.add(2**63 - 1, 1)
except OverflowError:
    pass
else:
    raise AssertionError("Native extension overflow did not raise")
'''

BASE_SOURCE = b'''# ------------------------=
# FUNC: add
# DESC: Returns the dependency's observable arithmetic result.
# ------------------=
def add(left, right):
    return left + right
'''
APP_SOURCE = b'''from infinity_test_app.nested.calculate import total
'''
CALCULATE_SOURCE = b'''from infinity_test_base import add

# ------------------------=
# FUNC: total
# DESC: Loads installed package data and uses the separately installed dependency.
# ------------------=
def total():
    path = __file__.rsplit("/", 1)[0] + "/numbers.txt"
    with open(path, "r", encoding="utf-8") as stream:
        left, right = (int(value) for value in stream.read().split())
    return add(left, right)
'''

IMPORT_SCRIPT = b'''import sys
import os
import hashlib
from importlib.metadata import distribution, version
from infinity_test_app import total
first = os.urandom(32)
second = os.urandom(32)
assert len(first) == len(second) == 32 and first != second
assert len(hashlib.sha256(first).digest()) == 32
try:
    with open("/packages/unauthorized-write", "wb") as stream:
        stream.write(b"forbidden")
except PermissionError:
    pass
else:
    raise AssertionError("An ordinary script received package write authority")
assert total() == 54
import tomli
assert tomli.version_marker == 123
try:
    import tomli._parser
except ModuleNotFoundError:
    pass
else:
    raise AssertionError("Bundled submodules contaminated an owned override")
assert version("infinity-test-app") == "1.0.0"
assert version("infinity-test-base") == "1.0.0"
metadata = distribution("infinity-test-app")
assert metadata.metadata["Name"] == "infinity-test-app"
assert metadata.requires == ["infinity-test-base==1.0.0"]
assert any(str(path).endswith("nested/numbers.txt") for path in metadata.files)
sys.exit(0)
'''

ABSENT_SCRIPT = b'''import sys
from importlib.util import find_spec
from importlib.metadata import PackageNotFoundError, version
assert find_spec("infinity_test_app") is None
import tomli
assert tomli.loads("value=42") == {"value": 42}
try:
    version("infinity-test-app")
except PackageNotFoundError:
    pass
else:
    raise AssertionError("Removed package metadata remains visible")
from infinity_test_base import add
assert add(19, 35) == 54
sys.exit(0)
'''

UPGRADED_SCRIPT = b'''import sys
from importlib.metadata import distributions, version
from infinity_test_app import total
assert total() == 55
assert version("infinity-test-app") == "1.1.0"
assert version("infinity-test-base") == "1.0.0"
versions = [item.version for item in distributions()
            if item.metadata["Name"] == "infinity-test-app"]
assert versions == ["1.1.0"]
with open("/packages/infinity_test_app/nested/numbers.txt", "rb") as stream:
    assert stream.read() == b"20 35\\n"
sys.exit(0)
'''

ISOLATED_SCRIPT = b'''import sys
import tomli
assert tomli.loads("value=42") == {"value": 42}
from importlib.util import find_spec
from importlib.metadata import PackageNotFoundError, version
for name in ("infinity_test_app", "infinity_test_base"):
    assert find_spec(name) is None
    try:
        version(name)
    except PackageNotFoundError:
        pass
    else:
        raise AssertionError("Foreign package metadata is visible")
sys.exit(0)
'''

METADATA_SCRIPT = b'''import contextlib
import errno
import io
import json
import mmap
import sys
import sysconfig
from xml.parsers import expat
from importlib.metadata import distribution
import pip
from pip._internal.cli.main import main
from pip._internal.models.wheel import Wheel
from pip._internal.utils.compatibility_tags import get_supported
assert pip.__version__ == "26.2.1"
with mmap.mmap(-1, 4096) as region:
    assert region[:] == bytes(4096)
    region[:4] = b"test"
    assert region[:4] == b"test"
assert region.closed
parser = expat.ParserCreate()
assert parser.Parse(b"<packages><package name='sample'/></packages>", True) == 1
assert parser.ErrorCode == 0
try:
    expat.ParserCreate().Parse(b"<packages>", True)
except expat.ExpatError as error:
    assert error.code > 0
else:
    raise AssertionError("Malformed XML was accepted")
tags = get_supported()
assert tags
assert Wheel("portable-1.0.0-py3-none-any.whl").supported(tags)
for architecture in ("aarch64", "x86_64"):
    native = Wheel("native-1.0.0-cp313-cp313-infinityos_" + architecture + ".whl")
    assert native.supported(tags) == (sysconfig.get_platform().replace("-", "_") == "infinityos_" + architecture)
    assert not Wheel("native-1.0.0-cp312-cp312-infinityos_" + architecture + ".whl").supported(tags)
capture = io.StringIO()
with contextlib.redirect_stdout(capture):
    result = main(["list", "--format=json", "--disable-pip-version-check"])
assert result == 0
records = {record["name"].replace("_", "-").lower(): record["version"]
           for record in json.loads(capture.getvalue())}
assert records["infinity-test-app"] == "1.0.0"
assert records["infinity-test-base"] == "1.0.0"
with contextlib.redirect_stdout(io.StringIO()):
    assert main(["show", "infinity-test-app"]) == 0
    assert main(["check"]) == 0
assert distribution("infinity-test-app").requires == ["infinity-test-base==1.0.0"]
sys.exit(0)
'''


# ------------------------=
# FUNC: wheel_bytes
# DESC: Creates a deterministic standards-shaped wheel with metadata and SHA-256 RECORD entries.
# ------------------=
def wheel_bytes(name, files, requirements=(), tag="py3-none-any", compression=zipfile.ZIP_STORED, version="1.0.0"):
    normalized = name.replace("-", "_")
    info = normalized + "-" + version + ".dist-info"
    entries = dict(files)
    metadata = ("Metadata-Version: 2.1\nName: " + name + "\nVersion: " + version + "\n"
                + "Summary: InfinityOS native pip behavioral fixture\n"
                + "".join("Requires-Dist: " + value + "\n" for value in requirements)
                + "\n")
    entries[info + "/METADATA"] = metadata.encode()
    entries[info + "/WHEEL"] = ("Wheel-Version: 1.0\nGenerator: InfinityOS acceptance\n"
                                "Root-Is-Purelib: " + ("true" if tag == "py3-none-any" else "false")
                                + "\nTag: " + tag + "\n").encode()
    records = io.StringIO(newline="")
    writer = csv.writer(records, lineterminator="\n")
    for path, contents in sorted(entries.items()):
        digest = base64.urlsafe_b64encode(hashlib.sha256(contents).digest()).rstrip(b"=").decode()
        writer.writerow((path, "sha256=" + digest, len(contents)))
    writer.writerow((info + "/RECORD", "", ""))
    entries[info + "/RECORD"] = records.getvalue().encode()
    output = io.BytesIO()
    with zipfile.ZipFile(output, "w", compression=compression) as archive:
        for path, contents in sorted(entries.items()):
            item = zipfile.ZipInfo(path, date_time=(2026, 1, 1, 0, 0, 0))
            item.compress_type = compression
            item.create_system = 3
            item.external_attr = 0o100644 << 16
            archive.writestr(item, contents)
    return output.getvalue()


# ------------------------=
# FUNC: fixtures
# DESC: Returns real dependency, nested-data, traversal and unsupported-platform wheel bytes without host package installation.
# ------------------=
def fixtures():
    result = {
        BASE: wheel_bytes("infinity-test-base", {"infinity_test_base/__init__.py": BASE_SOURCE}),
        APP: wheel_bytes("infinity-test-app", {
            "tomli/__init__.py": b"version_marker = 123\n",
            "infinity_test_app/__init__.py": APP_SOURCE,
            "infinity_test_app/nested/__init__.py": b"",
            "infinity_test_app/nested/calculate.py": CALCULATE_SOURCE,
            "infinity_test_app/nested/numbers.txt": b"19 35\n",
        }, requirements=("infinity-test-base==1.0.0",), compression=zipfile.ZIP_DEFLATED),
        UPGRADE: wheel_bytes("infinity-test-app", {
            "tomli/__init__.py": b"version_marker = 123\n",
            "infinity_test_app/__init__.py": APP_SOURCE,
            "infinity_test_app/nested/__init__.py": b"",
            "infinity_test_app/nested/calculate.py": CALCULATE_SOURCE,
            "infinity_test_app/nested/numbers.txt": b"20 35\n",
        }, requirements=("infinity-test-base==1.0.0",), compression=zipfile.ZIP_DEFLATED, version="1.1.0"),
        TRAVERSAL: wheel_bytes("infinity-test-escape", {"../../outside.py": b"escaped = True\n"}),
        PLATFORM: wheel_bytes("infinity-test-platform", {"infinity_test_platform.py": b"value = 1\n"},
                              tag="cp313-cp313-win_amd64"),
        INVALID_NATIVE: wheel_bytes("infinity-invalid-native", {"infinity_invalid_native.so": b"not an ELF image"}),
    }
    for arch, filename in NATIVE.items():
        result[filename] = wheel_bytes("infinity-test-native", {"infinity_test_native.py": b"value = 1\n"},
                                      tag="cp312-cp312-infinityos_" + arch)
    for arch, filename in COMPATIBLE.items():
        binary = (ROOT / "builds/toolchains" / arch / "infinity_native_test.so").read_bytes()
        result[filename] = wheel_bytes("infinity-native-package", {"infinity_native_test.so": binary},
                                      tag="cp313-cp313-infinityos_" + arch, compression=zipfile.ZIP_DEFLATED)
    return result


# ------------------------=
# FUNC: scenario
# DESC: Defines CLI and independent-interpreter state transitions without using rendered command output as a pass criterion.
# ------------------=
def scenario():
    return [
        {"name": "version", "argv": ["-m", "pip", "--version"], "exit": 0},
        {"name": "install", "argv": ["-m", "pip", "install", "--no-index", "--find-links=/work",
                                      "/work/" + APP, "--disable-pip-version-check", "--no-compile",
                                      "--progress-bar=off", "--only-binary=:all:"], "exit": 0},
        {"name": "import", "argv": ["/work/import.py"], "exit": 0},
        {"name": "metadata", "argv": ["/work/metadata.py"], "exit": 0},
        {"name": "reject-platform", "argv": ["-m", "pip", "install", "--no-index", "/work/" + PLATFORM],
         "exit_nonzero": True, "unchanged_store": True},
        {"name": "reject-traversal", "argv": ["-m", "pip", "install", "--no-index", "/work/" + TRAVERSAL],
         "exit_nonzero": True, "unchanged_store": True},
        {"name": "cold-boot-import", "argv": ["/work/import.py"], "exit": 0, "after_reboot": True},
        {"name": "foreign-user", "argv": ["/work/isolated.py"], "exit": 0, "different_user": True},
        {"name": "upgrade", "argv": ["-m", "pip", "install", "--upgrade", "--no-index", "--find-links=/work",
                                      "/work/" + UPGRADE, "--disable-pip-version-check", "--no-compile",
                                      "--progress-bar=off", "--only-binary=:all:"], "exit": 0},
        {"name": "upgraded-import", "argv": ["/work/upgraded.py"], "exit": 0, "archive_remount": True},
        {"name": "reject-native", "argv_by_arch": {
            arch: ["-m", "pip", "install", "--no-index", "/work/" + filename]
            for arch, filename in NATIVE.items()}, "exit_nonzero": True, "unchanged_store": True},
        {"name": "uninstall", "argv": ["-m", "pip", "uninstall", "-y", "infinity-test-app"], "exit": 0},
        {"name": "removed", "argv": ["/work/absent.py"], "exit": 0},
        {"name": "install-native", "argv_by_arch": {
            arch: ["-m", "pip", "install", "--no-index", "--no-compile", "/work/" + filename]
            for arch, filename in COMPATIBLE.items()}, "exit": 0},
        {"name": "import-native", "argv": ["/work/native-import.py"], "exit": 0, "archive_remount": True},
        {"name": "uninstall-native", "argv": ["-m", "pip", "uninstall", "-y", "infinity-native-package"], "exit": 0},
        {"name": "reject-invalid-native", "argv": ["-m", "pip", "install", "--no-index", "/work/" + INVALID_NATIVE],
         "transaction_exit": 65, "unchanged_store": True},
    ]


# ------------------------=
# FUNC: write_inputs
# DESC: Writes reusable native wheel and script fixtures only to an explicit repository build directory.
# ------------------=
def write_inputs(destination):
    destination = destination.resolve()
    if not any(destination.is_relative_to(ROOT / name) for name in ("build", "builds")):
        raise ValueError("Acceptance inputs must stay inside repository build/builds")
    destination.mkdir(parents=True, exist_ok=True)
    inputs = {**fixtures(), "import.py": IMPORT_SCRIPT, "metadata.py": METADATA_SCRIPT,
              "absent.py": ABSENT_SCRIPT, "isolated.py": ISOLATED_SCRIPT, "upgraded.py": UPGRADED_SCRIPT,
              "native-import.py": NATIVE_IMPORT}
    assert len(inputs) <= 16 and sum(map(len, inputs.values())) < 65536
    for name, data in inputs.items():
        (destination / name).write_bytes(data)
    manifest = {"version": 1, "installed_acceptance": False, "fixture_only": True,
                "inputs": {name: {"bytes": len(data), "sha256": hashlib.sha256(data).hexdigest(),
                                  "guest_path": "/work/" + name}
                           for name, data in inputs.items()}, "stages": scenario()}
    (destination / "fixtures.json").write_text(json.dumps(manifest, indent=2) + "\n")
    return manifest


# ------------------------=
# FUNC: verify_fixtures
# DESC: Verifies deterministic ZIP bytes, RECORD digests, dependency metadata and native assertion-script syntax without claiming native execution.
# ------------------=
def verify_fixtures():
    first = fixtures()
    assert first == fixtures()
    for name, data in first.items():
        with zipfile.ZipFile(io.BytesIO(data)) as archive:
            paths = archive.namelist()
            assert len(paths) == len(set(paths)) and archive.testzip() is None
            record = next(path for path in paths if path.endswith(".dist-info/RECORD"))
            for path, digest, size in csv.reader(io.StringIO(archive.read(record).decode())):
                content = archive.read(path)
                if path == record:
                    assert digest == size == ""
                else:
                    expected = base64.urlsafe_b64encode(hashlib.sha256(content).digest()).rstrip(b"=").decode()
                    assert digest == "sha256=" + expected and int(size) == len(content)
            if name in (APP, UPGRADE):
                assert all(item.compress_type == zipfile.ZIP_DEFLATED for item in archive.infolist())
                assert any(item.compress_size < item.file_size for item in archive.infolist())
                assert archive.read("infinity_test_app/nested/numbers.txt") == (b"19 35\n" if name == APP else b"20 35\n")
                assert "infinity_test_app/nested/calculate.py" in paths
            if name == TRAVERSAL:
                assert "../../outside.py" in paths
    for source in (IMPORT_SCRIPT, ABSENT_SCRIPT, ISOLATED_SCRIPT, METADATA_SCRIPT, UPGRADED_SCRIPT, NATIVE_IMPORT):
        compile(source, "<native-package-acceptance>", "exec")
    # Execute fixture arithmetic directly, not a host-installed substitute package.
    dependency = {}
    exec(compile(BASE_SOURCE, "<dependency>", "exec"), dependency)
    assert dependency["add"](19, 35) == 54
    return {"fixture_structure_verified": True, "deterministic": True,
            "native_execution_verified": False, "wheel_count": len(first)}


# ------------------------=
# FUNC: main
# DESC: Generates auditable offline package inputs under build-kit authority and verifies their actual binary contents.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Use ./build-kit run python3 tools/native-python/package-tests.py")
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=ROOT / "build/native-python/package-fixtures")
    args = parser.parse_args()
    proof = verify_fixtures()
    manifest = write_inputs(args.output)
    print(json.dumps({**proof, "directory": str(args.output.resolve()),
                      "files": len(manifest["inputs"])}))


if __name__ == "__main__":
    main()
