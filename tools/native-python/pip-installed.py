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
def execute(guest, python, command, success=True):
    before = python.snapshot(guest)
    assert before is not None
    guest.command(command)
    deadline = time.monotonic() + 150
    while time.monotonic() < deadline:
        value = python.snapshot(guest)
        if value and value[2] > before[2] and value[3] != 1:
            assert value[3] == 2, (command, value)
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
    assert count <= 512
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
    assert receipt["seeded_inputs"] == 4 and not receipt["package_state_changed"]
    assert receipt["reused_phase"] == phase
    return receipt


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
    args = parser.parse_args()
    iso = args.iso.resolve()
    assert iso.is_relative_to(ROOT) and iso.is_file()
    python = module("native_python_installed", ROOT / "tools/native-python/installed.py")
    fixtures = module("native_package_fixtures", ROOT / "tools/native-python/package-tests.py")
    native = python.native
    work = ROOT / "build" / f"browser-installed-pip-{args.arch}-{time.time_ns()}"
    artifacts = work / "artifacts"
    artifacts.mkdir(parents=True)
    kernels = native.desktop.extract_media_kernels(iso, artifacts, args.arch)
    try:
        layout = validate_media_layout(artifacts, args.arch)
    except ValueError as error:
        parser.error(str(error))
    shutil.copyfile(iso, artifacts / "installer.iso")
    fixture_dir = work / "package-fixtures"
    fixtures.verify_fixtures()
    fixture_manifest = fixtures.write_inputs(fixture_dir)
    helper = ROOT / "build/tools/python-package-disk"
    helper.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run(["rustc", "--edition=2021", "-A", "warnings", "tools/native-python/package-disk.rs",
                    "-o", str(helper)], check=True, cwd=ROOT)
    guest = native.NativeGuest(work, 1, f"/opt/homebrew/share/qemu/edk2-{args.arch}-code.fd",
                               width=1024, height=768, memory_mb=12288)
    guest.arch = args.arch
    guest.acceleration = "hvf" if args.arch == "aarch64" else "tcg"
    guest.patched = True
    guest.install_timeout_seconds = 1800 if args.arch == "x86_64" else 900
    guest.boot_timeout_seconds = 300 if args.arch == "x86_64" else 120
    destination = ROOT / "builds/validation/native-python" / args.arch / ("pip-installed-" + str(time.time_ns()))
    destination.mkdir(parents=True)
    proof = {"architecture": args.arch, "installed_acceptance": False, "media_kernels": kernels,
             "media_layout": layout, "fixture_manifest": fixture_manifest, "stages": [],
             "authenticated_second_user_test": False}
    documents = "/home/default/documents/"
    try:
        guest.install()
        guest.stop()
        subprocess.run(["python3", str(ROOT / "tools/update-installed-clone.py"), str(guest.disk),
                        str(artifacts / "installed-kernel.elf"), "--verify-only"], check=True)
        guest.onboard()
        guest.stop()
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
        proof["installed_acceptance"] = True
        with iso.open("rb") as stream:
            proof["iso_sha256"] = hashlib.file_digest(stream, "sha256").hexdigest()
    finally:
        guest.stop()
        proof["work"] = str(work)
        (destination / "installed.json").write_text(json.dumps(proof, indent=2) + "\n")


if __name__ == "__main__":
    main()
