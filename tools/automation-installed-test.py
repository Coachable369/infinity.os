#!/usr/bin/env python3
"""Fresh-installed daemon/schedule acceptance using real input and typed native state."""
from pathlib import Path
import argparse
import hashlib
import importlib.util
import json
import os
import platform
import shutil
import struct
import subprocess
import time

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("native_editor", ROOT / "tools/native-editor-installed-test.py")
native = importlib.util.module_from_spec(spec)
spec.loader.exec_module(native)
DOCS = "/home/default/documents/"
HELLO = "/home/ms9node01/documents/hello.py"

class AutomationGuest(native.NativeGuest):
    # ------------------------=
    # FUNC: text
    # DESC: Types script punctuation through actual keyboard events without writing guest memory.
    # ------------------=
    def text(self, value):
        native.ui.text(self, value)

# ------------------------=
# FUNC: snapshot
# DESC: Reads seqlocked process lifecycle evidence from the exact kernel extracted from this test's ISO.
# ------------------=
def snapshot(guest):
    address, size = native.base.symbol(guest.work.parent / "artifacts/installed-kernel.elf", "INFINITY_AUTOMATION_SNAPSHOT")
    assert size == 1600
    for _ in range(20):
        values = struct.unpack("<200Q", guest.memory(address, size))
        if values[:2] == (0x494e464155544f31, 1) and values[2] == values[7] and not values[2] & 1:
            assert values[4] <= 16
            rows = {}
            for index in range(values[4]):
                row = values[8 + index * 12:20 + index * 12]
                rows[row[0]] = dict(id=row[0], kind=row[1], state=row[2], runs=row[3],
                    exit=row[4] if row[4] < (1 << 63) else row[4] - (1 << 64),
                    has_exit=bool(row[5]), next=row[6], enabled=bool(row[7]), pid=row[8],
                    executed=bool(row[9]), stdout=row[10], stderr=row[11])
            assert len(rows) == values[4]
            return dict(sequence=values[2], revision=values[3], rows=rows)
        time.sleep(.05)
    return None

# ------------------------=
# FUNC: wait
# DESC: Waits for an observable catalog transition with bounded timeout and numeric failure evidence.
# ------------------=
def wait(guest, predicate, label, timeout=180):
    deadline = time.monotonic() + timeout
    last = None
    while time.monotonic() < deadline:
        assert guest.process.poll() is None, (label, guest.process.returncode)
        last = snapshot(guest)
        if last is not None and predicate(last):
            return last
        time.sleep(.1)
    guest.screenshot("automation-failure")
    raise AssertionError(dict(stage=label, snapshot=last))

# ------------------------=
# FUNC: ui_state
# DESC: Reads the real Task Manager tab and editor state without relying on rendered text.
# ------------------=
def ui_state(guest):
    address, size = native.base.symbol(guest.work.parent / "artifacts/installed-kernel.elf", "INFINITY_TASK_MANAGEMENT_SNAPSHOT")
    assert size == 64
    values = struct.unpack("<8Q", guest.memory(address, size))
    if values[:2] == (0x495441534b554931, 1) and values[2] == values[7] and not values[2] & 1:
        return values
    return None

# ------------------------=
# FUNC: wait_ui
# DESC: Waits for native tab or editor transitions after real keyboard activation.
# ------------------=
def wait_ui(guest, tab, mode):
    deadline = time.monotonic() + 30
    while time.monotonic() < deadline:
        value = ui_state(guest)
        if value and value[3:5] == (tab, mode):
            return value
        time.sleep(.1)
    raise AssertionError(dict(task_manager=ui_state(guest), expected=[tab, mode]))

# ------------------------=
# FUNC: created
# DESC: Submits a real create command and resolves the unique newly allocated stable record identifier.
# ------------------=
def created(guest, command, kind):
    before = set(wait(guest, lambda _: True, "catalog ready")["rows"])
    guest.command(command)
    state = wait(guest, lambda s: len(set(s["rows"]) - before) == 1, "record created")
    identifier = (set(state["rows"]) - before).pop()
    assert state["rows"][identifier]["kind"] == kind
    return identifier

# ------------------------=
# FUNC: completed
# DESC: Requires actual native execution, zero exit and captured script output before accepting completion.
# ------------------=
def completed(guest, identifier, runs):
    state = wait(guest, lambda s: identifier in s["rows"] and s["rows"][identifier]["runs"] >= runs and s["rows"][identifier]["state"] in (5, 6), "native process completion")
    row = state["rows"][identifier]
    assert row["state"] == 5 and row["has_exit"] and row["exit"] == 0 and row["runs"] == runs, row
    assert row["executed"] and row["stdout"] > 0 and row["stderr"] == 0 and row["pid"] == 0, row
    return row

# ------------------------=
# FUNC: gui_crud
# DESC: Creates, edits and deletes a disabled daemon through the actual inline Task Manager form.
# ------------------=
def gui_crud(guest, proof):
    guest.launch("task manager", 5)
    guest.key("tab")
    guest.key("tab")
    guest.key("ret")
    wait_ui(guest, 1, 0)
    proof["screenshots"].append(str(guest.screenshot("task-manager-services")))
    before = set(snapshot(guest)["rows"])
    guest.key("n")
    wait_ui(guest, 1, 1)
    guest.text("guiprobe")
    guest.key("tab")
    guest.text(HELLO)
    for _ in range(3):
        guest.key("tab")
    guest.key("ret")  # Enabled -> disabled.
    guest.key("tab")
    guest.key("tab")
    proof["screenshots"].append(str(guest.screenshot("task-manager-service-form")))
    guest.key("ret")
    wait_ui(guest, 1, 0)
    state = wait(guest, lambda s: len(set(s["rows"]) - before) == 1, "GUI service created")
    identifier = (set(state["rows"]) - before).pop()
    assert not state["rows"][identifier]["enabled"]
    for _ in range(len(state["rows"])):
        guest.key("down")
    guest.key("e")
    wait_ui(guest, 1, 2)
    for _ in range(4):
        guest.key("tab")
    guest.key("ret")  # Enable the manual service without launching it.
    guest.key("tab")
    guest.key("tab")
    guest.key("ret")
    wait_ui(guest, 1, 0)
    wait(guest, lambda s: s["rows"][identifier]["enabled"], "GUI service edited")
    guest.key("delete")
    wait_ui(guest, 1, 3)
    guest.key("shift", "tab")
    guest.key("ret")
    wait_ui(guest, 1, 0)
    wait(guest, lambda s: identifier not in s["rows"], "GUI service deleted")
    # New follows the three tabs in the shared keyboard order.
    guest.key("shift", "tab")
    guest.key("ret")
    wait_ui(guest, 2, 0)
    proof["screenshots"].append(str(guest.screenshot("task-manager-scheduled")))
    proof["gui_crud"] = True
    guest.launch("command", 5)

# ------------------------=
# FUNC: exercise
# DESC: Verifies native runs, cancellation, schedule claims, GUI CRUD and durable reboot state without installer media.
# ------------------=
def exercise(guest, proof):
    assert not guest.installer and guest.state()[3] == 1
    guest.launch("command", 5)
    wait(guest, lambda _: True, "automation runtime ready")
    service = created(guest, "service create proof " + HELLO + " start=manual", 0)
    guest.command("service start proof")
    proof["daemon_execution"] = completed(guest, service, 1)
    guest.command("service restart proof")
    proof["daemon_restart"] = completed(guest, service, 2)
    once = created(guest, "schedule create once " + HELLO + " at=1", 1)
    proof["once_execution"] = completed(guest, once, 1)
    # A long-running real Python sleep makes stop/kill and overlap assertions observable.
    guest.command("object create " + DOCS + "daemonprobe.py")
    guest.command("object write " + DOCS + "daemonprobe.py import time; print(1,flush=True); time.sleep(600)")
    guest.command("service edit proof " + DOCS + "daemonprobe.py enabled=false start=boot delay=2")
    wait(guest, lambda s: not s["rows"][service]["enabled"], "service configuration committed")
    guest.command("service start proof")
    wait(guest, lambda s: s["rows"][service]["state"] == 2 and s["rows"][service]["stdout"] > 0, "real daemon started")
    guest.command("service stop proof")
    stop = wait(guest, lambda s: s["rows"][service]["state"] == 0, "daemon stopped")["rows"][service]
    assert stop["exit"] == -130 and stop["pid"] == 0, stop
    guest.command("service start proof")
    wait(guest, lambda s: s["rows"][service]["state"] == 2 and s["rows"][service]["stdout"] > 0, "daemon restarted")
    guest.command("service kill proof")
    killed = wait(guest, lambda s: s["rows"][service]["state"] == 0, "daemon killed")["rows"][service]
    assert killed["exit"] == -9 and killed["pid"] == 0, killed
    interval = created(guest, "schedule create period " + DOCS + "daemonprobe.py every=1", 1)
    running = wait(guest, lambda s: s["rows"][interval]["state"] == 2 and s["rows"][interval]["stdout"] > 0, "scheduled native process running")["rows"][interval]
    deadline = time.monotonic() + 3
    while time.monotonic() < deadline:
        current = snapshot(guest)
        if current:
            assert current["rows"][interval]["runs"] == running["runs"] == 1
            assert current["rows"][interval]["pid"] == running["pid"]
        time.sleep(.2)
    guest.command("schedule disable period")
    guest.command("schedule kill period")
    wait(guest, lambda s: s["rows"][interval]["state"] == 0, "scheduled process killed")
    guest.command("schedule edit period " + HELLO + " every=120 enabled=false")
    guest.command("schedule run period")
    proof["edited_schedule_execution"] = completed(guest, interval, 2)
    assert not snapshot(guest)["rows"][interval]["enabled"]
    gui_crud(guest, proof)
    before = snapshot(guest)
    guest.stop()
    guest.boot(False)
    guest.authenticate()
    guest.launch("command", 5)
    after = wait(guest, lambda s: all(i in s["rows"] for i in (service, once, interval)), "catalog restored without media")
    for identifier in (service, once, interval):
        assert after["rows"][identifier]["runs"] == before["rows"][identifier]["runs"]
        assert after["rows"][identifier]["enabled"] == before["rows"][identifier]["enabled"]
    assert after["rows"][once]["state"] == 5 and after["rows"][once]["runs"] == 1
    proof["cold_reboot"] = after
    guest.command("service delete proof")
    guest.command("schedule delete once")
    guest.command("schedule delete period")
    wait(guest, lambda s: not any(i in s["rows"] for i in (service, once, interval)), "CLI deletion")
    guest.stop()
    guest.boot(False)
    guest.authenticate()
    after = wait(guest, lambda s: not any(i in s["rows"] for i in (service, once, interval)), "deletion persists across reboot")
    proof["deleted_reboot"] = after

# ------------------------=
# FUNC: main
# DESC: Creates a private installed machine from unchanged ISO bytes and writes a failure-preserving acceptance receipt.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Use ./build-kit run python3 tools/automation-installed-test.py --arch ARCH --iso PATH")
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--arch", choices=("aarch64", "x86_64"), required=True)
    parser.add_argument("--iso", type=Path, required=True)
    parser.add_argument("--reuse", type=Path, help="Reuse a harness-owned build directory; cannot prove a fresh installation")
    parser.add_argument("--firmware", type=Path)
    args = parser.parse_args()
    iso = args.iso.resolve()
    assert iso.is_relative_to(ROOT) and iso.is_file()
    reuse = args.reuse is not None
    work = args.reuse.resolve() if reuse else ROOT / "build" / f"automation-installed-{args.arch}-{time.time_ns()}"
    assert work.parent == ROOT / "build" and work.name.startswith("automation-installed-")
    artifacts = work / "artifacts"
    if not reuse:
        artifacts.mkdir(parents=True)
        native.desktop.copy_artifact(iso, artifacts / "installer.iso")
        receipt = native.desktop.extract_media_kernels(iso, artifacts, args.arch)
    else:
        with iso.open("rb") as source, (artifacts / "installer.iso").open("rb") as previous:
            assert hashlib.file_digest(source, "sha256").digest() == hashlib.file_digest(previous, "sha256").digest()
        receipt = {"reused": True}
    firmware = args.firmware or Path(f"/opt/homebrew/share/qemu/edk2-{args.arch}-code.fd")
    guest = AutomationGuest(work, 1, str(firmware), reuse=reuse, width=1024, height=768, memory_mb=12288)
    guest.arch = args.arch
    guest.acceleration = "hvf" if args.arch == "aarch64" and platform.system() == "Darwin" and platform.machine() == "arm64" else "tcg"
    guest.patched = True
    guest.install_timeout_seconds = 1800 if args.arch == "x86_64" else 900
    guest.boot_timeout_seconds = 300 if args.arch == "x86_64" else 120
    proof = dict(architecture=args.arch, installed_acceptance=False, fresh_install=False,
                 reused_private_disk=reuse, media_kernels=receipt, screenshots=[])
    destination = ROOT / "builds/validation/automation" / args.arch / "installed.json"
    destination.parent.mkdir(parents=True, exist_ok=True)
    try:
        if not reuse:
            guest.install()
            guest.stop()
            subprocess.run(["python3", str(ROOT / "tools/update-installed-clone.py"), str(guest.disk),
                            str(artifacts / "installed-kernel.elf"), "--verify-only"], check=True)
            proof["fresh_install"] = True
        guest.onboard()
        exercise(guest, proof)
        proof["installed_acceptance"] = True
        with iso.open("rb") as source:
            proof["iso_sha256"] = hashlib.file_digest(source, "sha256").hexdigest()
    except BaseException as error:
        proof["failure"] = str(error)
        raise
    finally:
        guest.stop()
        proof["work"] = str(work)
        destination.write_text(json.dumps(proof, indent=2) + "\n")

if __name__ == "__main__":
    main()
