#!/usr/bin/env python3
"""Exercise installed GUI launch, task pause/resume/end, and resource reclamation."""
from pathlib import Path
import argparse
import importlib.util
import json
import struct
import time

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("tkproof", ROOT / "tools/native-python/tk-installed.py")
tk = importlib.util.module_from_spec(spec)
spec.loader.exec_module(tk)
python = tk.module("pythonproof", ROOT / "tools/native-python/installed.py")
native = python.native


# ------------------------=
# FUNC: progress
# DESC: Reads native process identity, execution counters and registry handle without modifying guest state.
# ------------------=
def progress(guest):
    address, size = native.base.symbol(guest.work.parent / "artifacts/installed-kernel.elf", "INFINITY_NATIVE_PROCESS_PROGRESS")
    assert size == 80
    values = struct.unpack("<10Q", guest.memory(address, size))
    assert values[0] == 1
    return values


# ------------------------=
# FUNC: main
# DESC: Uses real installed-OS input to launch both bundled apps and govern a live process through the shared task commands.
# ------------------=
def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--work", type=Path, required=True)
    parser.add_argument("--firmware", required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    guest = native.NativeGuest(args.work, 1, args.firmware, reuse=True, width=1440, height=1000, memory_mb=20480)
    guest.arch = "aarch64"
    guest.acceleration = "hvf"
    guest.patched = True
    guest.boot_timeout_seconds = 300
    proof = []
    try:
        guest.boot(False)
        guest.authenticate()
        for index, name in enumerate(("InfinityCode", "Infinity Memory", "InfinityCode")):
            before = python.snapshot(guest)[2]
            started = time.monotonic()
            guest.launch(name, 5)
            state = tk.wait(guest, python, lambda: tk.gui(guest, native),
                            lambda value: any(w["visible"] and w["surface"] for w in value["windows"]), timeout=30)
            window = next(w for w in state["windows"] if w["visible"] and w["surface"])
            assert window["height"] <= guest.state()[12] - 164
            elapsed = time.monotonic() - started
            time.sleep(3)
            guest.stable_screenshot(f"app-{index}", (window["x"] + 80, window["y"] + 10, window["width"] - 100, window["height"] - 30))
            task = progress(guest)[8]
            assert task > 0
            if index == 0:
                guest.launch("task manager", 5)
                guest.screenshot("task-manager")
                guest.launch("command", 5)
                guest.command(f"task pause {task}")
                time.sleep(.5)
                paused_state = progress(guest)
                paused = paused_state[2]
                time.sleep(1)
                after_pause = progress(guest)
                guest.screenshot("paused-command")
                assert after_pause[2] == paused, (task, paused_state, after_pause, guest.state())
                guest.command(f"task resume {task}")
                tk.wait(guest, python, lambda: progress(guest)[2], lambda visits: visits > paused)
                guest.command(f"task end {task}")
                ended = tk.wait(guest, python, lambda: python.snapshot(guest),
                                lambda value: value[2] > before and value[3] != 1)
                assert ended[3] == 3 and ended[7] == ended[8], ended
                assert not tk.gui(guest, native)["windows"]
                guest.command("top")
                guest.screenshot("top")
                guest.key("q")
                action = "paused-resumed-terminated"
            else:
                native.adaptive_click(guest, *tk.close_point(guest, window))
                tk.finished(guest, python, before)
                action = "closed"
            proof.append(dict(app=name, window=window, visible_seconds=elapsed, handle=task, action=action))
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(proof, indent=2) + "\n")
    finally:
        guest.stop()


if __name__ == "__main__":
    main()
