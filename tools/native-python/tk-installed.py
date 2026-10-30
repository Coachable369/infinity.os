#!/usr/bin/env python3
"""Exercise genuine Tk widgets through actual installed-OS input and pixels.

This driver seeds only four source fixtures on a stopped disposable disk. It
never injects application state, writes guest RAM, patches a kernel, or supplies
widget responses. Native snapshots describe geometry/lifecycle only.
"""
from pathlib import Path
import argparse
import hashlib
import importlib.util
import json
import os
import shutil
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[2]


# ------------------------=
# FUNC: module
# DESC: Reuses installed-media, real-input and typed process-receipt helpers without introducing a VM framework.
# ------------------=
def module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


# ------------------------=
# FUNC: gui
# DESC: Decodes only seqlocked public native geometry, preserving signed desktop positions.
# ------------------=
def gui(guest, native):
    raw = native.read(guest, "INFINITY_NATIVE_GUI_SNAPSHOT", 80)
    if raw[0] == 0:
        return dict(sequence=0, windows=[], focused=0, owner=0, revision=0)
    assert raw[:2] == (0x494e464e47554931, 1) and raw[3] <= 8, raw
    windows = []
    for index in range(raw[3]):
        row = raw[8 + index * 8:16 + index * 8]
        window = dict(zip(("id", "x", "y", "width", "height", "state", "visible", "surface"), row))
        for name in ("x", "y"):
            if window[name] >= 1 << 63:
                window[name] -= 1 << 64
        assert window["state"] <= 4 and window["visible"] <= 1
        windows.append(window)
    return dict(sequence=raw[2], windows=windows, focused=raw[4], owner=raw[5], revision=raw[6])


# ------------------------=
# FUNC: wait
# DESC: Waits for bounded observable state while immediately reporting a native process failure.
# ------------------=
def wait(guest, python, read, predicate, timeout=30):
    deadline = time.monotonic() + timeout
    while True:
        value = read()
        if predicate(value):
            return value
        execution = python.snapshot(guest)
        assert not execution or execution[3] < 2 or execution[4] == 0, execution
        assert time.monotonic() < deadline, value
        time.sleep(.15)


# ------------------------=
# FUNC: screenshot
# DESC: Captures real framebuffer pixels and parses bounded QEMU PPM data without OCR or a text oracle.
# ------------------=
def screenshot(guest, label):
    path = guest.screenshot(label)
    with path.open("rb") as stream:
        assert stream.readline().strip() == b"P6"
        dimensions = stream.readline()
        while dimensions.startswith(b"#"):
            dimensions = stream.readline()
        width, height = map(int, dimensions.split())
        assert 1 <= width <= 8192 and 1 <= height <= 8192
        assert int(stream.readline()) == 255
        data = stream.read(width * height * 3 + 1)
    assert len(data) == width * height * 3
    return dict(path=str(path), width=width, height=height, pixels=data)


# ------------------------=
# FUNC: pixel
# DESC: Reads one actual rendered RGB value at a checked framebuffer coordinate.
# ------------------=
def pixel(capture, x, y):
    assert 0 <= x < capture["width"] and 0 <= y < capture["height"], (x, y)
    offset = (y * capture["width"] + x) * 3
    return tuple(capture["pixels"][offset:offset + 3])


# ------------------------=
# FUNC: observe
# DESC: Waits for real Canvas state cells and records screenshot evidence without accepting prose or source text as proof.
# ------------------=
def observe(guest, native, python, indices, label):
    deadline = time.monotonic() + 25
    while True:
        windows = gui(guest, native)["windows"]
        assert len(windows) == 1, windows
        window = windows[0]
        capture = screenshot(guest, label)
        colors = [pixel(capture, window["x"] + 32 + index * 40, window["y"] + 32) for index in indices]
        if all(color == (34, 197, 94) for color in colors):
            return dict(window=window, cells=indices, screenshot=capture["path"])
        execution = python.snapshot(guest)
        assert not execution or execution[3] < 2, execution
        assert time.monotonic() < deadline, dict(window=window, actual=colors, expected=(34, 197, 94))
        time.sleep(.2)


# ------------------------=
# FUNC: drag
# DESC: Sends paired hardware pointer edges and intermediate motion using the existing acceleration-aware driver.
# ------------------=
def drag(guest, start, end):
    guest.width, guest.height = guest.state()[11:13]
    guest.click(*start, press=False)
    guest.qmp("input-send-event", {"events": [{"type": "btn", "data": {"button": "left", "down": True}}]})
    guest.wait(lambda value: value[15] & 1, "native Tk chrome press")
    try:
        guest.click(*end, press=False)
    finally:
        guest.qmp("input-send-event", {"events": [{"type": "btn", "data": {"button": "left", "down": False}}]})
    guest.wait(lambda value: not value[15] & 1, "native Tk chrome release")


# ------------------------=
# FUNC: close_point
# DESC: Uses the shared OS chrome's documented rightmost-control geometry outside application client pixels.
# ------------------=
def close_point(guest, window):
    width, height = guest.state()[11:13]
    scale = 2 if width >= 2560 and height >= 1440 else 1
    return window["x"] + window["width"] - 25 * scale, window["y"] - 22 * scale


# ------------------------=
# FUNC: finished
# DESC: Requires a new successful native CPython receipt with complete physical-memory reclamation.
# ------------------=
def finished(guest, python, previous):
    value = wait(guest, python, lambda: python.snapshot(guest),
                 lambda state: state and state[2] > previous and state[3] != 1, timeout=90)
    assert value[3] == 2 and value[4] == 0 and value[7] == value[8], value
    return value


# ------------------------=
# FUNC: window_cycles
# DESC: Corroborates three actual eight-window allocations, complete child cleanup and true withdrawal using Window Server state.
# ------------------=
def window_cycles(guest, native, python):
    previous = python.snapshot(guest)[2]
    guest.command("python /home/default/documents/tk/tk-window-cycle.py")
    deadline = time.monotonic() + 90
    allocations = set()
    withdrawn = False
    restored_after = set()
    latest_group = None
    while True:
        state = gui(guest, native)
        windows = state["windows"]
        if len(windows) == 1 and windows[0]["state"] == 4 and not windows[0]["visible"]:
            withdrawn = True
        if len(windows) == 8 and all(window["visible"] for window in windows):
            latest_group = tuple(sorted(window["id"] for window in windows))
            allocations.add(latest_group)
        elif len(windows) == 1 and windows[0]["visible"] and latest_group:
            restored_after.add(latest_group)
        execution = python.snapshot(guest)
        if execution and execution[2] > previous and execution[3] != 1:
            assert execution[3] == 2 and execution[4] == 0 and execution[7] == execution[8], execution
            assert withdrawn and len(allocations) == 3 and restored_after == allocations, dict(
                withdrawn=withdrawn, allocations=sorted(allocations), cleaned=sorted(restored_after))
            return dict(receipt=execution, withdrawn=True, window_id_groups=sorted(allocations),
                        each_group_destroyed=True, memory_reclaimed=True)
        assert time.monotonic() < deadline, state
        time.sleep(.05)


# ------------------------=
# FUNC: interactive
# DESC: Proves Tk timer, Entry input, ttk callback, PhotoImage/Canvas pixels, pack resize, movable OS chrome and close-veto behavior.
# ------------------=
def interactive(guest, native, python, control_selection=False):
    previous = python.snapshot(guest)[2]
    command_bounds = native.features(guest)[21:25]
    backdrop = screenshot(guest, "tk-desktop-before")
    guest.command("python /home/default/documents/tk/tk-acceptance.py" +
                  (" --control-selection" if control_selection else ""))
    initial = wait(guest, python, lambda: gui(guest, native),
                   lambda state: len(state["windows"]) == 1 and state["windows"][0]["width"] == 620
                   and state["windows"][0]["height"] == 420, timeout=90)
    result = dict(initial=initial, stages=[])
    ready = observe(guest, native, python, [0, 5], "tk-initial")
    result["stages"].append(ready)
    window = ready["window"]
    capture = screenshot(guest, "tk-images")
    checks = [(24, 174, (226, 232, 240)), (32, 174, (56, 189, 248)),
              (130, 200, (56, 189, 248)), (101, 171, (11, 18, 32)),
              (window["width"] - 16, window["height"] - 16, (244, 63, 94))]
    for x, y, expected in checks:
        assert pixel(capture, window["x"] + x, window["y"] + y) == expected, (x, y, expected)
    result["image_pixels"] = dict(screenshot=capture["path"], checks=checks)
    native.ui.click(guest, window["x"] + 100, window["y"] + 99)
    if control_selection:
        for character in "discard":
            guest.key(character)
        guest.key("ctrl", "a")
    for character in "native42":
        guest.key(character)
    result["stages"].append(observe(guest, native, python, [0, 1, 5], "tk-entry"))
    native.ui.click(guest, window["x"] + 450, window["y"] + 99)
    result["stages"].append(observe(guest, native, python, [0, 1, 2, 5], "tk-button"))
    drag(guest, (window["x"] + 80, window["y"] - 22), (window["x"] + 120, window["y"] - 2))
    moved = wait(guest, python, lambda: gui(guest, native),
                 lambda state: len(state["windows"]) == 1 and abs(state["windows"][0]["x"] - window["x"]) >= 20)
    result["moved"] = moved
    window = moved["windows"][0]
    assert window["width"] == 620 and window["height"] == 420
    drag(guest, (window["x"] + window["width"] + 2, window["y"] + window["height"] + 2),
         (window["x"] + 724, window["y"] + 504))
    resized = observe(guest, native, python, [0, 1, 2, 3, 5], "tk-resized")
    result["stages"].append(resized)
    window = resized["window"]
    capture = screenshot(guest, "tk-resized-layout")
    assert pixel(capture, window["x"] + window["width"] - 16,
                 window["y"] + window["height"] - 16) == (244, 63, 94)
    native.ui.click(guest, *close_point(guest, window))
    result["stages"].append(observe(guest, native, python, list(range(6)), "tk-close-veto"))
    assert python.snapshot(guest)[3] == 1
    native.ui.click(guest, *close_point(guest, window))
    result["exit"] = finished(guest, python, previous)
    result["closed"] = wait(guest, python, lambda: gui(guest, native), lambda state: not state["windows"])
    # The left desktop strip is outside the console's changing output, top bar,
    # clock, dock and status widgets. Both moved Tk client/chrome occupied it.
    right = min(command_bounds[0] - 8, backdrop["width"] // 3, window["x"] + window["width"])
    points = [(x, y) for y in range(max(88, window["y"] - 44),
                                    min(backdrop["height"] - 120, window["y"] + window["height"]), 8)
              for x in range(max(8, window["x"]), right, 8)]
    assert len(points) >= 50, (command_bounds, window, len(points))
    deadline = time.monotonic() + 20
    while True:
        restored = screenshot(guest, "tk-desktop-restored")
        differences = [(x, y, pixel(backdrop, x, y), pixel(restored, x, y))
                       for x, y in points if pixel(backdrop, x, y) != pixel(restored, x, y)]
        if not differences:
            break
        assert time.monotonic() < deadline, differences[:12]
        time.sleep(.2)
    result["cleanup_pixels"] = dict(before=backdrop["path"], after=restored["path"],
                                    exact_restored_samples=len(points))
    result["passed"] = True
    result["control_selection"] = control_selection
    return result


# ------------------------=
# FUNC: manual_demo
# DESC: Launches the standard upstream python -m tkinter demo and closes it through actual OS chrome.
# ------------------=
def manual_demo(guest, native, python):
    guest.launch("command", 5)
    previous = python.snapshot(guest)[2]
    guest.command("python -m tkinter")
    state = wait(guest, python, lambda: gui(guest, native), lambda value: len(value["windows"]) == 1
                 and value["windows"][0]["visible"] and value["windows"][0]["width"] > 100, timeout=90)
    capture = screenshot(guest, "tk-standard-module-demo")
    window = state["windows"][0]
    # A real demo must paint multiple widget/text colors; an empty published
    # surface or title-only window cannot satisfy the pixel evidence.
    colors = {pixel(capture, x, y) for y in range(window["y"], window["y"] + window["height"], 3)
              for x in range(window["x"], window["x"] + window["width"], 3)}
    assert len(colors) > 12, len(colors)
    native.ui.click(guest, *close_point(guest, window))
    return dict(window=window, screenshot=capture["path"], colors=len(colors),
                exit=finished(guest, python, previous))


# ------------------------=
# FUNC: durable_data
# DESC: Verifies actual SQLite full-sync commits, hot-journal recovery and file isolation across detached cold boots and abnormal native exits.
# ------------------=
def durable_data(guest, python, pip, helper, fixtures):
    proof = dict(fixtures=pip.seed(guest, helper, fixtures, "data"), stages=[])
    pip.resume(guest)
    command = "python /home/default/documents/data/data-acceptance.py "
    proof["stages"].append(pip.execute(guest, python, command + "write"))
    guest.stop()
    pip.resume(guest)
    proof["stages"].append(pip.execute(guest, python, command + "read-a"))
    committed = pip.execute(guest, python, command + "commit-crash", success=False)
    assert committed["receipt"][4] == 19
    proof["stages"].append(committed)
    guest.stop()
    pip.resume(guest)
    proof["stages"].append(pip.execute(guest, python, command + "read-b"))
    interrupted = pip.execute(guest, python, command + "pending-crash", success=False)
    assert interrupted["receipt"][4] == 23
    proof["stages"].append(interrupted)
    guest.stop()
    pip.resume(guest)
    proof["stages"].append(pip.execute(guest, python, command + "read-b"))
    proof["stages"].append(pip.execute(guest, python, "python /home/default/documents/data/data-isolation.py"))
    proof["stages"].append(pip.execute(guest, python, command + "read-b"))
    proof["detached_cold_boots"] = 3
    proof["durable_full_sync_and_hot_journal_recovery"] = True
    return proof


# ------------------------=
# FUNC: infinitycode
# DESC: Launches the installed IDE through its real command and verifies painted client pixels, input responsiveness and clean native teardown.
# ------------------=
def infinitycode(guest, native, python):
    previous = python.snapshot(guest)[2]
    guest.command("infinitycode")
    state = wait(guest, python, lambda: gui(guest, native),
                 lambda value: len(value["windows"]) == 1 and value["windows"][0]["visible"]
                 and value["windows"][0]["width"] >= 600, timeout=150)
    window = state["windows"][0]
    capture = wait(guest, python, lambda: screenshot(guest, "infinitycode-open"),
                   lambda image: sum(pixel(image, x, y) == (9, 15, 38)
                       for y in range(max(0, window["y"]), min(image["height"], window["y"] + window["height"]), 4)
                       for x in range(max(0, window["x"]), min(image["width"], window["x"] + window["width"]), 4)) > 1000,
                   timeout=150)
    colors = {pixel(capture, x, y)
              for y in range(max(0, window["y"]), min(capture["height"], window["y"] + window["height"]), 3)
              for x in range(max(0, window["x"]), min(capture["width"], window["x"] + window["width"]), 3)}
    assert len(colors) > 12, len(colors)
    before = guest.state()[2]
    guest.key("tab")
    assert guest.state()[2] > before
    native.ui.click(guest, *close_point(guest, window))
    receipt = finished(guest, python, previous)
    wait(guest, python, lambda: gui(guest, native), lambda value: not value["windows"])
    return dict(window=window, screenshot=capture["path"], colors=len(colors), exit=receipt)


# ------------------------=
# FUNC: main
# DESC: Installs unchanged media or reuses a stopped disposable installation, seeds source inputs and verifies native GUI behavior without a host GUI backend.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Use ./build-kit run python3 tools/native-python/tk-installed.py --arch ARCH --iso PATH")
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--arch", choices=("aarch64", "x86_64"), required=True)
    parser.add_argument("--iso", type=Path, required=True)
    parser.add_argument("--reuse-work", type=Path, help="Reuse an unchanged stopped disposable installed-test disk; does not claim a new installation")
    parser.add_argument("--control-selection", action="store_true",
                        help="Require real Control+A selection replacement; media must include normalized native shortcut delivery")
    parser.add_argument("--durable-data", action="store_true",
                        help="Require real source-scoped persistent files and SQLite after cold boots; needs updated data-capable kernel")
    parser.add_argument("--infinitycode-only", action="store_true",
                        help="Exercise installed InfinityCode launch, drawing, input and clean close")
    args = parser.parse_args()
    iso = args.iso.resolve()
    assert iso.is_relative_to(ROOT) and iso.is_file()
    pip = module("tk_installed_package_helpers", ROOT / "tools/native-python/pip-installed.py")
    python = module("tk_native_python", ROOT / "tools/native-python/installed.py")
    native = python.native
    work = args.reuse_work.resolve() if args.reuse_work else ROOT / "build" / ("browser-installed-tk-" + args.arch + "-" + str(time.time_ns()))
    assert work.is_relative_to(ROOT / "build")
    artifacts = work / "artifacts"
    artifacts.mkdir(parents=True, exist_ok=bool(args.reuse_work))
    if args.reuse_work:
        assert (artifacts / "installer.iso").is_file()
        for path in (iso, artifacts / "installer.iso"):
            with path.open("rb") as stream:
                digest = hashlib.file_digest(stream, "sha256").hexdigest()
            if path == iso:
                expected_digest = digest
            else:
                assert digest == expected_digest, "Reused disk belongs to different installer media"
    else:
        # APFS clones preserve an independent immutable test input without
        # reserving another complete model-bearing ISO on the same volume.
        if sys.platform == "darwin":
            subprocess.run(["cp", "-c", str(iso), str(artifacts / "installer.iso")], check=True)
        else:
            shutil.copyfile(iso, artifacts / "installer.iso")
        native.desktop.extract_media_kernels(iso, artifacts, args.arch)
    layout = pip.validate_media_layout(artifacts, args.arch)
    helper = ROOT / "build/tools/python-package-disk"
    helper.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run(["rustc", "--edition=2021", "-A", "warnings", "tools/native-python/package-disk.rs", "-o", str(helper)], check=True, cwd=ROOT)
    guest = native.NativeGuest(work, 1, f"/opt/homebrew/share/qemu/edk2-{args.arch}-code.fd",
                               reuse=bool(args.reuse_work), width=1024, height=768, memory_mb=12288)
    active = subprocess.run(["lsof", "-t", str(guest.disk)], capture_output=True)
    assert active.returncode in (0, 1) and not active.stdout, "Disposable test disk is still open by another process"
    guest.arch = args.arch
    guest.acceleration = "hvf" if args.arch == "aarch64" else "tcg"
    guest.patched = True
    guest.install_timeout_seconds = 1800 if args.arch == "x86_64" else 900
    guest.boot_timeout_seconds = 300
    fixtures = work / ("tk-inputs-" + str(time.time_ns()))
    fixtures.mkdir()
    for name in ("tk-acceptance.py", "tk-import.py", "tk-window-cycle.py", "tk-widget-contract.py", "data-acceptance.py", "data-isolation.py"):
        shutil.copyfile(ROOT / "tools/native-python" / name, fixtures / name)
    destination = ROOT / "builds/validation/native-python" / args.arch / ("tk-installed-" + str(time.time_ns()))
    destination.mkdir(parents=True)
    proof = dict(installed_acceptance=False, architecture=args.arch, media=str(iso), media_layout=layout,
                 work=str(work), fresh_installation=not bool(args.reuse_work), host_gui_backend=False, stages=[])
    try:
        if not args.reuse_work:
            guest.install()
            guest.stop()
            proof["identity"] = guest.onboard()
            guest.stop()
        subprocess.run(["python3", str(ROOT / "tools/update-installed-clone.py"), str(guest.disk),
                        str(artifacts / "installed-kernel.elf"), "--verify-only"], check=True)
        if not args.infinitycode_only:
            proof["fixtures"] = pip.seed(guest, helper, fixtures, "tk")
        if args.infinitycode_only:
            guest.boot(False)
            guest.authenticate()
            # The isolated launch probe uses the existing keyboard launcher;
            # pointer delivery is checked separately after the IDE has drawn.
            deadline = time.monotonic() + 45
            while guest.state()[4] == 5 and time.monotonic() < deadline:
                guest.key("slash")
                time.sleep(1)
            assert guest.state()[4] == 6, "Keyboard launcher did not become interactive"
            guest.text("command")
            guest.key("ret")
            guest.wait(lambda state: state[4] == 5, "command window selected")
            proof["infinitycode"] = infinitycode(guest, native, python)
            proof["installed_acceptance"] = True
            return
        pip.resume(guest)
        for name in ("tk-import.py", "tk-widget-contract.py"):
            proof["stages"].append(pip.execute(guest, python, "python /home/default/documents/tk/" + name))
        proof["window_cycles"] = window_cycles(guest, native, python)
        proof["interactive"] = interactive(guest, native, python, args.control_selection)
        proof["module_demo"] = manual_demo(guest, native, python)
        if args.durable_data:
            proof["durable_data"] = durable_data(guest, python, pip, helper, fixtures)
        proof["installed_acceptance"] = True
    finally:
        try:
            if guest.process:
                try:
                    proof["final_python"] = python.snapshot(guest)
                    proof["final_gui"] = gui(guest, native)
                    guest.screenshot("tk-final")
                except Exception as error:
                    proof["final_capture_error"] = str(error)
        finally:
            guest.stop()
            (destination / "installed.json").write_text(json.dumps(proof, indent=2) + "\n")


if __name__ == "__main__":
    main()
