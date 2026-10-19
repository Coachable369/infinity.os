"""Native editor compilation acceptance through real input on a disposable installed OS."""
import importlib.util
import argparse
import array
import functools
import hashlib
import json
import os
import pathlib
import re
import shutil
import struct
import subprocess
import sys
import time
import zlib

ROOT = pathlib.Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("editor_ui", ROOT / "tools/editor-assistant-installed-test.py")
ui = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ui)
base = ui.base
guest_spec = importlib.util.spec_from_file_location("desktop_guest", ROOT / "tools/servo-platform-probe/run-desktop-installed.py")
desktop = importlib.util.module_from_spec(guest_spec)
guest_spec.loader.exec_module(desktop)
case_spec = importlib.util.spec_from_file_location("native_application_case", ROOT / "tools/native-application-case.py")
application_case = importlib.util.module_from_spec(case_spec)
case_spec.loader.exec_module(application_case)

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
        if symbol == "INFINITY_NATIVE_APP_SNAPSHOT" or state[2] == state[-1] and state[2] % 2 == 0:
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
# FUNC: assert_workflow
# DESC: Verifies command output, publication, launch, running-process, and ordered voice milestones exposed by the installed OS.
# ------------------=
def assert_workflow(guest):
    workflow = read(guest, "INFINITY_NATIVE_WORKFLOW_SNAPSHOT", 9)
    assert workflow[:2] == (0x494e4643574f524b, 1)
    assert workflow[2] == workflow[8] and workflow[2] % 2 == 0
    assert workflow[3] == 5 and workflow[4] > 4 and workflow[6] > 0
    expected = sum(1 << stage for stage in (1, 2, 3, 4, 5))
    assert workflow[7] & expected == expected
    narration = read(guest, "INFINITY_NATIVE_WORKFLOW_VOICE_SNAPSHOT", 8)
    assert narration[:2] == (0x494e46574f494345, 1)
    assert narration[2] == narration[7] and narration[2] % 2 == 0
    expected_narration = sum(1 << stage for stage in range(5))
    assert narration[3] & expected_narration == expected_narration

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
# FUNC: create_application
# DESC: Uses real desktop chat and keyboard input to verify automatic application creation and arithmetic-derived exit status.
# ------------------=
def create_application(guest):
    before = read(guest, "INFINITY_NATIVE_BUILD_SNAPSHOT", 10)[2]
    guest.key("ctrl", "j")
    ui.text(guest, "Create an integer calculator supporting addition and multiplication reading expressions as a op b and quit to exit with the last calculated result as its exit code application")
    guest.key("ret")
    ui.wait_feature(guest, lambda p: p[12] == 2)
    deadline = time.monotonic() + 600
    next_progress = 0
    previous_response = None
    captures = 0
    while time.monotonic() < deadline:
        state = ui.features(guest)
        if state[19] != previous_response and captures < 64:
            previous_response = state[19]
            guest.screenshot(f"create-model-{captures:02d}")
            captures += 1
        if time.monotonic() >= next_progress:
            guest.screenshot("create-current")
            print(json.dumps({"creation_state": state}), flush=True)
            guest.qmp("input-send-event", {"events": [{"type": "rel", "data": {
                "axis": "x", "value": -1 if guest.state()[13] > 500 else 1}}]})
            next_progress = time.monotonic() + 20
        assert guest.state()[4] not in (9, 10), "Authenticated creation session was lost"
        build = read(guest, "INFINITY_NATIVE_BUILD_SNAPSHOT", 10)
        if build[2] > before and build[3]:
            features = ui.features(guest)
            assert features[7] > 0 and features[11] == 1
            break
        time.sleep(.5)
    else:
        raise AssertionError("Creation did not reach native compilation")
    compiled = wait_build(guest, before, 3)
    ui.wait_feature(guest, lambda p: p[12] == 1)
    assert read(guest, "INFINITY_NATIVE_APP_SNAPSHOT", 8)[3] == 2
    assert_workflow(guest)
    guest.screenshot("generated-calculator-running")
    for expression in ("2 + 3", "9 * 6", "quit"):
        ui.text(guest, expression)
        guest.key("ret")
        # Wait for actual input consumption, not an arbitrary inter-command sleep.
        end = time.monotonic() + 30
        while time.monotonic() < end:
            state = read(guest, "INFINITY_NATIVE_APP_SNAPSHOT", 8)
            if state[3] != 2 or state[6] == 1:
                break
            time.sleep(.1)
        else:
            raise AssertionError("Application did not consume terminal input")
    end = time.monotonic() + 30
    while time.monotonic() < end:
        state = read(guest, "INFINITY_NATIVE_APP_SNAPSHOT", 8)
        if state[3] >= 3:
            assert state[3] == 3 and state[6] == 54, state
            guest.screenshot("generated-calculator-complete")
            return {"compile": compiled, "application": state, "arithmetic_exit": 54}
        time.sleep(.1)
    raise AssertionError("Generated calculator did not exit")

# ------------------------=
# FUNC: create_ui_application
# DESC: Proves that a desktop request generates, compiles, launches and presents a semantic Infinity UI Kit application.
# ------------------=
def create_ui_application(guest):
    before = read(guest, "INFINITY_NATIVE_BUILD_SNAPSHOT", 10)[2]
    guest.key("ctrl", "j")
    ui.text(guest, "Build a simple UI C based calculator application")
    guest.key("ret")
    ui.wait_feature(guest, lambda state: state[12] == 2)
    deadline = time.monotonic() + 600
    next_progress = 0
    streamed_lengths = []
    activity_direction = 1
    while time.monotonic() < deadline:
        length = features(guest)[7]
        if length and (not streamed_lengths or streamed_lengths[-1] != length):
            streamed_lengths.append(length)
        build = read(guest, "INFINITY_NATIVE_BUILD_SNAPSHOT", 10)
        if build[2] > before and build[3]:
            break
        if time.monotonic() >= next_progress:
            guest.screenshot("create-ui-current")
            # Model a present user moving the pointer, without disabling the
            # real inactivity lock or changing the document being generated.
            guest.qmp("input-send-event", {"events": [{"type": "rel", "data": {
                "axis": "x", "value": activity_direction}}]})
            activity_direction = -activity_direction
            next_progress = time.monotonic() + 20
        assert guest.state()[4] not in (9, 10), "Authenticated UI creation session was lost"
        time.sleep(.5)
    else:
        raise AssertionError("UI creation did not reach native compilation")
    compiled = wait_build(guest, before, 3)
    assert len(streamed_lengths) >= 2, {"missing_incremental_editor_output": streamed_lengths}
    ui.wait_feature(guest, lambda state: state[12] == 1)
    deadline = time.monotonic() + 30
    while time.monotonic() < deadline:
        presentation = read(guest, "INFINITY_NATIVE_UI_SNAPSHOT", 11)
        assert presentation[:2] == (0x494e4655494b4954, 1)
        assert presentation[2] == presentation[10] and presentation[2] % 2 == 0
        if presentation[3] == 1 and presentation[4:6] == (480, 420) and presentation[6] >= 10:
            guest.screenshot("generated-ui-calculator-running")
            guest.key("esc")
            return {"compile": compiled, "presentation": presentation,
                    "streamed_editor_lengths": streamed_lengths}
        time.sleep(.1)
    raise AssertionError("Generated application did not present a complete UI Kit calculator")

# ------------------------=
# FUNC: generation_progress
# DESC: Reads the versioned generation snapshot for prompt prefill, model output and actual editor delivery.
# ------------------=
def generation_progress(guest):
    state = read(guest, "INFINITY_APP_GENERATION_SNAPSHOT", 14)
    assert state[:2] == (0x494e4647454e3031, 1)
    return {"sequence": state[2], **dict(zip(("status", "prompt_tokens", "processed_tokens", "generated_tokens",
                     "output_bytes", "source_bytes", "editor_bytes", "editor_flushes",
                     "repairs", "clock_ns"), state[3:13]))}

# ------------------------=
# FUNC: editor_lifecycle_state
# DESC: Reads document integrity, dialog state and the actual source transaction without pausing or writing guest memory.
# ------------------=
def editor_lifecycle_state(guest):
    state = features(guest)
    navigator = read(guest, "INFINITY_NAVIGATOR_DIAGNOSTIC_SNAPSHOT", 64)
    assert navigator[:2] == (0x494e464e41563131, 1)
    return {"document_bytes": state[7], "document_fingerprint": state[16],
            "saved": bool(state[11]), "owner": state[12], "dialog": navigator[9],
            "document_path_crc32": navigator[13], "panel_expanded": bool(state[13]),
            "pending_action": state[14], "composer_bytes": state[18],
            "response_bytes": state[19], "generation": generation_progress(guest)}

# ------------------------=
# FUNC: editor_window_control
# DESC: Clicks the real shared titlebar control using the observed window bounds and display scale.
# ------------------=
def editor_window_control(guest, control):
    state = features(guest)
    assert state[12] == 2 and control in ("minimize", "close")
    x, y, width, _ = state[21:25]
    display = guest.state()
    scale = 2 if display[11] >= 2560 and display[12] >= 1440 else 1
    target_x = x + width - (93 if control == "minimize" else 25) * scale
    target_y = y + 24 * scale
    assert 0 <= target_x < display[11] and 0 <= target_y < display[12]
    ui.click(guest, target_x, target_y)

# ------------------------=
# FUNC: verify_editor_lifecycle
# DESC: Exercises unsaved typing, minimize, Cancel and Discard through real input and checks preserved bytes and cleared assistant state.
# ------------------=
def verify_editor_lifecycle(guest):
    evidence = guest.work.parent / f"editor-lifecycle-{time.time_ns()}"
    evidence.mkdir()
    result = {"passed": False, "generated_application_acceptance": False,
              "model_stop_retry_verified": False, "evidence_directory": str(evidence), "stages": []}
    guest.editor_lifecycle_evidence = result

    # ------------------------=
    # FUNC: record
    # DESC: Persists partial typed lifecycle receipts so a later assertion failure does not erase earlier observations.
    # ------------------=
    def record(stage):
        state = editor_lifecycle_state(guest)
        result["stages"].append({"stage": stage, **state})
        (evidence / "evidence.json").write_text(json.dumps(result, indent=2) + "\n")
        guest.screenshot("editor-lifecycle-" + stage)
        return state

    # A restored plan is not this test's request. Stop it through supported UI;
    # never overwrite an existing unsaved document to make the fixture pass.
    guest.key("ctrl", "j")
    guest.key("esc")
    deadline = time.monotonic() + 30
    while time.monotonic() < deadline:
        generation = generation_progress(guest)
        if generation["status"] not in (1, 2) and generation["prompt_tokens"] == 0:
            break
        time.sleep(.1)
    else:
        raise AssertionError({"restored_request_not_stopped": generation})
    guest.launch("text", 5)
    initial = ui.wait_feature(guest, lambda state: state[12] == 2)
    assert initial[7] == 0 and initial[11], "Lifecycle fixture requires an empty saved editor; existing work was not replaced"
    assert editor_lifecycle_state(guest)["dialog"] == 0
    if initial[13]:
        ui.panel_click(guest, "toggle")
    state = features(guest)
    ui.click(guest, state[30] + min(80, state[27] // 2), state[29] + min(16, state[28] // 2))
    source = "int main(void) { return 7; }\n"
    ui.text(guest, source)
    ui.wait_feature(guest, lambda state: state[7] == len(source) and not state[11])

    # Populate real assistant source bytes through its existing local insertion
    # action. This is not LLM-generated application evidence; it makes the later
    # transaction-clearing assertion non-vacuous without an expensive model run.
    addition = "// retained"
    ui.panel_click(guest, "toggle")
    ui.wait_feature(guest, lambda state: state[13] and state[15])
    ui.text(guest, "insert " + addition)
    guest.key("ret")
    ui.wait_feature(guest, lambda state: state[7] == len(source + addition) and state[14] == 0)
    fingerprint = 1469598103934665603
    for byte in (source + addition).encode("ascii"):
        fingerprint = ((fingerprint ^ byte) * 1099511628211) & ((1 << 64) - 1)
    deadline = time.monotonic() + 30
    while generation_progress(guest)["source_bytes"] != len(addition):
        assert time.monotonic() < deadline, generation_progress(guest)
        time.sleep(.1)
    original = record("unsaved")
    assert original["document_fingerprint"] == fingerprint and not original["saved"]
    assert original["response_bytes"] > 0
    build_sequence = read(guest, "INFINITY_NATIVE_BUILD_SNAPSHOT", 10)[2]
    editor_window_control(guest, "minimize")
    ui.wait_feature(guest, lambda state: state[12] != 2)
    minimized = record("minimized")
    assert minimized["document_fingerprint"] == fingerprint and minimized["dialog"] == 0
    guest.launch("text", 5)
    ui.wait_feature(guest, lambda state: state[12] == 2)
    restored = record("restored")
    assert restored["document_fingerprint"] == fingerprint and not restored["saved"]
    assert restored["generation"]["source_bytes"] == len(addition)
    editor_window_control(guest, "close")
    ui.wait_editor_dialog(guest, 3)
    assert record("close-prompt")["document_fingerprint"] == fingerprint
    guest.key("esc")
    ui.wait_editor_dialog(guest, 0)
    cancelled = record("close-cancelled")
    assert cancelled["owner"] == 2 and cancelled["document_fingerprint"] == fingerprint and not cancelled["saved"]
    assert cancelled["generation"]["source_bytes"] == len(addition)
    editor_window_control(guest, "close")
    ui.wait_editor_dialog(guest, 3)
    # The real three-choice sheet initially focuses Save (2); Left selects
    # Discard (1). No string matching or direct state injection is involved.
    guest.key("left")
    guest.key("ret")
    ui.wait_editor_dialog(guest, 0)
    ui.wait_feature(guest, lambda state: state[12] != 2 and state[7] == 0 and state[11])
    record("discarded")
    guest.launch("text", 5)
    ui.wait_feature(guest, lambda state: state[12] == 2)
    deadline = time.monotonic() + 30
    cleared_fields = ("status", "prompt_tokens", "processed_tokens", "generated_tokens", "output_bytes",
                      "source_bytes", "editor_bytes", "editor_flushes", "repairs")
    while True:
        reopened = editor_lifecycle_state(guest)
        if all(reopened["generation"][key] == 0 for key in cleared_fields):
            break
        assert time.monotonic() < deadline, {"source_transaction_not_cleared": reopened}
        time.sleep(.1)
    assert reopened["document_bytes"] == 0 and reopened["document_fingerprint"] == initial[16] and reopened["saved"]
    assert reopened["document_path_crc32"] == zlib.crc32(b"")
    assert not reopened["panel_expanded"] and reopened["pending_action"] == reopened["composer_bytes"] == reopened["response_bytes"] == 0
    # Observe later event-loop ticks to reject delayed stale source resurrection.
    until = time.monotonic() + 2
    while time.monotonic() < until:
        state = editor_lifecycle_state(guest)
        assert state["document_bytes"] == 0 and all(state["generation"][key] == 0 for key in cleared_fields), state
        time.sleep(.1)
    assert read(guest, "INFINITY_NATIVE_BUILD_SNAPSHOT", 10)[2] == build_sequence
    result.update(passed=True, minimize_preserves=True, close_cancel_preserves=True,
                  discard_reopens_blank=True, source_transaction_cleared=True, no_compile_started=True)
    record("reopened-empty")
    return result

# ------------------------=
# FUNC: voice_runtime_state
# DESC: Reads content-free audio admission diagnostics without inferring readiness from UI copy or modifying guest state.
# ------------------=
def voice_runtime_state(guest):
    state = read(guest, "INFINITY_NATIVE_WORKFLOW_AUDIO_SNAPSHOT", 16)
    assert state[:2] == (0x494e46415544494f, 1)
    values = dict(zip(("conversation", "output", "error", "playback_hz", "capture_available",
                       "input_prepared", "input_failed", "frames", "first_playback_ns",
                       "output_owner_matches", "pending", "inflight"), state[3:15]))
    if values["error"] >= 1 << 63:
        values["error"] -= 1 << 64
    return values

# ------------------------=
# FUNC: diagnostic_bytes
# DESC: Reads a bounded exported data buffer without writing guest memory or injecting application actions.
# ------------------=
def diagnostic_bytes(guest, symbol, expected_size):
    address, size = base.symbol(guest.work.parent / "artifacts/installed-kernel.elf", symbol)
    assert size == expected_size
    return guest.memory(address, size)

# ------------------------=
# FUNC: interaction_state
# DESC: Reads one coherent published application contract and real native syscall receipts as structured fields.
# ------------------=
def interaction_state(guest):
    for _ in range(20):
        header = read(guest, "INFINITY_APP_INTERACTION_SNAPSHOT", 18)
        assert header[:2] == (0x494e4641494e5431, 1)
        assert header[7] <= 2048 and header[8] <= 128 and header[9] <= 16
        state = diagnostic_bytes(guest, "INFINITY_APP_INTERACTION_STATE", 2048)[:header[7]]
        records = diagnostic_bytes(guest, "INFINITY_APP_INTERACTION_CHOICES", 128 * 416)
        if read(guest, "INFINITY_APP_INTERACTION_SNAPSHOT", 18)[2] == header[2]:
            break
    else:
        raise AssertionError("Application contract changed during every diagnostic capture")
    choices = []
    for index in range(header[8]):
        record = records[index * 416:(index + 1) * 416]
        version, identifier, risk, typed = struct.unpack_from("<4I", record)
        assert version == 1 and risk <= 2 and typed <= 1
        choice = {"id": identifier, "risk": risk, "typed": bool(typed),
                  "action": record[16:64].split(b"\0", 1)[0].decode("utf-8"),
                  "arguments": struct.unpack_from("<4q", record, 160)}
        if typed:
            choice["parameters"] = []
            for slot in range(4):
                at = 192 + slot * 48
                choice["parameters"].append({"name": record[at:at+24].split(b"\0", 1)[0].decode("utf-8"),
                    "kind": struct.unpack_from("<I", record, at+24)[0],
                    "minimum": struct.unpack_from("<q", record, at+32)[0],
                    "maximum": struct.unpack_from("<q", record, at+40)[0]})
            choice["text_name"] = record[384:408].split(b"\0", 1)[0].decode("utf-8")
            choice["text_limit"] = struct.unpack_from("<I", record, 408)[0]
        choices.append(choice)
    result = dict(zip(("instance", "revision", "agent_turn", "terminal", "state_bytes",
                       "choice_count", "schema_count", "commits", "delivered", "applied",
                       "rejected", "last_choice", "last_token", "last_result"), header[3:17]))
    result["choices"] = choices
    # Free-form observation prose is diagnostic only. A JSON object may provide
    # typed app-local state, which domain tests must corroborate through real UI.
    result["observation"] = state.decode("utf-8")
    try:
        result["structured_observation"] = json.loads(state)
    except (ValueError, UnicodeDecodeError):
        result["structured_observation"] = None
    return result

# ------------------------=
# FUNC: native_ui_state
# DESC: Reads actual generated native UI IDs, values and rectangles using the renderer's coherent diagnostic snapshot.
# ------------------=
def native_ui_state(guest):
    for _ in range(20):
        header = read(guest, "INFINITY_NATIVE_UI_SNAPSHOT", 11)
        assert header[:2] == (0x494e4655494b4954, 1) and header[6] <= 96
        records = diagnostic_bytes(guest, "INFINITY_NATIVE_UI_ELEMENTS", 96 * 64)
        receipt = struct.unpack("<4Q", diagnostic_bytes(guest, "INFINITY_NATIVE_UI_EVENT_RECEIPT", 32))
        if read(guest, "INFINITY_NATIVE_UI_SNAPSHOT", 11)[2] == header[2]:
            break
    else:
        raise AssertionError("Native UI changed during every diagnostic capture")
    elements = []
    for index in range(header[6]):
        record = records[index*64:(index+1)*64]
        version, identifier, kind, x, y, width, height, length = struct.unpack_from("<8I", record)
        assert version == 1 and kind in (1, 2, 3) and length <= 32
        elements.append({"id": identifier, "kind": kind, "x": x, "y": y,
                         "width": width, "height": height, "value": record[32:32+length].decode("utf-8")})
    return {"sequence": header[2], "active": bool(header[3]), "width": header[4], "height": header[5],
            "revision": header[7], "event_kind": receipt[1], "event_id": receipt[2],
            "queued_events": receipt[0], "consumed_events": receipt[3], "elements": elements}

# ------------------------=
# FUNC: click_native_element
# DESC: Converts an actual generated button's semantic rectangle to desktop pixels and delivers a real user pointer click.
# ------------------=
def click_native_element(guest, identifier):
    state = native_ui_state(guest)
    candidates = [element for element in state["elements"] if element["id"] == identifier and element["kind"] == 3]
    assert len(candidates) == 1, {"missing_native_button": identifier}
    element = candidates[0]
    owner = features(guest)
    assert owner[12] == 1
    x, y, width, height = owner[21:25]
    display = guest.state()
    scale = 2 if display[11] >= 2560 and display[12] >= 1440 else 1
    # Shared DesktopAppWindowGeometry reserves 48 title, 42 toolbar, 12 top
    # content inset, and 16 left/right/bottom units at the current UI scale.
    content = (x + 16*scale, y + 102*scale, width - 32*scale, height - 118*scale)
    target_x = content[0] + (element["x"]*2 + element["width"]) * content[2] // (state["width"]*2)
    target_y = content[1] + (element["y"]*2 + element["height"]) * content[3] // (state["height"]*2)
    adaptive_click(guest, target_x, target_y)
    deadline = time.monotonic() + 10
    while time.monotonic() < deadline:
        after = native_ui_state(guest)
        if after["queued_events"] > state["queued_events"] and after["consumed_events"] > state["consumed_events"] and after["event_kind"] == 1 and after["event_id"] == identifier:
            return after
        time.sleep(.1)
    raise AssertionError({"button_activation_not_delivered": identifier})

# ------------------------=
# FUNC: export_generated_artifacts
# DESC: Extracts the stopped private VM's actual stored C and ELF, correlating them with editor and publication identities.
# ------------------=
def export_generated_artifacts(guest, generation, architecture):
    assert guest.process is None or guest.process.poll() is not None
    destination = guest.work.parent / "generated-application" / f"stored-artifacts-{time.time_ns()}"
    destination.parent.mkdir(exist_ok=True)
    subprocess.run([str(ROOT / "build/tools/installed-object-test"), str(guest.disk),
                    "--export-generated", str(destination)], check=True)
    receipt = json.loads((destination / "objects.json").read_text())
    assert receipt["version"] == 1 and receipt["read_only_native_storage"]
    source_match = None
    executable_match = None
    for artifact in receipt["artifacts"]:
        path = destination / artifact["file"]
        assert path.parent == destination and path.name == artifact["file"]
        content = path.read_bytes()
        assert len(content) == artifact["bytes"] and zlib.crc32(content) == artifact["crc32"]
        artifact["sha256"] = hashlib.sha256(content).hexdigest()
        if generation and artifact["source"]:
            fingerprint = 1469598103934665603
            for value in content:
                fingerprint = ((fingerprint ^ value) * 1099511628211) & ((1 << 64) - 1)
            if len(content) == generation["saved_source_bytes"] and fingerprint == generation["saved_source_fingerprint"]:
                source_match = artifact["file"]
        if generation and not artifact["source"] and artifact["object"] == generation["published_executable_object"]:
            assert content[:6] == b"\x7fELF\x02\x01"
            assert struct.unpack_from("<H", content, 18)[0] == {"aarch64": 183, "x86_64": 62}[architecture]
            executable_match = artifact["file"]
    receipt.update(source_matches_live_editor=source_match, executable_matches_publication=executable_match)
    (destination / "objects.json").write_text(json.dumps(receipt, indent=2) + "\n")
    if generation:
        assert source_match and executable_match, "Stored artifacts differ from the actual editor/publication snapshot"
    return {"directory": str(destination), **receipt}

# ------------------------=
# FUNC: create_requested_application
# DESC: Exercises an arbitrary desktop request and captures generation, native compilation and real presentation without claiming domain correctness.
# ------------------=
def create_requested_application(guest, request, timeout, case_path=None):
    before = read(guest, "INFINITY_NATIVE_BUILD_SNAPSHOT", 10)[2]
    evidence = guest.work.parent / "generated-application"
    evidence.mkdir(exist_ok=True)
    (evidence / "request.txt").write_text(request)
    guest.key("ctrl", "j")
    ui.text(guest, request)
    guest.key("ret")
    ui.wait_feature(guest, lambda state: state[12] == 2)
    started = time.monotonic()
    next_capture = 0
    source_lengths = []
    progress = []
    with (evidence / "progress.jsonl").open("w") as output:
        while time.monotonic() - started < timeout:
            snapshot = generation_progress(guest)
            snapshot["elapsed_seconds"] = time.monotonic() - started
            progress.append(snapshot)
            output.write(json.dumps(snapshot) + "\n")
            output.flush()
            length = features(guest)[7]
            if length and (not source_lengths or source_lengths[-1] != length):
                source_lengths.append(length)
            build = read(guest, "INFINITY_NATIVE_BUILD_SNAPSHOT", 10)
            if build[2] > before and build[3]:
                break
            if time.monotonic() >= next_capture:
                print(json.dumps({"generation": snapshot}), flush=True)
                guest.screenshot("generated-request-current")
                guest.qmp("input-send-event", {"events": [{"type": "rel", "data": {
                    "axis": "x", "value": -1 if guest.state()[13] > 500 else 1}}]})
                next_capture = time.monotonic() + 10
            assert snapshot["status"] not in (4, 5), snapshot
            assert guest.state()[4] not in (9, 10), "Authenticated generation session was lost"
            time.sleep(.5)
        else:
            raise AssertionError({"generation_deadline_seconds": timeout, "last_progress": progress[-1]})
    compiled = wait_build(guest, before, 3)
    ui.wait_feature(guest, lambda state: state[12] == 1)
    deadline = time.monotonic() + 30
    while time.monotonic() < deadline:
        presentation = read(guest, "INFINITY_NATIVE_UI_SNAPSHOT", 11)
        assert presentation[:2] == (0x494e4655494b4954, 1)
        if presentation[3] == 1 and presentation[4] > 0 and presentation[5] > 0 and presentation[6] > 0:
            guest.screenshot("generated-request-running")
            assert len(source_lengths) >= 2, {"missing_incremental_editor_output": source_lengths}
            ui_state = native_ui_state(guest)
            contract = interaction_state(guest)
            saved = features(guest)
            published = read(guest, "INFINITY_NATIVE_APP_SNAPSHOT", 8)
            (evidence / "ui-state.json").write_text(json.dumps(ui_state, indent=2) + "\n")
            (evidence / "interaction-state.json").write_text(json.dumps(contract, indent=2) + "\n")
            result = {"request": request, "compile": compiled, "presentation": presentation,
                    "streamed_editor_lengths": source_lengths, "progress": progress,
                    "ui": ui_state, "interaction": contract,
                    "saved_source_bytes": saved[7], "saved_source_fingerprint": saved[16],
                    "published_executable_object": struct.pack("<2Q", *published[4:6]).hex(),
                    "native_generation_compile_presentation": True, "domain_behavior_verified": False}
            if case_path is not None:
                result["domain_case"] = application_case.run(guest, case_path, evidence / f"domain-case-{time.time_ns()}",
                                                             native_ui_state, interaction_state, click_native_element)
                result["domain_behavior_verified"] = result["domain_case"]["passed"]
            return result
        time.sleep(.1)
    raise AssertionError("Generated native application did not present a UI")

# ------------------------=
# FUNC: cancel_generation_by_edit
# DESC: Delivers an actual editor keystroke and verifies revision-fenced cancellation without saving or compiling partial source.
# ------------------=
def cancel_generation_by_edit(guest):
    before = features(guest)
    assert before[12] == 2
    build = read(guest, "INFINITY_NATIVE_BUILD_SNAPSHOT", 10)
    ui.click(guest, before[30] + min(80, before[27] // 2),
             before[29] + min(16, before[28] // 2))
    guest.key("spc")
    deadline = time.monotonic() + 30
    while time.monotonic() < deadline:
        state = features(guest)
        progress = generation_progress(guest)
        if progress["status"] == 5 and state[16] != before[16]:
            assert read(guest, "INFINITY_NATIVE_BUILD_SNAPSHOT", 10)[2] == build[2]
            return {"generation": progress, "document_bytes": state[7],
                    "document_fingerprint": state[16], "build_sequence_unchanged": True}
        time.sleep(.1)
    raise AssertionError({"edit_did_not_cancel_generation": progress,
                          "document_before": before[16], "document_after": state[16]})

# ------------------------=
# FUNC: wait_cancelled_checkpoint
# DESC: Requires the real cancelled generation's terminal speech receipt without submitting another request or trusting narration prose.
# ------------------=
def wait_cancelled_checkpoint(guest, result, evidence):
    deadline = time.monotonic() + 60
    next_diagnostic = 0
    while time.monotonic() < deadline:
        narration = read(guest, "INFINITY_NATIVE_WORKFLOW_VOICE_SNAPSHOT", 8)
        assert narration[:2] == (0x494e46574f494345, 1)
        progress = generation_progress(guest)
        record = {"scheduled_mask": narration[3], "announced_mask": narration[4],
                  "pending": narration[5], "inflight": narration[6],
                  "generation": progress, "voice_runtime": voice_runtime_state(guest),
                  "audio_capture_bytes": guest.audio_capture.stat().st_size}
        result["terminal_checkpoint"] = record
        (evidence / "evidence.json").write_text(json.dumps(result, indent=2) + "\n")
        assert progress["status"] == 5, {"cancelled_generation_changed": record}
        if narration[4] & (1 << 15):
            result["announced_checkpoints"] = narration[4]
            result["audio_request_end_byte"] = record["audio_capture_bytes"]
            result["audio_terminal_end_byte"] = record["audio_capture_bytes"]
            return record
        if time.monotonic() >= next_diagnostic:
            print(json.dumps({"terminal_checkpoint_wait": record}), flush=True)
            next_diagnostic = time.monotonic() + 10
        time.sleep(.2)
    raise AssertionError({"cancelled_checkpoint_not_completed": record})

# ------------------------=
# FUNC: prepare_streaming_request
# DESC: Cancels a restored goal through the actual desktop controls before entering a new fixture request, preserving unsaved editor contents.
# ------------------=
def prepare_streaming_request(guest, request):
    before = generation_progress(guest)
    guest.key("ctrl", "j")
    guest.key("esc")
    deadline = time.monotonic() + 30
    while time.monotonic() < deadline:
        after = generation_progress(guest)
        if after["status"] not in (1, 2) and after["prompt_tokens"] == 0:
            break
        time.sleep(.2)
    else:
        raise AssertionError({"restored_goal_still_active": after})
    assert features(guest)[7] == 0, "Fresh fixture admission requires an empty editor; unsaved source was preserved"
    guest.key("ctrl", "j")
    # Desktop Ctrl+X clears the current composer through its supported editing
    # path; it does not replace or delete the editor's document.
    guest.key("ctrl", "x")
    ui.text(guest, request)
    guest.screenshot("native-streaming-request-entered")
    guest.key("ret")
    ui.wait_feature(guest, lambda state: state[12] == 2)
    deadline = time.monotonic() + 30
    while time.monotonic() < deadline:
        admitted = generation_progress(guest)
        if admitted["sequence"] > after["sequence"] and admitted["status"] == 2 and admitted["prompt_tokens"] > 0:
            break
        time.sleep(.2)
    else:
        raise AssertionError({"new_request_not_admitted": admitted, "prior_generation": after})
    guest.screenshot("native-streaming-request-admitted")
    return {"previous_generation": before, "quiescent_generation": after,
            "admitted_generation": admitted,
            "request_bytes": len(request.encode()),
            "request_sha256": hashlib.sha256(request.encode()).hexdigest()}

# ------------------------=
# FUNC: verify_generation_streaming
# DESC: Observes real model/editor progress before completion, then proves cancellation releases the inference lease for a new request.
# ------------------=
def verify_generation_streaming(guest, request, timeout):
    evidence = guest.work.parent / f"streaming-{time.time_ns()}"
    evidence.mkdir()
    (evidence / "request.txt").write_text(request)
    result = {"streaming_verified": False, "cancel_and_readmission_verified": False,
              "generated_application_acceptance": False, "audio_capture_playback_verified": False,
              "evidence_directory": str(evidence),
              "audio_scope": ("terminal_only" if getattr(guest, "terminal_audio_only", False)
                              else "sustained_generation_and_terminal" if getattr(guest, "audio_capture", None) else "none")}
    guest.streaming_evidence = result
    result["request_admission"] = prepare_streaming_request(guest, request)
    if getattr(guest, "audio_capture", None):
        result["audio_request_start_byte"] = guest.audio_capture.stat().st_size
    started = time.monotonic()
    next_capture = 0
    deliveries = []
    sustained_audio = bool(getattr(guest, "audio_capture", None)) and not getattr(guest, "terminal_audio_only", False)
    minimum_bytes = 1024 if sustained_audio else 1
    next_source_capture = 256
    with (evidence / "progress.jsonl").open("w") as output:
        while time.monotonic() - started < timeout:
            progress = generation_progress(guest)
            state = features(guest)
            narration = read(guest, "INFINITY_NATIVE_WORKFLOW_VOICE_SNAPSHOT", 8)
            assert narration[:2] == (0x494e46574f494345, 1)
            record = {"elapsed_seconds": time.monotonic() - started, "generation": progress,
                      "editor_bytes": state[7], "editor_fingerprint": state[16],
                      "narration": {"scheduled_mask": narration[3], "announced_mask": narration[4],
                                    "pending": narration[5], "inflight": narration[6]}}
            if getattr(guest, "audio_capture", None):
                record["audio_capture_bytes"] = guest.audio_capture.stat().st_size
                record["voice_runtime"] = voice_runtime_state(guest)
            result["last_observation"] = record
            output.write(json.dumps(record) + "\n")
            output.flush()
            assert progress["editor_bytes"] <= 256 * progress["editor_flushes"], record
            assert progress["status"] not in (3, 4, 5), {"ended_before_streaming_verified": record}
            if (progress["status"] == 2 and progress["generated_tokens"] > 0
                    and progress["editor_bytes"] > 0 and state[7] > 0
                    and (not deliveries or progress["editor_bytes"] > deliveries[-1]["generation"]["editor_bytes"]
                         and state[16] != deliveries[-1]["editor_fingerprint"])):
                deliveries.append(record)
                if len(deliveries) <= 2 or progress["editor_bytes"] >= next_source_capture:
                    guest.screenshot(f"native-streaming-{progress['editor_bytes']:05d}-bytes")
                    next_source_capture = max(next_source_capture, (progress["editor_bytes"] // 256 + 1) * 256)
            if (len(deliveries) >= 2 and progress["editor_bytes"] >= minimum_bytes
                    and (not sustained_audio or narration[4] & (1 << 17))):
                break
            if time.monotonic() >= next_capture:
                print(json.dumps({"streaming_probe": record}), flush=True)
                guest.qmp("input-send-event", {"events": [{"type": "rel", "data": {
                    "axis": "x", "value": -1 if guest.state()[13] > 500 else 1}}]})
                next_capture = time.monotonic() + 20
            assert guest.state()[4] not in (9, 10), "Authenticated streaming session was lost"
            time.sleep(.2)
        else:
            raise AssertionError({"streaming_deadline_seconds": timeout, "last_progress": record})
    result.update(streaming_verified=True, minimum_delivered_bytes=minimum_bytes, deliveries=deliveries,
                  sustained_generation_checkpoint_verified=sustained_audio and bool(narration[4] & (1 << 17)))
    if getattr(guest, "audio_capture", None):
        result["audio_request_end_byte"] = guest.audio_capture.stat().st_size
        result["announced_checkpoints"] = narration[4]
        result["voice_runtime"] = record["voice_runtime"]
    (evidence / "evidence.json").write_text(json.dumps(result, indent=2) + "\n")
    if getattr(guest, "audio_capture", None):
        result["audio_terminal_start_byte"] = guest.audio_capture.stat().st_size
    cancelled = cancel_generation_by_edit(guest)
    result["cancelled"] = cancelled
    (evidence / "evidence.json").write_text(json.dumps(result, indent=2) + "\n")
    guest.screenshot("native-streaming-cancelled")
    if getattr(guest, "audio_capture", None):
        wait_cancelled_checkpoint(guest, result, evidence)
    # Use the same attached application service, not a fast-path OS action, so
    # Running plus processed prompt tokens proves the shared model was admitted.
    ui.panel_click(guest, "composer")
    ui.wait_feature(guest, lambda state: state[13] and state[15])
    # A second app would correctly be refused while cancellation leaves unsaved
    # source. Ordinary inference exercises admission without replacing that work.
    followup = "Explain how a counter variable changes when incremented"
    ui.text(guest, followup)
    ui.wait_feature(guest, lambda state: state[18] == len(followup))
    guest.key("ret")
    deadline = time.monotonic() + 60
    while time.monotonic() < deadline:
        resumed = generation_progress(guest)
        result["readmission_latest"] = resumed
        assert resumed["status"] != 4, {"fresh_generation_failed": resumed}
        if resumed["status"] == 2 and resumed["prompt_tokens"] > 0 and resumed["processed_tokens"] > 0:
            break
        time.sleep(.2)
    else:
        (evidence / "evidence.json").write_text(json.dumps(result, indent=2) + "\n")
        raise AssertionError({"fresh_generation_not_admitted": resumed})
    assert features(guest)[16] == cancelled["document_fingerprint"], "Ordinary inference changed unsaved source"
    guest.screenshot("native-streaming-readmitted")
    followup_cancelled = cancel_generation_by_edit(guest)
    result.update(cancel_and_readmission_verified=True, readmitted=resumed,
                  followup_cancelled=followup_cancelled)
    (evidence / "evidence.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps({"streaming_probe_passed": True, "evidence": str(evidence)}), flush=True)
    return result

# ------------------------=
# FUNC: verify_checkpoint_audio
# DESC: Checks stopped QEMU HDA PCM emitted during the actual generation interval, independently of narration prose.
# ------------------=
def verify_checkpoint_audio(guest, streaming):
    raw = guest.audio_capture.read_bytes()
    assert raw[:4] == b"RIFF" and raw[8:16] == b"WAVEfmt " and raw[36:40] == b"data"
    assert struct.unpack_from("<HHIIHH", raw, 20) == (1, 2, 48000, 192000, 4, 16)
    start = max(44, streaming["audio_request_start_byte"])
    end = min(len(raw), streaming["audio_request_end_byte"])
    start += (44 - start) % 4
    end -= (end - 44) % 4
    samples = array.array("h", raw[start:end])
    if sys.byteorder != "little":
        samples.byteswap()
    nonzero = sum(abs(sample) > 100 for sample in samples)
    assert nonzero > 4800
    if streaming["audio_scope"] != "terminal_only":
        assert streaming["announced_checkpoints"] & (1 << 17)
    terminal_nonzero = None
    if streaming.get("audio_terminal_end_byte") is not None:
        terminal_start = max(44, streaming["audio_terminal_start_byte"])
        terminal_end = min(len(raw), streaming["audio_terminal_end_byte"])
        terminal_start += (44 - terminal_start) % 4
        terminal_end -= (terminal_end - 44) % 4
        terminal_samples = array.array("h", raw[terminal_start:terminal_end])
        if sys.byteorder != "little":
            terminal_samples.byteswap()
        terminal_nonzero = sum(abs(sample) > 100 for sample in terminal_samples)
        assert streaming["announced_checkpoints"] & (1 << 15) and terminal_nonzero > 4800
    return {"path": str(guest.audio_capture), "sha256": hashlib.sha256(raw).hexdigest(),
            "sample_rate": 48000, "channels": 2, "observed_interval_bytes": end-start,
            "non_silent_samples": nonzero, "terminal_non_silent_samples": terminal_nonzero,
            "announced_mask": streaming["announced_checkpoints"],
            "audio_scope": streaming["audio_scope"],
            "checkpoint_pcm_verified": True, "microphone_verified": False}

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
    parser.add_argument("--boot-only", action="store_true", help="Verify detached installed boot, account setup and cold-boot identity only; retain the stopped private disk without application generation")
    parser.add_argument("--editor-lifecycle-only", action="store_true", help="Verify actual unsaved editor minimize, close Cancel, and close Discard/reopen behavior without model generation or compilation")
    parser.add_argument("--streaming-only", action="store_true", help="Verify real incremental model/editor output and cancellation/readmission only; not generated application acceptance")
    parser.add_argument("--audio-checkpoints", action="store_true", help="Add actual emulated HDA/WAV and require checkpoint completion plus non-silent PCM during --streaming-only")
    parser.add_argument("--terminal-audio-only", action="store_true", help="Isolate spoken cancellation after two real source deliveries; does not claim the sustained-generation checkpoint")
    parser.add_argument("--compiler-only", action="store_true", help="Diagnose compile, publication and launch without claiming model-generation acceptance")
    parser.add_argument("--save-compile", action="store_true", help="Exercise save-and-compile, Save As cancellation, and subsequent named-source saving")
    parser.add_argument("--generation-only", action="store_true", help="Retest generated-source acceptance without repeating passing manual-source cases")
    parser.add_argument("--create-application", action="store_true", help="Exercise desktop intent through generated, saved, compiled and automatically launched interactive C")
    parser.add_argument("--create-ui-application", action="store_true", help="Exercise AI-generated native C UI Kit compilation and presentation")
    parser.add_argument("--generate-request", help="Generate arbitrary UI application through the real desktop; domain behavior requires separate verification")
    parser.add_argument("--generation-timeout", type=int, default=600, help="Bound arbitrary application generation in seconds (1..3600)")
    parser.add_argument("--application-case", type=pathlib.Path, help="Repository-local bounded JSON real-input/typed-domain assertions for --generate-request")
    parser.add_argument("--trace-native-faults", action="store_true", help="Enable address-only HVF fault tracing at compilation, not during boot or model inference")
    parser.add_argument("work", type=pathlib.Path)
    args = parser.parse_args()
    if args.update_kernel and not args.reuse_installed:
        parser.error("--update-kernel requires --reuse-installed")
    if (args.audio_checkpoints or args.terminal_audio_only) and not args.streaming_only:
        parser.error("Audio checkpoint modes require --streaming-only")
    if args.audio_checkpoints and args.terminal_audio_only:
        parser.error("Choose sustained audio checkpoints or terminal-only audio, not both")
    if args.generation_only and args.compiler_only:
        parser.error("Choose one focused case")
    if args.editor_lifecycle_only and (args.boot_only or args.streaming_only or args.generate_request
                                      or args.create_application or args.create_ui_application or args.compiler_only
                                      or args.generation_only or args.save_compile):
        parser.error("--editor-lifecycle-only cannot be combined with another focused case")
    if args.boot_only and (args.streaming_only or args.generate_request or args.create_application or args.create_ui_application
                          or args.compiler_only or args.generation_only or args.save_compile):
        parser.error("--boot-only cannot be combined with an application workflow")
    if not 1 <= args.generation_timeout <= 3600:
        parser.error("--generation-timeout must be within 1..3600")
    if args.generate_request and (args.create_application or args.create_ui_application or args.compiler_only or args.generation_only):
        parser.error("--generate-request cannot be combined with another focused case")
    if args.application_case and not args.generate_request:
        parser.error("--application-case requires --generate-request")
    if args.streaming_only and (args.application_case or args.create_application or args.create_ui_application
                               or args.compiler_only or args.generation_only or args.save_compile):
        parser.error("--streaming-only cannot be combined with other application cases")
    if args.application_case:
        args.application_case = args.application_case.resolve()
        args.application_case.relative_to(ROOT)
    if args.arch == "aarch64" and args.iso is None:
        parser.error("ARM QEMU requires explicit --iso media linked for its RAM layout; the published ARM ISO targets VirtualBox")
    if args.generate_request or args.streaming_only or args.editor_lifecycle_only:
        if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
            parser.error("Generated application acceptance must run through ./build-kit run")
    if args.generate_request and not args.streaming_only:
        (ROOT / "build/tools").mkdir(parents=True, exist_ok=True)
        subprocess.run(["rustc", "--edition=2021", "-A", "warnings", str(ROOT / "tools/installed-object-test.rs"),
                        "-o", str(ROOT / "build/tools/installed-object-test")], check=True)
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
    guest.terminal_audio_only = args.terminal_audio_only
    if args.audio_checkpoints or args.terminal_audio_only:
        guest.audio_capture = guest.work / f"workflow-checkpoints-{time.time_ns()}.wav"
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
        if args.boot_only:
            state = guest.state()
            assert state[3] == 1 and state[4] == 5 and state[9] & 3 == 3
            assert report["identity"]["cold_boot_identity"]
            report["installed_boot_verified"] = True
            report["generated_workflow_passed"] = False
            report["domain_behavior_verified"] = False
            report["desktop_state"] = list(state)
            guest.screenshot("native-baseline-desktop")
            return
        if args.editor_lifecycle_only:
            report["editor_lifecycle"] = verify_editor_lifecycle(guest)
            report["installed_boot_verified"] = True
            report["generated_workflow_passed"] = False
            report["domain_behavior_verified"] = False
            return
        if args.generate_request:
            if args.streaming_only:
                report["streaming"] = verify_generation_streaming(guest, args.generate_request, args.generation_timeout)
                report["installed_boot_verified"] = True
                return
            report["generated_application"] = create_requested_application(guest, args.generate_request, args.generation_timeout, args.application_case)
            report["generated_workflow_passed"] = True
            report["domain_behavior_verified"] = report["generated_application"]["domain_behavior_verified"]
            report["installed_acceptance"] = report["domain_behavior_verified"]
            return
        if args.streaming_only:
            report["streaming"] = verify_generation_streaming(guest, "Create a simple graphical calculator app", args.generation_timeout)
            report["installed_boot_verified"] = True
            return
        if args.create_application:
            report["creation"] = create_application(guest)
            report["installed_acceptance"] = True
            return
        if args.create_ui_application:
            report["ui_creation"] = create_ui_application(guest)
            report["installed_acceptance"] = True
            return
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
                assert_workflow(guest)
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
        if getattr(guest, "streaming_evidence", None):
            report["streaming"] = guest.streaming_evidence
        if getattr(guest, "editor_lifecycle_evidence", None):
            report["editor_lifecycle"] = guest.editor_lifecycle_evidence
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
        export_failed = False
        if (args.audio_checkpoints or args.terminal_audio_only) and report.get("streaming", {}).get("announced_checkpoints"):
            try:
                report["checkpoint_audio"] = verify_checkpoint_audio(guest, report["streaming"])
                report["streaming"]["audio_capture_playback_verified"] = True
            except (AssertionError, OSError) as error:
                report["checkpoint_audio_failure"] = repr(error)
                export_failed = True
        if args.generate_request and not args.streaming_only:
            try:
                report["stored_artifacts"] = export_generated_artifacts(guest, report.get("generated_application"), args.arch)
            except (AssertionError, OSError, subprocess.CalledProcessError) as error:
                report["artifact_export_failure"] = repr(error)
                export_failed = report.get("generated_workflow_passed", False)
                report["installed_acceptance"] = False
        (work / "evidence.json").write_text(json.dumps(report, indent=2) + "\n")
        evidence = ROOT / "builds/validation/native-apps" / (args.arch + "-installed")
        evidence.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(work / "evidence.json", evidence / "evidence.json")
        for screenshot in guest.work.glob("*.ppm"):
            shutil.copyfile(screenshot, evidence / screenshot.name)
        if export_failed:
            raise AssertionError("Generated application artifacts could not be verified from installed storage")
    print(json.dumps(report))

if __name__ == "__main__":
    main()
