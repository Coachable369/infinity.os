"""Native editor compilation acceptance through real input on a disposable installed OS."""
import importlib.util
import argparse
import functools
import hashlib
import json
import pathlib
import re
import shutil
import struct
import subprocess
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
    # FUNC: launch
    # DESC: Uses the dock launcher so a restored editor cannot consume slash as document text.
    # ------------------=
    def launch(self, query, mode, section=None):
        state = self.state()
        if state[4] in (9, 10):
            state = self.authenticate()
        width, height = state[11:13]
        scale = 2 if width >= 2560 and height >= 1440 else 1
        dock_width = width * 54 // 100
        adaptive_click(self, (width-dock_width)//2 + (dock_width//8)//2, height-46*scale)
        self.wait(lambda s: s[4] == 6, "dock launcher opened", timeout=30)
        ui.text(self, query)
        self.key("ret")
        return self.wait(lambda s: s[4] == mode and (section is None or s[8] == section), "selected app", timeout=30)

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
# DESC: Reads exported typed state without stopping inference workers or modifying guest memory.
# ------------------=
def read(guest, symbol, count):
    elf = guest.work.parent / "artifacts/installed-kernel.elf"
    address, size = base.symbol(elf, symbol)
    assert size == count * 8
    for _ in range(20):
        state = struct.unpack(f"<{count}Q", guest.memory(address, size))
        # App publication uses a monotonic counter, while the feature/build
        # snapshots publish matching even sequence words at both ends.
        if count == 8 or state[2] == state[-1] and state[2] % 2 == 0:
            return state
        time.sleep(.01)
    raise AssertionError("Diagnostic snapshot remained in flight")

# ------------------------=
# FUNC: features
# DESC: Reads the seqlocked app snapshot without pausing SMP inference workers under HVF.
# ------------------=
def features(guest):
    state = read(guest, "INFINITY_APP_FEATURE_SNAPSHOT", 32)
    assert state[:2] == (0x494e464150505331, 1)
    assert state[3:7] == (0x1ff, 16384, 8, 9)
    return state

ui.features = features

# ------------------------=
# FUNC: wait_build
# DESC: Waits for real compiler completion and records service state transitions with a bounded deadline.
# ------------------=
def wait_build(guest, previous, expected):
    started = time.monotonic()
    deadline = started + 180
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
                result = {"snapshot": state, "transitions": transitions,
                          "input_latency_seconds": input_latency,
                          "elapsed_seconds": time.monotonic() - started}
                (guest.work / "native-build-last.json").write_text(json.dumps(result, indent=2) + "\n")
                assert state[3] == expected, state
                if input_started is not None and input_latency is None:
                    guest.wait(lambda s: s[13] != input_x, "pointer after compiler completion", timeout=5)
                    input_latency = time.monotonic() - input_started
                result["input_latency_seconds"] = input_latency
                return result
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
    ui.text(guest, "save and compile my code" if getattr(guest, "save_compile", False) else "compile this program")
    if getattr(guest, "trace_native_faults", False):
        guest.qmp("trace-event-set-state", {"name": "hvf_data_abort", "enable": True})
    guest.key("ret")
    if getattr(guest, "save_compile", False) and not getattr(guest, "source_named", False):
        # The first source has no namespace object. Cancellation must not start
        # the compiler, and retrying must resume once after a real Save As.
        ui.wait_editor_dialog(guest, 1)
        guest.key("esc")
        ui.wait_editor_dialog(guest, 0)
        assert read(guest, "INFINITY_NATIVE_BUILD_SNAPSHOT", 10)[2] == previous
        ui.panel_click(guest, "composer")
        ui.text(guest, "save and compile my code")
        guest.key("ret")
        ui.wait_editor_dialog(guest, 1)
        ui.text(guest, "save-compile-proof.c")
        guest.key("ret")
        ui.wait_feature(guest, lambda p: p[11] and p[12] == 2)
        guest.source_named = True
    # Explicit commands can apply directly; generated proposals require review.
    if ui.features(guest)[14]:
        ui.panel_click(guest, "apply")
    return wait_build(guest, previous, expected)

# ------------------------=
# FUNC: generate_program
# DESC: Requests actual generation and checks that the editor receives source and compiles it without an Apply click.
# ------------------=
def generate_program(guest):
    assert ui.features(guest)[13]
    ui.panel_click(guest, "composer")
    ui.text(guest, "Write and compile a simple C program with int main(void) that returns 54. Replace the entire document.")
    before = read(guest, "INFINITY_NATIVE_BUILD_SNAPSHOT", 10)[2]
    started = time.monotonic()
    next_progress = started + 20
    prior_length = None
    captures = 0
    guest.key("ret")
    while time.monotonic() - started < 600:
        state = ui.features(guest)
        if state[19] != prior_length and captures < 64:
            prior_length = state[19]
            guest.screenshot(f"native-editor-model-{captures:02d}")
            captures += 1
        if time.monotonic() >= next_progress:
            guest.screenshot("native-editor-generation-progress")
            print(json.dumps({"generation_seconds": round(time.monotonic()-started, 2),
                              "owner": state[12], "pending_action": state[14],
                              "input_bytes": state[18], "response_bytes": state[19]}), flush=True)
            # Exercise normal pointer input while waiting, keeping the active
            # test session alive without changing authentication policy.
            guest.qmp("input-send-event", {"events": [{"type": "rel", "data": {
                "axis": "x", "value": -1 if guest.state()[13] > 500 else 1}}]})
            next_progress = time.monotonic() + 20
        build = read(guest, "INFINITY_NATIVE_BUILD_SNAPSHOT", 10)
        if build[2] > before and build[3] != 0:
            assert state[7] > 0 and state[14] == 0, ("Generated source was not applied to the editor", state)
            (guest.work / "native-generated-proposal.json").write_text(json.dumps({
                "owner": state[12], "action": state[14], "snapshot": state,
                "elapsed_seconds": time.monotonic() - started,
                "applied": True, "compile_started_without_confirmation": True,
            }, indent=2) + "\n")
            guest.screenshot("native-editor-generated-source")
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
    parser.add_argument("--update-kernel", type=pathlib.Path, help="Update only this disposable installed disk; not fresh-install parity evidence")
    parser.add_argument("--compiler-only", action="store_true", help="Diagnose compile, publication and launch without claiming model-generation acceptance")
    parser.add_argument("--save-compile", action="store_true", help="Exercise save-and-compile, Save As cancellation, and subsequent named-source saving")
    parser.add_argument("--generation-only", action="store_true", help="Retest generated-source acceptance without repeating passing manual-source cases")
    parser.add_argument("--trace-native-faults", action="store_true", help="Enable address-only HVF fault tracing at compilation, not during boot or model inference")
    parser.add_argument("work", type=pathlib.Path)
    args = parser.parse_args()
    if args.update_kernel and not args.reuse_installed:
        parser.error("--update-kernel requires --reuse-installed")
    if args.generation_only and args.compiler_only:
        parser.error("Choose one focused case")
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
    guest.trace_native_faults = args.trace_native_faults
    guest.save_compile = args.save_compile
    # Suppress the base harness's unrelated browser patching path.
    guest.patched = True
    guest.boot_timeout_seconds = 600
    report = {"installed_acceptance": False, "architecture": args.arch, "packaged": packaged,
              "source_media": str(source_iso), "canonical_release_media": args.iso is None,
              "acceleration": guest.acceleration}
    if args.update_kernel:
        kernel = args.update_kernel.resolve()
        kernel.relative_to(ROOT)
        subprocess.run([sys.executable, str(ROOT / "tools/update-installed-clone.py"),
                        str(guest.work / "installed.raw"), str(kernel), "--apply"], check=True)
        shutil.copyfile(kernel, artifacts / "installed-kernel.elf")
        with kernel.open("rb") as stream:
            digest = hashlib.file_digest(stream, "sha256").hexdigest()
        report["updated_kernel_sha256"] = digest
        report["canonical_release_media"] = False
    elif previous_report and "updated_kernel_sha256" in previous_report:
        report["updated_kernel_sha256"] = previous_report["updated_kernel_sha256"]
        report["canonical_release_media"] = False
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
        if not args.generation_only:
            report["compile"] = compile_document(guest, 'int main(void) { return 54; }', 3)
        published = read(guest, "INFINITY_NATIVE_APP_SNAPSHOT", 8)
        if not args.generation_only:
            assert published[3] == 1 and any(published[4:6])
        address, size = base.symbol(artifacts / "installed-kernel.elf", "INFINITY_NATIVE_APP_NAME")
        assert size == 28
        name = guest.memory(address, size).decode("ascii")
        if not args.generation_only:
            guest.screenshot("native-editor-compiled")
            report["invalid"] = compile_document(guest, 'int main(void) { this is not C; }', 4)
            assert read(guest, "INFINITY_NATIVE_APP_SNAPSHOT", 8)[2] == published[2]
        elif not ui.features(guest)[13]:
            ui.panel_click(guest, "toggle")
            ui.wait_feature(guest, lambda p: p[13] and p[15])
        if not args.compiler_only:
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
                   for result in [report[key] for key in ("compile", "invalid", "generated") if key in report]), "Compiler input latency exceeded five seconds"
        report["installed_acceptance"] = not args.compiler_only and not args.generation_only
        report["model_generated_source"] = not args.compiler_only
        report["compiler_workflow_passed"] = True
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
                try:
                    guest.process.wait(timeout=20)
                except subprocess.TimeoutExpired:
                    guest.process.kill()
                    guest.process.wait(timeout=20)
        evidence = ROOT / "builds/validation/native-apps" / (args.arch + "-installed")
        evidence.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(work / "evidence.json", evidence / "evidence.json")
        for screenshot in guest.work.glob("*.ppm"):
            shutil.copyfile(screenshot, evidence / screenshot.name)
    print(json.dumps(report))

if __name__ == "__main__":
    main()
