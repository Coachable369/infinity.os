#!/usr/bin/env python3
"""Capture real PEP 517 failure scrollback from an unchanged installed guest.

Diagnostic pixels and numeric process state only; this is not a passing
acceptance gate and never changes the installed kernel or package inputs.
"""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import time

ROOT = Path(__file__).resolve().parents[2]


# ------------------------=
# FUNC: load
# DESC: Reuses the installed guest and terminal input drivers without creating another execution path.
# ------------------=
def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


# ------------------------=
# FUNC: main
# DESC: Runs the failing source install through the actual dock and captures all retained diagnostic pages with real scrollbar input.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Use ./build-kit run python3 tools/native-python/source-diagnostic.py --work PATH")
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--work", type=Path, required=True)
    args = parser.parse_args()
    work = args.work.resolve()
    assert work.is_relative_to(ROOT / "build") and (work / "node-1/installed.raw").is_file()
    python = load("source_diagnostic_python", ROOT / "tools/native-python/installed.py")
    terminal = load("source_diagnostic_terminal", ROOT / "tools/editor-terminal-installed-test.py")
    native = python.native
    guest = native.NativeGuest(work, 1, "/opt/homebrew/share/qemu/edk2-aarch64-code.fd",
                               reuse=True, width=1024, height=768, memory_mb=12288)
    guest.arch, guest.acceleration, guest.patched = "aarch64", "hvf", True
    directory = work / ("source-diagnostics-" + str(time.time_ns()))
    directory.mkdir()
    receipt = {"acceptance": False, "diagnostic_only": True, "pages": []}
    try:
        subprocess.run(["python3", str(ROOT / "tools/update-installed-clone.py"), str(guest.disk),
                        str(work / "artifacts/installed-kernel.elf"), "--verify-only"], check=True)
        guest.boot(False)
        guest.authenticate()
        guest.launch("text", 5)
        editor = native.features(guest)
        native.ui.click(guest, editor[21] + editor[23] - 59, editor[22] + 24)
        native.ui.wait_feature(guest, lambda value: value[24] > editor[24])
        if not terminal.terminal(guest, native)["open"]:
            guest.key("ctrl", "grave_accent")
        terminal.wait(lambda: terminal.terminal(guest, native), lambda value: value["open"])
        terminal.click(guest, native, "input")
        before = python.snapshot(guest)
        terminal.type_text(guest, "pip install /home/default/documents/infinity_source_probe-1.0.0.zip")
        guest.key("ret")
        result = terminal.wait(lambda: python.snapshot(guest),
                               lambda value: value and value[2] > before[2] and value[3] != 1, 650)
        receipt["python"] = result
        for page in range(30):
            state = terminal.terminal(guest, native)
            ppm = guest.screenshot("source-trace-" + str(page))
            png = directory / (str(page) + ".png")
            subprocess.run(["sips", "-s", "format", "png", str(ppm), "--out", str(png)], check=True,
                           stdout=subprocess.DEVNULL)
            receipt["pages"].append({"image": str(png), "terminal": state})
            # Use the actual scroll track; baseline QEMU wheel delivery does
            # not reliably reach this new dock even though pointer dragging does.
            x, y, width, height = state["thumb"]
            track = state["track"]
            native.ui.click(guest, x + width // 2,
                            max(track[1] + 1, y - max(1, height // 2)))
            after = terminal.terminal(guest, native)
            if after["scroll"] == state["scroll"]:
                break
    finally:
        guest.stop()
        (directory / "receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")
        print(json.dumps({"diagnostic_only": True, "receipt": str(directory / "receipt.json")}))


if __name__ == "__main__":
    main()
