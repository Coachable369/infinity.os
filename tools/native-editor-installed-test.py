"""Native editor compilation acceptance through real input on a disposable installed OS."""
import importlib.util
import argparse
import json
import pathlib
import shutil
import struct
import sys
import time

ROOT = pathlib.Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("editor_ui", ROOT / "tools/editor-assistant-installed-test.py")
ui = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ui)
base = ui.base
guest_spec = importlib.util.spec_from_file_location("desktop_guest", ROOT / "tools/servo-platform-probe/run-desktop-installed.py")
desktop = importlib.util.module_from_spec(guest_spec)
guest_spec.loader.exec_module(desktop)

# ------------------------=
# FUNC: read
# DESC: Reads exported typed state from a stopped guest without modifying memory or invoking privileged test hooks.
# ------------------=
def read(guest, symbol, count):
    elf = guest.work.parent / "artifacts/installed-kernel.elf"
    address, size = base.symbol(elf, symbol)
    assert size == count * 8
    guest.qmp("stop")
    try:
        return struct.unpack(f"<{count}Q", guest.memory(address, size))
    finally:
        guest.qmp("cont")

# ------------------------=
# FUNC: wait_build
# DESC: Waits for real compiler completion and records service state transitions with a bounded deadline.
# ------------------=
def wait_build(guest, previous, expected):
    deadline = time.monotonic() + 180
    transitions = []
    input_latency = None
    while time.monotonic() < deadline:
        state = read(guest, "INFINITY_NATIVE_BUILD_SNAPSHOT", 10)
        assert state[:2] == (0x494e46434c414e47, 1)
        assert state[2] == state[9] and state[2] % 2 == 0
        if state[2] > previous:
            if not transitions or transitions[-1] != state[3]:
                transitions.append(state[3])
            if state[3] in (1, 2) and input_latency is None:
                before = guest.state()
                assert before is not None
                started = time.monotonic()
                guest.qmp("input-send-event", {"events": [{"type": "rel", "data": {
                    "axis": "x", "value": -4 if before[13] > 500 else 4}}]})
                guest.wait(lambda s: s[13] != before[13], "input while native compiler runs", timeout=5)
                input_latency = time.monotonic() - started
            if state[3] >= 3:
                assert state[3] == expected, state
                return {"snapshot": state, "transitions": transitions,
                        "input_latency_seconds": input_latency}
        time.sleep(.2)
    raise AssertionError(("Native build deadline", transitions))

# ------------------------=
# FUNC: compile_document
# DESC: Requests compilation through the actual attached AI composer and confirms through its normal review button when required.
# ------------------=
def compile_document(guest, source, expected):
    ui.panel_click(guest, "toggle") if ui.features(guest)[13] else None
    state = ui.features(guest)
    x, y, w, h = state[21:25]
    ui.click(guest, x + 100, y + 140)
    guest.key("ctrl", "a")
    ui.text(guest, source)
    ui.wait_feature(guest, lambda p: p[7] == len(source))
    previous = read(guest, "INFINITY_NATIVE_BUILD_SNAPSHOT", 10)[2]
    ui.panel_click(guest, "toggle")
    ui.wait_feature(guest, lambda p: p[13] and p[15])
    ui.text(guest, "compile this program")
    guest.key("ret")
    # Explicit commands can apply directly; generated proposals require review.
    if ui.features(guest)[14]:
        ui.panel_click(guest, "apply")
    return wait_build(guest, previous, expected)

# ------------------------=
# FUNC: generate_program
# DESC: Requests actual local model generation, verifies the reviewed typed action, then uses the normal Apply control to compile it.
# ------------------=
def generate_program(guest):
    assert ui.features(guest)[13]
    ui.panel_click(guest, "composer")
    ui.text(guest, "Write and compile a simple C program with int main(void) that returns 54. Replace the entire document.")
    before = read(guest, "INFINITY_NATIVE_BUILD_SNAPSHOT", 10)[2]
    started = time.monotonic()
    guest.key("ret")
    while time.monotonic() - started < 600:
        state = ui.features(guest)
        if state[14]:
            assert state[14] == 14, ("Expected reviewed ReplaceCompile action", state[14])
            guest.screenshot("native-editor-generated-review")
            ui.panel_click(guest, "apply")
            result = wait_build(guest, before, 3)
            result["generation_and_compile_seconds"] = time.monotonic() - started
            return result
        time.sleep(.5)
    raise AssertionError("Local model did not produce a complete compile proposal")

# ------------------------=
# FUNC: main
# DESC: Installs, cold-boots without ISO, compiles valid and invalid source, opens the stored ELF through File Navigator and checks its actual exit.
# ------------------=
def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--arch", choices=("aarch64", "x86_64"), required=True)
    parser.add_argument("work", type=pathlib.Path)
    args = parser.parse_args()
    work = args.work.resolve()
    work.relative_to(ROOT / "build")
    work.mkdir(exist_ok=False)
    artifacts = work / "artifacts"
    artifacts.mkdir()
    shutil.copyfile(ROOT / f"builds/InfinityOS-{args.arch}.iso", artifacts / "installer.iso")
    packaged = desktop.extract_media_kernels(artifacts / "installer.iso", artifacts, args.arch)
    guest = desktop.Guest(work, 1, f"/opt/homebrew/share/qemu/edk2-{args.arch}-code.fd",
                           width=1024, height=768, memory_mb=12288)
    guest.arch = args.arch
    guest.acceleration = "hvf" if args.arch == "aarch64" else "tcg"
    # No post-install patch: this is the unmodified ISO's installed generation.
    guest.patched = True
    guest.boot_timeout_seconds = 600
    report = {"installed_acceptance": False, "architecture": args.arch, "packaged": packaged}
    try:
        guest.install()
        guest.stop()
        report["identity"] = guest.onboard()
        guest.launch("text", 5)
        ui.wait_feature(guest, lambda p: p[12] == 2)
        report["compile"] = compile_document(guest, 'int main(void) { return 54; }', 3)
        published = read(guest, "INFINITY_NATIVE_APP_SNAPSHOT", 8)
        assert published[3] == 1 and any(published[4:6])
        address, size = base.symbol(artifacts / "installed-kernel.elf", "INFINITY_NATIVE_APP_NAME")
        assert size == 28
        name = guest.memory(address, size).decode("ascii")
        guest.screenshot("native-editor-compiled")
        report["invalid"] = compile_document(guest, 'int main(void) { this is not C; }', 4)
        assert read(guest, "INFINITY_NATIVE_APP_SNAPSHOT", 8)[2] == published[2]
        report["generated"] = generate_program(guest)
        assert read(guest, "INFINITY_NATIVE_APP_SNAPSHOT", 8)[2] == published[2] + 1
        name = guest.memory(address, size).decode("ascii")
        guest.launch("file", 5)
        ui.wait_feature(guest, lambda p: 5 <= p[12] < 11)
        ui.panel_click(guest, "toggle")
        ui.wait_feature(guest, lambda p: p[13] and p[15])
        ui.text(guest, "find " + name)
        guest.key("ret")
        if ui.features(guest)[14]:
            ui.panel_click(guest, "apply")
        ui.panel_click(guest, "toggle")
        guest.key("ret")
        deadline = time.monotonic() + 60
        while time.monotonic() < deadline:
            app = read(guest, "INFINITY_NATIVE_APP_SNAPSHOT", 8)
            if app[3] >= 3:
                assert app[3] == 3 and app[6] == 54, app
                report["application"] = app
                break
            time.sleep(.2)
        else:
            raise AssertionError("Stored native executable did not finish")
        guest.screenshot("native-editor-executed")
        report["installed_acceptance"] = True
        report["model_generated_source"] = True
    finally:
        (work / "evidence.json").write_text(json.dumps(report, indent=2) + "\n")
        guest.stop()
        evidence = ROOT / "builds/validation/native-apps" / (args.arch + "-installed")
        evidence.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(work / "evidence.json", evidence / "evidence.json")
        for screenshot in guest.work.glob("*.ppm"):
            shutil.copyfile(screenshot, evidence / screenshot.name)
    print(json.dumps(report))

if __name__ == "__main__":
    main()
