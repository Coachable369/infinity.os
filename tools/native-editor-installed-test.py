"""Native editor compilation acceptance through real input on a disposable installed OS."""
import importlib.util
import argparse
import functools
import json
import pathlib
import re
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

# Each run extracts immutable kernel artifacts once. Avoid rereading a gigabyte
# ELF on every guest-state poll while measuring responsiveness.
base.symbol = functools.lru_cache(maxsize=64)(base.symbol)
desktop.base.symbol = base.symbol

# ------------------------=
# FUNC: adaptive_click
# DESC: Uses measured display geometry and the existing acceleration-aware real pointer driver.
# ------------------=
def adaptive_click(guest, x, y, button="left"):
    state = guest.wait(lambda s: s[11] > 0 and s[12] > 0, "display geometry")
    guest.width, guest.height = state[11:13]
    guest.click(x, y, button=button)

ui.click = adaptive_click

class NativeGuest(desktop.Guest):
    # ------------------------=
    # FUNC: key
    # DESC: Waits for delivered input, allowing focused composers to consume key release without publishing a second snapshot.
    # ------------------=
    def key(self, *codes):
        before = self.state()
        self.qmp("send-key", {"keys": [{"type": "qcode", "data": code} for code in codes], "hold-time": 150})
        time.sleep(.2)
        transition = before is not None and codes == ("ret",) and (
            before[4] in (9, 10) and before[8] == 1 or before[4] == 3 and before[5] == 6)
        if before is not None and not transition:
            self.wait(lambda state: state[2] > before[2], "delivered keyboard input", timeout=30)

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
    input_started = None
    input_x = None
    while time.monotonic() < deadline:
        state = read(guest, "INFINITY_NATIVE_BUILD_SNAPSHOT", 10)
        assert state[:2] == (0x494e46434c414e47, 1)
        assert state[2] == state[9] and state[2] % 2 == 0
        if state[2] > previous:
            if not transitions or transitions[-1] != state[3]:
                transitions.append(state[3])
            if input_started is not None and input_latency is None:
                observed = guest.state()
                if observed is not None and observed[13] != input_x:
                    input_latency = time.monotonic() - input_started
            if state[3] in (1, 2) and input_started is None:
                before = guest.state()
                assert before is not None
                input_started = time.monotonic()
                input_x = before[13]
                guest.qmp("input-send-event", {"events": [{"type": "rel", "data": {
                    "axis": "x", "value": -4 if before[13] > 500 else 4}}]})
            if state[3] >= 3:
                assert state[3] == expected, state
                if input_started is not None and input_latency is None:
                    guest.wait(lambda s: s[13] != input_x, "pointer after compiler completion", timeout=5)
                    input_latency = time.monotonic() - input_started
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
    parser.add_argument("--iso", type=pathlib.Path, help="Explicit QEMU-compatible test media for ARM; never substitute silently for the published VirtualBox ISO")
    parser.add_argument("--reuse-installed", action="store_true", help="Continue failed workflow checks on this harness's already cold-boot-verified disk")
    parser.add_argument("--accel", choices=("hvf", "tcg"), help="Explicit accelerator for diagnosing installed execution transitions")
    parser.add_argument("work", type=pathlib.Path)
    args = parser.parse_args()
    if args.arch == "aarch64" and args.iso is None:
        parser.error("ARM QEMU requires explicit --iso media linked for its RAM layout; the published ARM ISO targets VirtualBox")
    work = args.work.resolve()
    work.relative_to(ROOT / "build")
    work.mkdir(exist_ok=args.reuse_installed)
    artifacts = work / "artifacts"
    artifacts.mkdir(exist_ok=args.reuse_installed)
    source_iso = args.iso.resolve() if args.iso else ROOT / f"builds/InfinityOS-{args.arch}.iso"
    source_iso.relative_to(ROOT)
    previous_report = json.loads((work / "evidence.json").read_text()) if args.reuse_installed else None
    if previous_report:
        assert previous_report["identity"]["cold_boot_identity"]
        assert previous_report["source_media"] == str(source_iso)
        packaged = previous_report["packaged"]
    else:
        shutil.copyfile(source_iso, artifacts / "installer.iso")
        packaged = desktop.extract_media_kernels(artifacts / "installer.iso", artifacts, args.arch)
    guest = NativeGuest(work, 1, f"/opt/homebrew/share/qemu/edk2-{args.arch}-code.fd",
                           reuse=args.reuse_installed, width=1024, height=768, memory_mb=12288)
    guest.arch = args.arch
    guest.acceleration = args.accel or ("hvf" if args.arch == "aarch64" else "tcg")
    # No post-install patch: this is the unmodified ISO's installed generation.
    guest.patched = True
    guest.boot_timeout_seconds = 600
    report = {"installed_acceptance": False, "architecture": args.arch, "packaged": packaged,
              "source_media": str(source_iso), "canonical_release_media": args.iso is None,
              "acceleration": guest.acceleration}
    try:
        if args.reuse_installed:
            guest.boot(False)
            guest.authenticate()
            report["identity"] = previous_report["identity"]
        else:
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
        assert all(result["input_latency_seconds"] is None or result["input_latency_seconds"] < 5
                   for result in (report["compile"], report["invalid"], report["generated"])), "Compiler input latency exceeded five seconds"
        report["installed_acceptance"] = True
        report["model_generated_source"] = True
    except BaseException as error:
        report["failure"] = repr(error)
        if guest.process is not None and guest.process.poll() is None:
            try:
                registers = guest.qmp("human-monitor-command", {"command-line": "info registers"})
                # Retain execution addresses only, never general/vector registers
                # which can contain private model input or authentication data.
                report["cpu_diagnostic"] = dict(re.findall(r"\b(PC|SP|RIP|RSP|RBP)=([0-9a-fA-F]+)", registers))
                guest.screenshot("native-editor-failure")
            except (OSError, ValueError) as diagnostic_error:
                report["diagnostic_failure"] = type(diagnostic_error).__name__
        raise
    finally:
        (work / "evidence.json").write_text(json.dumps(report, indent=2) + "\n")
        try:
            guest.stop()
        except OSError:
            if guest.process is not None and guest.process.poll() is None:
                guest.process.terminate()
                guest.process.wait(timeout=20)
        evidence = ROOT / "builds/validation/native-apps" / (args.arch + "-installed")
        evidence.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(work / "evidence.json", evidence / "evidence.json")
        for screenshot in guest.work.glob("*.ppm"):
            shutil.copyfile(screenshot, evidence / screenshot.name)
    print(json.dumps(report))

if __name__ == "__main__":
    main()
