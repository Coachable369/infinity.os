"""Real installed-OS Python creation and named C-source update acceptance."""
import decimal
import hashlib
import json
import pathlib
import struct
import time
import zlib


# ------------------------=
# FUNC: state
# DESC: Reads coherent owner-scoped plan receipts and the actual editor namespace, never writing guest memory.
# ------------------=
def state(guest, harness):
    for _ in range(20):
        raw = harness.read(guest, "INFINITY_SOURCE_WORKFLOW_SNAPSHOT", 24)
        assert raw[:2] == (0x494e465352433031, 1)
        assert raw[18] <= 95
        path = harness.diagnostic_bytes(guest, "INFINITY_SOURCE_WORKFLOW_PATH", 95)[:raw[18]]
        after = harness.read(guest, "INFINITY_SOURCE_WORKFLOW_SNAPSHOT", 24)
        if after[3:23] == raw[3:23] and zlib.crc32(path) == raw[17]:
            break
    else:
        raise AssertionError("Source receipt changed during every capture")
    return {"sequence": raw[2], "goal": raw[3], "plan_state": raw[4], "tasks": raw[5],
            "completed_tasks": raw[6], "source": struct.pack("<2Q", *raw[7:9]).hex(),
            "source_revision": raw[9], "artifact": struct.pack("<2Q", *raw[10:12]).hex(),
            "context": raw[12], "bound_source": struct.pack("<2Q", *raw[13:15]).hex(),
            "version": raw[15], "language": raw[16], "path": path.decode("ascii"),
            "request_fingerprint": raw[19], "document_bytes": raw[20],
            "saved": bool(raw[21]), "active_goal": bool(raw[22])}


# ------------------------=
# FUNC: terminal_receipt
# DESC: Requires ordered native save, validation and optional launch evidence rather than generated completion prose.
# ------------------=
def terminal_receipt(receipt, previous_goal, language, launch):
    return receipt["goal"] > previous_goal and receipt["plan_state"] == 5 and not receipt["active_goal"] \
        and receipt["completed_tasks"] == receipt["tasks"] == (2 if launch else 1) \
        and receipt["source"] != "00" * 16 and receipt["artifact"] != "00" * 16 \
        and receipt["source"] == receipt["bound_source"] and receipt["version"] > 0 \
        and receipt["saved"] and receipt["document_bytes"] > 0 and receipt["language"] == language \
        and bool(receipt["context"]) == launch


# ------------------------=
# FUNC: attended_activity
# DESC: Models an attending user's bounded pointer motion on the top-frame margin without clicking, changing security policy or reviving a locked session.
# ------------------=
def attended_activity(guest, now, due):
    if now < due:
        return due
    desktop = guest.state()
    assert desktop[4] not in (9, 10), "Attended workflow lost its authenticated session"
    guest.width, guest.height = desktop[11:13]
    x = guest.width // 2 + (8 if int(now // 20) % 2 else -8)
    guest.click(x, 24, press=False)
    return now + 20


# ------------------------=
# FUNC: request
# DESC: Submits an actual desktop instruction and records bounded model/editor progress until trusted native plan completion.
# ------------------=
def request(guest, harness, text, language, launch, timeout, directory, prior_source=None):
    directory.mkdir()
    before = state(guest, harness)
    build_before = harness.read(guest, "INFINITY_NATIVE_BUILD_SNAPSHOT", 10)[2]
    python_before = harness.read(guest, "INFINITY_PYTHON_SNAPSHOT", 10)[2]
    guest.key("ctrl", "j")
    harness.ui.text(guest, text)
    guest.key("ret")
    started = time.monotonic()
    next_activity = started + 20
    deliveries = []
    reloaded = False
    next_capture = 0
    with (directory / "progress.jsonl").open("w") as log:
        while time.monotonic() - started < timeout:
            next_activity = attended_activity(guest, time.monotonic(), next_activity)
            receipt = state(guest, harness)
            progress = harness.generation_progress(guest)
            editor = harness.features(guest)
            row = {"elapsed_seconds": time.monotonic() - started, "receipt": receipt,
                   "generation": progress, "editor_bytes": editor[7], "editor_fingerprint": editor[16]}
            log.write(json.dumps(row) + "\n")
            log.flush()
            if receipt["goal"] > before["goal"]:
                if prior_source and receipt["bound_source"] == prior_source["source"] \
                        and receipt["version"] == prior_source["version"] \
                        and receipt["path"] == prior_source["path"] \
                        and editor[16] == prior_source["editor_fingerprint"]:
                    reloaded = True
                if progress["editor_bytes"] and (not deliveries or deliveries[-1] != progress["editor_bytes"]):
                    deliveries.append(progress["editor_bytes"])
                assert receipt["plan_state"] not in (4, 6, 7), row
                if terminal_receipt(receipt, before["goal"], language, launch):
                    if prior_source:
                        assert reloaded, {"named_source_not_reloaded": row}
                        assert receipt["source"] == prior_source["source"]
                        assert receipt["path"] == prior_source["path"]
                        assert receipt["version"] > prior_source["version"]
                    assert deliveries, {"source_not_delivered_to_editor": row}
                    assert receipt["path"].endswith(".py" if language else ".c")
                    if language:
                        native = harness.read(guest, "INFINITY_PYTHON_SNAPSHOT", 10)
                        assert native[:2] == (0x494e46505954484e, 1)
                        assert native[2] > python_before and native[3:5] == (2, 0), native
                        assert not launch or native[5] > 0, native
                        assert harness.read(guest, "INFINITY_NATIVE_BUILD_SNAPSHOT", 10)[2] == build_before
                    else:
                        native = harness.read(guest, "INFINITY_NATIVE_BUILD_SNAPSHOT", 10)
                        assert native[2] > build_before and native[3] == 3, native
                    result = {**receipt, "request": text, "editor_fingerprint": editor[16],
                              "native_receipt": list(native), "delivered_lengths": deliveries,
                              "elapsed_seconds": time.monotonic() - started, "named_reload": reloaded}
                    (directory / "result.json").write_text(json.dumps(result, indent=2) + "\n")
                    guest.screenshot(directory.name + "-completed")
                    return result
            if time.monotonic() >= next_capture:
                print(json.dumps({"source_workflow": directory.name, **row}), flush=True)
                guest.screenshot(directory.name + "-current")
                next_capture = time.monotonic() + 15
            assert guest.state()[4] not in (9, 10), "Authenticated workflow session was lost"
            time.sleep(.5)
    raise AssertionError({"workflow_timeout": text, "seconds": timeout, "last": row})


# ------------------------=
# FUNC: new_saved_buffer
# DESC: Clears only a saved document through the editor's real New command so the following update must resolve and reopen its named source.
# ------------------=
def new_saved_buffer(guest, harness):
    guest.launch("text", 5)
    harness.ui.wait_feature(guest, lambda value: value[12] == 2 and value[11])
    guest.key("ctrl", "n")
    harness.ui.wait_feature(guest, lambda value: value[12] == 2 and value[7] == 0 and value[11])
    deadline = time.monotonic() + 10
    while state(guest, harness)["path"]:
        assert time.monotonic() < deadline, "New did not detach the saved source"
        time.sleep(.1)


# ------------------------=
# FUNC: wait_native_ui
# DESC: Waits for real UIKit publication after process launch without confusing admission with completed application initialization.
# ------------------=
def wait_native_ui(guest, harness, context):
    deadline = time.monotonic() + 30
    next_activity = time.monotonic() + 20
    while time.monotonic() < deadline:
        next_activity = attended_activity(guest, time.monotonic(), next_activity)
        actual = harness.interaction_state(guest)["instance"]
        presentation = harness.native_ui_state(guest)
        if actual == context and presentation["active"] and presentation["elements"]:
            return presentation
        time.sleep(.1)
    raise AssertionError({"launched_context_without_ui": context, "instance": actual, "ui": presentation})


# ------------------------=
# FUNC: calculator_result
# DESC: Reads one numeric SDK Display or the reserved app_status Label without treating arbitrary rendered prose as success evidence.
# ------------------=
def calculator_result(snapshot):
    displays = [element for element in snapshot["elements"] if element["kind"] == 2]
    if not displays:
        displays = [element for element in snapshot["elements"]
                    if element["kind"] == 1 and element["id"] == 0x7fffffff]
    if len(displays) != 1:
        return None
    try:
        value = decimal.Decimal(displays[0]["value"].strip())
        return value if value.is_finite() else None
    except decimal.InvalidOperation:
        return None


# ------------------------=
# FUNC: calculator_cases
# DESC: Uses generated controls only as input locators and verifies independent numerical arithmetic results from real pointer interactions.
# ------------------=
def calculator_cases(guest, harness):
    guest.launch("command", 5)
    harness.ui.wait_feature(guest, lambda value: value[12] == 1)
    # These are semantic input selectors, not success/status-copy assertions.
    aliases = {"clear": ("C", "AC", "Clear"), "+": ("+",), "-": ("-", "−"),
               "*": ("*", "×", "x"), "/": ("/", "÷"), "=": ("=",)}
    results = []
    for left, operator, right, expected in [(7, "+", 5, 12), (9, "-", 3, 6), (6, "*", 7, 42), (8, "/", 2, 4)]:
        for token in ["clear", str(left), operator, str(right), "="]:
            snapshot = harness.native_ui_state(guest)
            choices = [element for element in snapshot["elements"] if element["kind"] == 3
                       and element["value"].strip() in aliases.get(token, (token,))]
            assert len(choices) == 1, {"ambiguous_or_missing_calculator_input": token, "ui": snapshot}
            harness.click_native_element(guest, choices[0]["id"])
        deadline = time.monotonic() + 10
        while True:
            snapshot = harness.native_ui_state(guest)
            actual = calculator_result(snapshot)
            if actual == decimal.Decimal(expected):
                results.append({"left": left, "operator": operator, "right": right, "result": expected})
                break
            assert time.monotonic() < deadline, {"incorrect_arithmetic": results, "wanted": expected, "ui": snapshot}
            time.sleep(.1)
    guest.screenshot("updated-calculator-arithmetic")
    return results


# ------------------------=
# FUNC: verify
# DESC: Exercises installed model generation and source updates, optionally retaining separately attributed, byte-verified Python evidence from an earlier guest.
# ------------------=
def verify(guest, harness, timeout, previous_python=None, previous_created=None):
    directory = guest.work.parent / f"source-workflows-{time.time_ns()}"
    directory.mkdir()
    result = {"directory": str(directory), "passed": False, "stages": [],
              "scope": "updates-only" if previous_created else "creation-and-updates"}
    guest.source_workflow_evidence = result
    try:
        if previous_python is None:
            python = request(guest, harness, "Create Hello world in Python", 1, True, timeout, directory / "01-python")
            result["stages"].append(python)
        else:
            result["previous_python_evidence"] = verified_python_evidence(previous_python)
        if previous_created is None:
            created = request(guest, harness, "Create a simple uikit calculator app", 0, True, timeout, directory / "02-calculator")
            result["stages"].append(created)
            running = created["context"]
            original_ui = wait_native_ui(guest, harness, running)["elements"]
        else:
            result["previous_creation_evidence"] = verified_c_source_evidence(previous_created)
            result["creation_ui_verified"] = False
            created = result["previous_creation_evidence"]["stage"]
            running = harness.interaction_state(guest)["instance"]
            original_ui = harness.native_ui_state(guest)["elements"]
        result["pre_update_execution"] = {"context": running,
            "native": list(harness.read(guest, "INFINITY_NATIVE_APP_SNAPSHOT", 8))}
        new_saved_buffer(guest, harness)
        updated = request(guest, harness,
            "Update the calculator app and add numeric buttons and traditional mathematic calculator operators",
            0, False, timeout, directory / "03-named-update", created)
        result["stages"].append(updated)
        assert harness.interaction_state(guest)["instance"] == running
        assert harness.native_ui_state(guest)["elements"] == original_ui
        after = list(harness.read(guest, "INFINITY_NATIVE_APP_SNAPSHOT", 8))
        assert after[3] == 1, {"unexpected_execution_after_save_compile": after}
        result["no_launch_verified"] = {"context": running, "native": after}
        new_saved_buffer(guest, harness)
        relaunched = request(guest, harness, "Update the calculator app and add a clear button and launch",
                             0, True, timeout, directory / "04-relaunch", updated)
        result["stages"].append(relaunched)
        assert relaunched["context"] != running
        wait_native_ui(guest, harness, relaunched["context"])
        result["arithmetic"] = calculator_cases(guest, harness)
        result["passed"] = True
        return result
    finally:
        (directory / "result.json").write_text(json.dumps(result, indent=2) + "\n")


# ------------------------=
# FUNC: verified_python_evidence
# DESC: Carries an independently completed prior native Python stage with exact artifact verification and provenance, never claiming it ran in the current guest.
# ------------------=
def verified_python_evidence(path):
    encoded = path.read_bytes()
    report = json.loads(encoded)
    stages = report["source_workflows"]["stages"]
    assert stages and report["architecture"] == "aarch64"
    stage = stages[0]
    assert terminal_receipt(stage, 0, 1, True)
    native = stage["native_receipt"]
    assert len(native) == 10 and native[:2] == [0x494e46505954484e, 1]
    assert native[3:5] == [2, 0] and native[5] > 0 and native[7] == native[8]
    verify_exports(report["stored_artifacts"], {"stages": [stage]})
    return {"report": str(path), "sha256": hashlib.sha256(encoded).hexdigest(),
            "kernel_sha256": report.get("updated_kernel_sha256"), "stage": stage,
            "executed_in_current_guest": False}


# ------------------------=
# FUNC: verified_c_source_evidence
# DESC: Reuses only a byte-verified, actually generated and compiled saved C source for independent updates, preserving the prior UI failure explicitly.
# ------------------=
def verified_c_source_evidence(path):
    encoded = path.read_bytes()
    report = json.loads(encoded)
    assert report["architecture"] == "aarch64"
    candidates = [stage for stage in report["source_workflows"]["stages"] if stage["language"] == 0]
    assert len(candidates) == 1
    stage = candidates[0]
    assert terminal_receipt(stage, 0, 0, True) and stage["path"].endswith(".c")
    native = stage["native_receipt"]
    assert len(native) == 10 and native[:2] == [0x494e46434c414e47, 1] and native[3] == 3
    verify_exports(report["stored_artifacts"], {"stages": [stage]})
    return {"report": str(path), "sha256": hashlib.sha256(encoded).hexdigest(), "stage": stage,
            "prior_failure": report.get("failure"), "created_in_current_guest": False,
            "creation_ui_verified": False}


# ------------------------=
# FUNC: verify_exports
# DESC: Correlates the final saved versions with immutable ObjectStore extraction and the live editor's numeric byte fingerprint.
# ------------------=
def verify_exports(receipt, workflows):
    final_versions = {stage["source"]: stage for stage in workflows["stages"]}
    assert final_versions
    directory = pathlib.Path(receipt["directory"])
    matched = []
    for object_id, stage in final_versions.items():
        matches = [artifact for artifact in receipt["artifacts"] if artifact["source"] and artifact["object"] == object_id]
        assert len(matches) == 1
        artifact = matches[0]
        assert bytes.fromhex(artifact["path_hex"]).decode("ascii") == stage["path"]
        assert artifact["version"] == stage["version"]
        content = (directory / artifact["file"]).read_bytes()
        fingerprint = 1469598103934665603
        for byte in content:
            fingerprint = ((fingerprint ^ byte) * 1099511628211) & ((1 << 64) - 1)
        assert len(content) == stage["document_bytes"] and fingerprint == stage["editor_fingerprint"]
        matched.append(object_id)
    return {"verified_source_objects": matched}
