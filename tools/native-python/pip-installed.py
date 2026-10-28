#!/usr/bin/env python3
"""Actual pip lifecycle on a fresh installed OS, with detached-media cold-boot proof."""
from pathlib import Path
import argparse
import hashlib
import importlib.util
import json
import os
import shutil
import struct
import subprocess
import time

ROOT = Path(__file__).resolve().parents[2]


# ------------------------=
# FUNC: module
# DESC: Loads existing native guest and fixture helpers without duplicating a VM framework.
# ------------------=
def module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


# ------------------------=
# FUNC: validate_media_layout
# DESC: Checks both media-owned ELF load maps against the selected emulator before creating a VM or changing a disk.
# ------------------=
def validate_media_layout(artifacts, architecture):
    layout = module("native_package_kernel_layout", ROOT / "tools/kernel_layout.py")
    platform = "qemu" if architecture == "aarch64" else "virtualbox"
    expected_machine = {"aarch64": 183, "x86_64": 62}[architecture]
    ranges = {}
    for name in ("kernel.elf", "installed-kernel.elf"):
        with (artifacts / name).open("rb") as stream:
            header = stream.read(65536)
        try:
            if len(header) < 20 or struct.unpack_from("<H", header, 18)[0] != expected_machine:
                raise ValueError("Kernel architecture does not match the selected guest")
            ranges[name] = layout.validate(header, platform=platform)
        except ValueError as error:
            if architecture == "aarch64":
                raise ValueError(f"{name}: {error}. This QEMU runner requires ARM QEMU-linked media; "
                                 "use --iso build/test-media/InfinityOS-aarch64-qemu.bootmedia "
                                 "(or InfinityOS-aarch64-qemu-models.bootmedia). "
                                 "The published ARM ISO is for VirtualBox, not QEMU virt.") from error
            raise ValueError(f"{name}: {error}") from error
    return {"emulator": "qemu-virt" if architecture == "aarch64" else "qemu-pc", "load_ranges": ranges}


# ------------------------=
# FUNC: execute
# DESC: Submits real console input and verifies typed exit and memory-reclamation receipts rather than output prose.
# ------------------=
def execute(guest, python, command, success=True, timeout=150, package_rejected=False):
    before = python.snapshot(guest)
    assert before is not None
    guest.command(command)
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        value = python.snapshot(guest)
        if value and value[2] > before[2] and value[3] != 1:
            assert value[3] == (5 if package_rejected else 2), (command, value)
            if package_rejected:
                assert not success and value[4] == 65, (command, value)
            assert (value[4] == 0) == success, (command, value)
            if not success:
                assert 0 < value[4] < 256, (command, value)
            assert value[7] == value[8], (command, value)
            return {"command": command, "receipt": value, "successful_exit": success,
                    "memory_reclaimed": True}
        time.sleep(.1)
    raise AssertionError({"command_not_completed": command, "snapshot": python.snapshot(guest)})


# ------------------------=
# FUNC: archive_entries
# DESC: Parses the actual versioned committed package archive and rejects truncated, duplicate or out-of-root records.
# ------------------=
def archive_entries(data):
    assert len(data) >= 16 and data[:8] == b"IPKG0001" and data[12:16] == bytes(4)
    count, = struct.unpack_from("<I", data, 8)
    assert count <= 2048
    cursor = 16
    entries = {}
    for _ in range(count):
        length, flags, size = struct.unpack_from("<HHI", data, cursor)
        cursor += 8
        assert 10 < length <= 255 and flags <= 3 and (not flags & 1 or size == 0)
        end = cursor + length + size
        assert end <= len(data)
        path = data[cursor:cursor + length].decode("utf-8")
        assert path.startswith("/packages/") and ".." not in path.split("/") and path not in entries
        entries[path] = {"directory": bool(flags & 1), "bytes": data[cursor + length:end]}
        cursor = end
    assert cursor == len(data)
    return entries


# ------------------------=
# FUNC: inspect
# DESC: Stops the VM before read-only native ObjectStore inspection and captures exact package bytes across remount.
# ------------------=
def inspect(guest, helper, destination):
    guest.stop()
    destination.mkdir(parents=True)
    result = subprocess.run([str(helper), "inspect", str(guest.disk), str(destination)],
                            check=True, capture_output=True, text=True)
    receipt = json.loads(result.stdout)
    archive = (destination / "packages.bin").read_bytes()
    assert receipt["read_only"] and receipt["foreign_owner_empty"]
    assert receipt["archive_bytes"] == len(archive)
    entries = archive_entries(archive)
    receipt["archive_sha256"] = hashlib.sha256(archive).hexdigest()
    receipt["entries"] = len(entries)
    (destination / "receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")
    return receipt, entries


# ------------------------=
# FUNC: resume
# DESC: Boots the unchanged installed disk without installer media and authenticates through the real password field.
# ------------------=
def resume(guest):
    guest.boot(False)
    guest.authenticate()
    assert not guest.installer and guest.state()[3] == 1
    guest.launch("command", 5)


# ------------------------=
# FUNC: seed
# DESC: Swaps only four test-input objects on the stopped disk while preserving the committed package archive and namespace headroom.
# ------------------=
def seed(guest, helper, fixtures, phase):
    guest.stop()
    result = subprocess.run([str(helper), "seed", str(guest.disk), str(fixtures), phase, guest.arch],
                            check=True, capture_output=True, text=True)
    receipt = json.loads(result.stdout)
    expected = 3 if phase in ("bundled", "bundled-import") else 4
    assert receipt["seeded_inputs"] == expected and not receipt["package_state_changed"]
    assert receipt["reused_phase"] == phase
    return receipt


# ------------------------=
# FUNC: online_inputs
# DESC: Writes only assertions for public packages; no wheel or installed package bytes are supplied by the host.
# ------------------=
def online_inputs(directory):
    inputs = {
        "online.py": b'''import six
from dateutil.parser import isoparse
from packaging.version import Version
assert six.text_type(123) == "123"
assert isoparse("2026-10-19T12:34:56Z").hour == 12
assert Version("2.0") > Version("1.9")
''',
        "online-metadata.py": b'''from importlib.metadata import version, distribution
assert version("six") == "1.17.0"
assert version("python-dateutil") == "2.9.0.post0"
assert version("packaging") == "25.0"
assert any(item.startswith("six") for item in distribution("python-dateutil").requires)
''',
        "online-absent.py": b'''from importlib.metadata import version
from _infinity_inventory import INVENTORY
for name in ("six", "python-dateutil", "packaging"):
    assert version(name) == INVENTORY["bundled_distributions"][name]
import six, dateutil, packaging
assert all(module.__file__.startswith("/stdlib/") for module in (six, dateutil, packaging))
''',
        "online-denied.py": b'''import _infinity_https
try:
    _infinity_https.begin(b"https://pypi.org/simple/six/", b"")
except PermissionError:
    pass
else:
    raise AssertionError("Ordinary script acquired ungranted package network authority")
''',
        "stdlib-tls.py": b'''import socket, ssl, http.client, urllib.request, json
context = ssl.create_default_context()
assert context.verify_mode == ssl.CERT_REQUIRED and context.check_hostname
raw = socket.create_connection(("pypi.org", 443), timeout=30)
with context.wrap_socket(raw, server_hostname="pypi.org") as stream:
    assert stream.version() == "TLSv1.3"
    stream.sendall(b"GET /pypi/six/json HTTP/1.1\\r\\nHost: pypi.org\\r\\nConnection: close\\r\\n\\r\\n")
    response = http.client.HTTPResponse(stream)
    response.begin()
    assert response.status == 200
    body = response.read()
    assert len(body) > 16384 and json.loads(body)["info"]["name"] == "six"
with urllib.request.urlopen("https://pypi.org/pypi/packaging/json", timeout=30) as response:
    assert response.status == 200
    body = response.read()
    assert len(body) > 16384 and json.loads(body)["info"]["name"] == "packaging"
''',
        "stdlib-tls-denied.py": b'''import socket, ssl, os
from importlib.resources import files, as_file
payload = bytes(range(256)) * 100
descriptor = os.open("/tmp/write-boundary.bin", os.O_CREAT | os.O_WRONLY | os.O_TRUNC, 0o600)
try:
    assert os.write(descriptor, payload) == len(payload)
finally:
    os.close(descriptor)
with open("/tmp/write-boundary.bin", "rb") as stream:
    assert stream.read() == payload
resource = files("pip._vendor.pyproject_hooks._in_process").joinpath("_in_process.py")
expected = resource.read_bytes()
assert len(expected) > 4096
with as_file(resource) as path:
    actual = path.read_bytes()
    assert actual == expected
    compile(actual, str(path), "exec")
context = ssl.create_default_context()
assert context.verify_mode == ssl.CERT_REQUIRED
try:
    context.check_hostname = False
except ValueError:
    pass
assert context.check_hostname
try:
    context.verify_mode = ssl.CERT_NONE
except ValueError:
    pass
else:
    raise AssertionError("TLS validation was disabled")
try:
    socket.create_connection(("pypi.org", 443), timeout=1)
except PermissionError:
    pass
else:
    raise AssertionError("Unapproved script acquired native network access")
''',
        "stdlib-tls-hostname.py": b'''import socket, ssl
context = ssl.create_default_context()
raw = socket.create_connection(("pypi.org", 443), timeout=30)
try:
    context.wrap_socket(raw, server_hostname="invalid.infinityos.example")
except ssl.SSLCertVerificationError:
    pass
else:
    raise AssertionError("TLS accepted a certificate for the wrong hostname")
finally:
    raw.close()
''',
        "stdlib-sqlite.py": b'''import sqlite3, os
path = "/tmp/native.db"
first = sqlite3.connect(path, timeout=0)
first.execute("CREATE TABLE sample (id INTEGER PRIMARY KEY, name TEXT NOT NULL)")
first.commit()
second = sqlite3.connect(path, timeout=0)
first.execute("BEGIN IMMEDIATE")
first.execute("INSERT INTO sample VALUES (1, 'rollback')")
try:
    second.execute("INSERT INTO sample VALUES (2, 'blocked')")
except sqlite3.OperationalError as error:
    assert error.sqlite_errorcode == sqlite3.SQLITE_BUSY
else:
    raise AssertionError("Concurrent SQLite writer bypassed the native lock")
first.rollback()
assert first.execute("SELECT count(*) FROM sample").fetchone() == (0,)
second.execute("INSERT INTO sample VALUES (3, 'committed')")
second.commit()
assert first.execute("SELECT id, name FROM sample").fetchall() == [(3, 'committed')]
assert first.execute("PRAGMA integrity_check").fetchone() == ("ok",)
first.close()
second.close()
with sqlite3.connect(path) as reopened:
    assert reopened.execute("SELECT id FROM sample").fetchall() == [(3,)]
try:
    forbidden = sqlite3.connect("/system/forbidden.db")
    forbidden.execute("CREATE TABLE forbidden (id INTEGER)")
except sqlite3.OperationalError:
    pass
else:
    raise AssertionError("SQLite acquired ungranted System write authority")
os.chdir("/tmp")
with sqlite3.connect("relative.db") as relative:
    relative.execute("CREATE TABLE data (value INTEGER)")
    relative.execute("INSERT INTO data VALUES (7)")
with sqlite3.connect("/tmp/./relative.db") as canonical:
    assert canonical.execute("SELECT value FROM data").fetchall() == [(7,)]
try:
    sqlite3.connect("/tmp/missing-parent/denied.db")
except sqlite3.OperationalError as error:
    assert error.sqlite_errorcode == sqlite3.SQLITE_CANTOPEN
else:
    raise AssertionError("SQLite created an unapproved missing parent directory")
with sqlite3.connect("file:/tmp/native.db?mode=ro", uri=True) as readonly:
    assert readonly.execute("SELECT id FROM sample").fetchall() == [(3,)]
    try:
        readonly.execute("INSERT INTO sample VALUES (4, 'denied')")
    except sqlite3.OperationalError as error:
        assert error.sqlite_errorcode == sqlite3.SQLITE_READONLY
    else:
        raise AssertionError("SQLite wrote through a readonly connection")
''',
    }
    directory.mkdir(parents=True, exist_ok=True)
    for name, source in inputs.items():
        compile(source, name, "exec")
        (directory / name).write_bytes(source)
    return {name: hashlib.sha256(source).hexdigest() for name, source in inputs.items()}


# ------------------------=
# FUNC: online_lifecycle
# DESC: Installs three pinned public distributions through native DNS/TCP/TLS, imports their behavior, cold boots, and verifies rollback and uninstall.
# ------------------=
def online_lifecycle(guest, python, helper, fixture_dir, destination, proof, *, sqlite=True):
    proof["online_input_sha256"] = online_inputs(fixture_dir)
    proof["fixture_phases"] = [seed(guest, helper, fixture_dir, "tls")]
    resume(guest)
    # Exercise the real Network Settings path before any package operation.
    python.native.desktop.network.configure_nat(guest)
    guest.launch("command", 5)
    documents = "/home/default/documents/"
    for name in (("stdlib-sqlite.py",) if sqlite else ()) + ("stdlib-tls-denied.py",):
        proof["stages"].append(execute(guest, python, "python " + documents + name))
    for name in ("stdlib-tls.py", "stdlib-tls-hostname.py"):
        proof["stages"].append(execute(guest, python, "python --allow-network " + documents + name, timeout=200))
    proof["native_stdlib_tls"] = {"socket_handshake": True, "multi_record_http": True,
        "urllib_https": True, "hostname_rejection": True, "explicit_invocation_grant": True}
    proof["fixture_phases"].append(seed(guest, helper, fixture_dir, "online"))
    resume(guest)
    for package in ("six==1.17.0", "python-dateutil==2.9.0.post0", "packaging==25.0"):
        proof["stages"].append(execute(guest, python, "pip install --ignore-installed " + package, timeout=650))
    for name in ("online.py", "online-metadata.py", "online-denied.py"):
        proof["stages"].append(execute(guest, python, "python " + documents + name))
    before, entries = inspect(guest, helper, destination / "online-installed")
    assert "/packages/six.py" in entries
    assert "/packages/dateutil/parser/_parser.py" in entries
    assert "/packages/packaging/version.py" in entries
    proof["installed_archive"] = before
    resume(guest)
    proof["stages"].append(execute(guest, python, "python " + documents + "online.py"))
    proof["cold_boot_import"] = True
    proof["stages"].append(execute(guest, python, "pip install infinityos-nonexistent-acceptance-987654321", success=False, timeout=650))
    after, _ = inspect(guest, helper, destination / "online-rejected")
    assert before["archive_sha256"] == after["archive_sha256"] and before["version"] == after["version"]
    proof["failed_install_preserved_state"] = True
    resume(guest)
    proof["stages"].append(execute(guest, python, "pip uninstall -y six python-dateutil packaging"))
    proof["stages"].append(execute(guest, python, "python " + documents + "online-absent.py"))
    removed, _ = inspect(guest, helper, destination / "online-removed")
    assert removed["archive_sha256"] != before["archive_sha256"] and removed["version"] > before["version"]
    proof["removed_archive"] = removed
    proof["online_native_install_acceptance"] = True
    proof["host_supplied_package_bytes"] = False


# ------------------------=
# FUNC: source_lifecycle
# DESC: Builds a source-only distribution through real native PEP 517 children, verifies its result after reboot and proves failed backend rollback.
# ------------------=
def source_lifecycle(guest, python, helper, fixture_dir, destination, proof):
    sources = module("native_source_fixtures", ROOT / "tools/native-python/sdist-fixtures.py")
    inputs = sources.fixtures()
    for name, value in inputs.items():
        (fixture_dir / name).write_bytes(value)
    proof["source_input_sha256"] = {name: hashlib.sha256(value).hexdigest() for name, value in inputs.items()}
    proof.setdefault("fixture_phases", []).append(seed(guest, helper, fixture_dir, "source"))
    resume(guest)
    documents = "/home/default/documents/"
    proof["stages"].append(execute(guest, python, "pip install " + documents + sources.NAME, timeout=650))
    proof["stages"].append(execute(guest, python, "python " + documents + "source-import.py"))
    before, entries = inspect(guest, helper, destination / "source-installed")
    assert "/packages/infinity_source_probe.py" in entries
    resume(guest)
    proof["stages"].append(execute(guest, python, "python " + documents + "source-import.py"))
    proof["stages"].append(execute(guest, python, "pip install " + documents + sources.FAILURE,
                                   success=False, timeout=650))
    after, _ = inspect(guest, helper, destination / "source-rejected")
    assert before["archive_sha256"] == after["archive_sha256"] and before["version"] == after["version"]
    resume(guest)
    proof["stages"].append(execute(guest, python, "pip uninstall -y infinity-source-probe"))
    proof["stages"].append(execute(guest, python, "python " + documents + "source-absent.py"))
    removed, entries = inspect(guest, helper, destination / "source-removed")
    assert "/packages/infinity_source_probe.py" not in entries
    assert removed["version"] > before["version"]
    proof["native_pep517_source_build"] = {"isolated_child": True, "cold_boot_import": True,
        "failed_backend_rollback": True, "uninstall": True, "host_supplied_wheel": False}
    resume(guest)
    python.native.desktop.network.configure_nat(guest)
    guest.launch("command", 5)
    proof["stages"].append(execute(guest, python, "pip install --ignore-installed " + sources.PUBLIC_SOURCE + " six==1.17.0", timeout=650))
    proof["stages"].append(execute(guest, python, "python " + documents + "source-absent.py public"))
    public, entries = inspect(guest, helper, destination / "public-source-installed")
    assert "/packages/packaging/version.py" in entries and "/packages/six.py" in entries
    assert not any(path.startswith("/packages/flit_core") for path in entries)
    resume(guest)
    proof["stages"].append(execute(guest, python, "python " + documents + "source-absent.py public"))
    proof["native_public_source_build"] = {"source_url": sources.PUBLIC_SOURCE,
        "declared_backend": "flit_core>=3.3", "isolated_build_dependency": True,
        "subsequent_resolution": True, "cold_boot_import": True, "archive": public}


# ------------------------=
# FUNC: library_lifecycle
# DESC: Exercises real public HTTP, HTML parsing and templating packages after native installation, without supplying any host package bytes.
# ------------------=
def library_lifecycle(guest, python, helper, fixture_dir, destination, proof):
    inputs = {
        "libraries-http.py": b'''import requests
with requests.Session() as session:
    session.trust_env = False
    with session.get("https://pypi.org/pypi/six/json", timeout=30) as response:
        assert response.status_code == 200
        assert len(response.content) > 16384
        assert response.json()["info"]["name"] == "six"
''',
        "libraries-render.py": b'''from jinja2 import Environment
from markupsafe import escape
from bs4 import BeautifulSoup
import soupsieve
environment = Environment(autoescape=True)
document = environment.from_string("<ul>{% for item in values %}<li>{{ item }}</li>{% endfor %}</ul>").render(values=["<safe>", "second"])
assert "&lt;safe&gt;" in document and str(escape("<safe>")) == "&lt;safe&gt;"
tree = BeautifulSoup(document, "html.parser")
assert [item.get_text() for item in soupsieve.select("ul > li", tree)] == ["<safe>", "second"]
assert tree.select_one("li:nth-of-type(2)").get_text() == "second"
''',
        "libraries-metadata.py": b'''from importlib.metadata import version
assert version("requests") == "2.32.5"
assert version("certifi") == "2026.6.17"
assert version("Jinja2") == "3.1.6"
assert version("MarkupSafe") == "2.1.5"
assert version("beautifulsoup4") == "4.13.4"
assert version("soupsieve") == "2.8"
''',
        "libraries-absent.py": b'''from importlib.util import find_spec
from importlib.metadata import version
from _infinity_inventory import INVENTORY
for name in ("requests", "certifi"):
    assert find_spec(name) is None
for name in ("jinja2", "markupsafe", "beautifulsoup4", "soupsieve"):
    assert version(name) == INVENTORY["bundled_distributions"][name]
''',
    }
    for name, value in inputs.items():
        compile(value, name, "exec")
        (fixture_dir / name).write_bytes(value)
    proof["library_input_sha256"] = {name: hashlib.sha256(value).hexdigest() for name, value in inputs.items()}
    proof.setdefault("fixture_phases", []).append(seed(guest, helper, fixture_dir, "libraries"))
    resume(guest)
    python.native.desktop.network.configure_nat(guest)
    guest.launch("command", 5)
    documents = "/home/default/documents/"
    for command in (
        "pip install requests==2.32.5 certifi==2026.6.17",
        "pip install --ignore-installed Jinja2==3.1.6 MarkupSafe==2.1.5",
        "pip install --ignore-installed beautifulsoup4==4.13.4 soupsieve==2.8",
    ):
        proof["stages"].append(execute(guest, python, command, timeout=650))
    for name in ("libraries-render.py", "libraries-metadata.py"):
        proof["stages"].append(execute(guest, python, "python " + documents + name))
    proof["stages"].append(execute(guest, python, "python --allow-network " + documents + "libraries-http.py", timeout=200))
    before, entries = inspect(guest, helper, destination / "libraries-installed")
    for path in ("/packages/requests/api.py", "/packages/jinja2/environment.py", "/packages/bs4/__init__.py"):
        assert path in entries
    # The upstream MarkupSafe build may legitimately omit its optional speedup;
    # record what was actually installed instead of claiming a native extension.
    native_markup = any(path.startswith("/packages/markupsafe/_speedups") and path.endswith(".so") for path in entries)
    resume(guest)
    proof["stages"].append(execute(guest, python, "python " + documents + "libraries-render.py"))
    proof["stages"].append(execute(guest, python, "python --allow-network " + documents + "libraries-http.py", timeout=200))
    proof["stages"].append(execute(guest, python, "pip uninstall -y requests certifi Jinja2 MarkupSafe beautifulsoup4 soupsieve"))
    proof["stages"].append(execute(guest, python, "python " + documents + "libraries-absent.py"))
    removed, _ = inspect(guest, helper, destination / "libraries-removed")
    assert removed["version"] > before["version"] and removed["archive_sha256"] != before["archive_sha256"]
    proof["native_library_matrix"] = {"http": "requests", "templating": "Jinja2",
        "parsing": ["beautifulsoup4", "soupsieve"], "markupsafe_native_speedup": native_markup,
        "cold_boot_behavior": True, "uninstall": True, "host_supplied_package_bytes": False}


# ------------------------=
# FUNC: main
# DESC: Installs real media and proves native pip dependency resolution, package data, cold persistence, failed-install atomicity and removal.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Use ./build-kit run python3 tools/native-python/pip-installed.py --arch ARCH --iso PATH")
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--arch", choices=("aarch64", "x86_64"), required=True)
    parser.add_argument("--iso", type=Path, required=True)
    parser.add_argument("--online", action="store_true", help="Exercise real native HTTPS PyPI installs instead of offline fixture wheels")
    parser.add_argument("--source", action="store_true", help="Also build source-only packages using native PEP 517 children")
    parser.add_argument("--libraries", action="store_true", help="Also exercise actual HTTP, templating and HTML parsing packages")
    parser.add_argument("--bundled-only", action="store_true", help="Verify the offline baseline on fresh installed and cold-booted disks")
    parser.add_argument("--resume-work", type=Path, help="Continue independent checks on an unchanged stopped disposable installed guest")
    parser.add_argument("--skip-sqlite", action="store_true", help="Preserve a previous independent SQLite failure while continuing network/package checks")
    args = parser.parse_args()
    iso = args.iso.resolve()
    assert iso.is_relative_to(ROOT) and iso.is_file()
    python = module("native_python_installed", ROOT / "tools/native-python/installed.py")
    fixtures = module("native_package_fixtures", ROOT / "tools/native-python/package-tests.py")
    native = python.native
    if args.skip_sqlite and (args.resume_work is None or not args.online):
        parser.error("--skip-sqlite requires --resume-work and --online; it never produces overall installed acceptance")
    work = (args.resume_work.resolve() if args.resume_work else
            ROOT / "build" / f"browser-installed-pip-{args.arch}-{time.time_ns()}")
    assert work.is_relative_to(ROOT / "build")
    artifacts = work / "artifacts"
    if args.resume_work:
        assert (work / "node-1/installed.raw").is_file()
        for path in (iso, artifacts / "installer.iso"):
            with path.open("rb") as stream:
                digest = hashlib.file_digest(stream, "sha256").hexdigest()
            if path == iso:
                expected_digest = digest
            else:
                assert digest == expected_digest, "Resuming requires the exact original immutable media"
        kernels = {"unchanged_media_sha256": expected_digest, "resumed_work": str(work)}
    else:
        artifacts.mkdir(parents=True)
        kernels = native.desktop.extract_media_kernels(iso, artifacts, args.arch)
    try:
        layout = validate_media_layout(artifacts, args.arch)
    except ValueError as error:
        parser.error(str(error))
    if not args.resume_work:
        shutil.copyfile(iso, artifacts / "installer.iso")
    fixture_dir = work / "package-fixtures"
    fixtures.verify_fixtures()
    fixture_manifest = fixtures.write_inputs(fixture_dir)
    helper = ROOT / "build/tools/python-package-disk"
    helper.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run(["rustc", "--edition=2021", "-A", "warnings", "tools/native-python/package-disk.rs",
                    "-o", str(helper)], check=True, cwd=ROOT)
    guest = native.NativeGuest(work, 1, f"/opt/homebrew/share/qemu/edk2-{args.arch}-code.fd",
                               reuse=bool(args.resume_work), width=1024, height=768, memory_mb=12288)
    guest.arch = args.arch
    guest.acceleration = "hvf" if args.arch == "aarch64" else "tcg"
    guest.patched = True
    guest.install_timeout_seconds = 1800 if args.arch == "x86_64" else 900
    # Full-bundle x86 firmware is software-emulated on ARM hosts. The observed
    # 300-second failure was still executing firmware, before kernel startup.
    guest.boot_timeout_seconds = 600 if args.arch == "x86_64" else 120
    destination = ROOT / "builds/validation/native-python" / args.arch / ("pip-installed-" + str(time.time_ns()))
    destination.mkdir(parents=True)
    proof = {"architecture": args.arch, "installed_acceptance": False, "media_kernels": kernels,
             "media_layout": layout, "fixture_manifest": fixture_manifest, "stages": [],
             "authenticated_second_user_test": False}
    if args.skip_sqlite:
        proof["unresolved_independent_gates"] = ["sqlite-private-file"]
    documents = "/home/default/documents/"
    try:
        if not args.resume_work:
            guest.install()
            guest.stop()
        subprocess.run(["python3", str(ROOT / "tools/update-installed-clone.py"), str(guest.disk),
                        str(artifacts / "installed-kernel.elf"), "--verify-only"], check=True)
        if not args.resume_work:
            guest.onboard()
            guest.stop()
        if args.bundled_only:
            source = (ROOT / "tools/native-python/bundled-tests.py").read_bytes()
            (fixture_dir / "bundled-tests.py").write_bytes(source)
            proof["fixture_phases"] = [seed(guest, helper, fixture_dir, "bundled")]
            proof["bundled_source_sha256"] = hashlib.sha256(source).hexdigest()
            for iteration in range(2):
                resume(guest)
                proof["stages"].append(execute(guest, python,
                    "python " + documents + "bundled-tests.py", timeout=300))
                if iteration == 0:
                    proof["stages"].append(execute(guest, python,
                        "pip install --ignore-installed " + documents + fixtures.APP, timeout=300))
                    guest.stop()
                    proof["fixture_phases"].append(seed(guest, helper, fixture_dir, "bundled-import"))
                    resume(guest)
                    proof["stages"].append(execute(guest, python, "python " + documents + "import.py"))
                    proof["stages"].append(execute(guest, python,
                        "pip uninstall -y infinity-test-app infinity-test-base"))
                    proof["stages"].append(execute(guest, python,
                        "python " + documents + "bundled-tests.py", timeout=300))
                guest.stop()
            proof.update(installed_acceptance=True, bundled_offline=True,
                         cold_boot_import=True, installer_media_detached=True,
                         user_override_install_and_removal=True)
            with iso.open("rb") as stream:
                proof["iso_sha256"] = hashlib.file_digest(stream, "sha256").hexdigest()
            return
        if args.online:
            online_lifecycle(guest, python, helper, fixture_dir, destination, proof, sqlite=not args.skip_sqlite)
            if args.source:
                source_lifecycle(guest, python, helper, fixture_dir, destination, proof)
            if args.libraries:
                library_lifecycle(guest, python, helper, fixture_dir, destination, proof)
            proof["installed_acceptance"] = not proof.get("unresolved_independent_gates")
            with iso.open("rb") as stream:
                proof["iso_sha256"] = hashlib.file_digest(stream, "sha256").hexdigest()
            return
        proof["fixture_phases"] = [seed(guest, helper, fixture_dir, "install")]
        resume(guest)
        proof["stages"].append(execute(guest, python, "python " + documents + "hello.py"))
        proof["stages"].append(execute(guest, python, "python " + documents + "native_demo.py"))
        proof["stages"].append(execute(guest, python, "python -m pip --version"))
        proof["stages"].append(execute(guest, python, "pip install " + documents + fixtures.APP))
        proof["stages"].append(execute(guest, python, "python " + documents + "import.py"))
        proof["stages"].append(execute(guest, python, "python " + documents + "metadata.py"))
        before, entries = inspect(guest, helper, destination / "installed")
        assert entries["/packages/infinity_test_app/nested/numbers.txt"]["bytes"] == b"19 35\n"
        assert entries["/packages/infinity_test_base/__init__.py"]["bytes"] == fixtures.BASE_SOURCE
        proof["installed_archive"] = before
        proof["fixture_phases"].append(seed(guest, helper, fixture_dir, "reject"))
        resume(guest)
        proof["stages"].append(execute(guest, python, "python " + documents + "import.py"))
        proof["cold_boot_import"] = True
        for wheel in (fixtures.PLATFORM, fixtures.TRAVERSAL, fixtures.NATIVE[args.arch]):
            proof["stages"].append(execute(guest, python, "pip install " + documents + wheel, success=False))
        after, _ = inspect(guest, helper, destination / "rejected")
        assert before["archive_sha256"] == after["archive_sha256"] and before["version"] == after["version"]
        proof["failed_install_preserved_state"] = True
        proof["fixture_phases"].append(seed(guest, helper, fixture_dir, "upgrade"))
        resume(guest)
        proof["stages"].append(execute(guest, python, "pip install --upgrade " + documents + fixtures.UPGRADE))
        proof["stages"].append(execute(guest, python, "python " + documents + "upgraded.py"))
        upgraded, entries = inspect(guest, helper, destination / "upgraded")
        assert entries["/packages/infinity_test_app/nested/numbers.txt"]["bytes"] == b"20 35\n"
        assert not any(path.startswith("/packages/infinity_test_app-1.0.0.dist-info/") for path in entries)
        assert upgraded["version"] > before["version"] and upgraded["archive_sha256"] != before["archive_sha256"]
        proof["upgraded_archive"] = upgraded
        resume(guest)
        proof["stages"].append(execute(guest, python, "python " + documents + "upgraded.py"))
        proof["cold_boot_upgrade"] = True
        proof["stages"].append(execute(guest, python, "pip uninstall -y infinity-test-app"))
        proof["stages"].append(execute(guest, python, "python " + documents + "absent.py"))
        removed, entries = inspect(guest, helper, destination / "removed")
        assert "/packages/infinity_test_base/__init__.py" in entries
        assert not any(path.startswith(("/packages/infinity_test_app/", "/packages/infinity_test_app-")) for path in entries)
        assert removed["archive_sha256"] != upgraded["archive_sha256"] and removed["version"] > upgraded["version"]
        proof["removed_archive"] = removed
        proof["fixture_phases"].append(seed(guest, helper, fixture_dir, "native"))
        resume(guest)
        proof["stages"].append(execute(guest, python, "pip install " + documents + fixtures.COMPATIBLE[args.arch]))
        proof["stages"].append(execute(guest, python, "python " + documents + "native-import.py"))
        native, entries = inspect(guest, helper, destination / "native-installed")
        assert entries["/packages/infinity_native_test.so"]["bytes"] == (ROOT / "builds/toolchains" / args.arch / "infinity_native_test.so").read_bytes()
        resume(guest)
        proof["stages"].append(execute(guest, python, "python " + documents + "native-import.py"))
        proof["stages"].append(execute(guest, python, "pip install " + documents + fixtures.INVALID_NATIVE,
                                       success=False, package_rejected=True))
        rejected, _ = inspect(guest, helper, destination / "native-rejected")
        assert rejected["archive_sha256"] == native["archive_sha256"] and rejected["version"] == native["version"]
        resume(guest)
        proof["stages"].append(execute(guest, python, "pip uninstall -y infinity-native-package"))
        native_removed, entries = inspect(guest, helper, destination / "native-removed")
        assert "/packages/infinity_native_test.so" not in entries
        assert not any(path.startswith("/packages/infinity_native_package-") for path in entries)
        assert native_removed["archive_sha256"] != native["archive_sha256"] and native_removed["version"] > native["version"]
        proof["native_wheel_install_import_cold_boot_uninstall"] = True
        proof["invalid_native_wheel_atomic_rejection"] = True
        if args.source:
            source_lifecycle(guest, python, helper, fixture_dir, destination, proof)
        if args.libraries:
            library_lifecycle(guest, python, helper, fixture_dir, destination, proof)
        proof["installed_acceptance"] = True
        with iso.open("rb") as stream:
            proof["iso_sha256"] = hashlib.file_digest(stream, "sha256").hexdigest()
    except BaseException as error:
        proof["failure"] = repr(error)
        try:
            guest.screenshot("python-package-failure")
            proof["failure_snapshot"] = python.snapshot(guest)
        except Exception as diagnostic_error:
            proof["diagnostic_failure"] = repr(diagnostic_error)
        raise
    finally:
        guest.stop()
        proof["work"] = str(work)
        (destination / "installed.json").write_text(json.dumps(proof, indent=2) + "\n")


if __name__ == "__main__":
    main()
