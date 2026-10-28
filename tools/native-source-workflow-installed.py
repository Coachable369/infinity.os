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
def request(guest, harness, text, language, launch, timeout, directory, prior_source=None, python_live=False):
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
                        assert native[2] > python_before, native
                        if python_live:
                            assert launch and native[3] == 1, native
                        else:
                            assert native[3:5] == (2, 0), native
                            assert not launch or native[5] > 0, native
                        assert harness.read(guest, "INFINITY_NATIVE_BUILD_SNAPSHOT", 10)[2] == build_before
                    else:
                        native = harness.read(guest, "INFINITY_NATIVE_BUILD_SNAPSHOT", 10)
                        assert native[2] > build_before and native[3] == 3, native
                    result = {**receipt, "request": text, "editor_fingerprint": editor[16], "python_live": python_live,
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
# FUNC: dirty_update_rejection
# DESC: Leaves an unrelated unsaved editor document and proves a named update cannot overwrite it or start generation, compilation, or execution.
# ------------------=
def dirty_update_rejection(guest, harness):
    new_saved_buffer(guest, harness)
    editor = harness.features(guest)
    harness.ui.click(guest, editor[30] + 24, editor[29] + 12)
    guest.key("x")
    dirty = harness.ui.wait_feature(guest, lambda row: row[7] == 1 and not row[11])
    previous = state(guest, harness)
    compiler = harness.read(guest, "INFINITY_NATIVE_BUILD_SNAPSHOT", 10)
    python = harness.read(guest, "INFINITY_PYTHON_SNAPSHOT", 10)
    catalog = harness.read(guest, "INFINITY_GENERATED_APPS_SNAPSHOT", 16)
    guest.key("ctrl", "j")
    harness.ui.text(guest, "Update the calculator app and add numeric buttons")
    guest.key("ret")
    deadline = time.monotonic() + 30
    while True:
        receipt = state(guest, harness)
        after = harness.features(guest)
        assert after[7] == dirty[7] and after[16] == dirty[16] and not after[11], after
        assert harness.read(guest, "INFINITY_NATIVE_BUILD_SNAPSHOT", 10)[2] == compiler[2]
        assert harness.read(guest, "INFINITY_PYTHON_SNAPSHOT", 10)[2] == python[2]
        assert harness.read(guest, "INFINITY_GENERATED_APPS_SNAPSHOT", 16)[12:14] == catalog[12:14]
        if receipt["goal"] > previous["goal"] and receipt["plan_state"] in (6, 7) and not receipt["active_goal"]:
            break
        assert time.monotonic() < deadline, {"dirty_update_not_rejected": receipt}
        time.sleep(.1)
    guest.launch("text", 5)
    editor = harness.features(guest)
    harness.ui.click(guest, editor[30] + 24, editor[29] + 12)
    guest.key("ctrl", "z")
    harness.ui.wait_feature(guest, lambda row: row[7] == 0 and row[11])
    return {"rejection": receipt, "preserved_bytes": dirty[7], "preserved_fingerprint": dirty[16],
            "compiler_unchanged": True, "python_unchanged": True, "catalog_launches_unchanged": True}


# ------------------------=
# FUNC: atomic_update_history
# DESC: Uses real editor Undo and Redo to verify the complete AI source update is one atomic transaction with an unchanged durable checkpoint.
# ------------------=
def atomic_update_history(guest, harness, before, after):
    guest.launch("text", 5)
    editor = harness.features(guest)
    harness.ui.click(guest, editor[30] + 24, editor[29] + 12)
    compiler = harness.read(guest, "INFINITY_NATIVE_BUILD_SNAPSHOT", 10)[2]
    guest.key("ctrl", "z")
    undone = harness.ui.wait_feature(guest, lambda row: row[7] == before["document_bytes"]
                                    and row[16] == before["editor_fingerprint"] and not row[11])
    guest.key("ctrl", "y")
    redone = harness.ui.wait_feature(guest, lambda row: row[7] == after["document_bytes"]
                                    and row[16] == after["editor_fingerprint"] and row[11])
    receipt = state(guest, harness)
    assert receipt["bound_source"] == after["source"] and receipt["version"] == after["version"]
    assert harness.read(guest, "INFINITY_NATIVE_BUILD_SNAPSHOT", 10)[2] == compiler
    return {"undo_bytes": undone[7], "undo_fingerprint": undone[16], "redo_bytes": redone[7],
            "redo_fingerprint": redone[16], "persisted_version": receipt["version"]}


# ------------------------=
# FUNC: current_unsaved_source
# DESC: Inserts an unrelated user-owned comment through real editor input and retains its exact bytes as an immutable-artifact preservation expectation.
# ------------------=
def current_unsaved_source(guest, harness, stage):
    guest.launch("text", 5)
    editor = harness.features(guest)
    harness.ui.click(guest, editor[30] + 24, editor[29] + 12)
    guest.key("ctrl", "g")
    harness.ui.text(guest, "1")
    guest.key("ret")
    marker = ("/* local-note: " + format(time.time_ns(), "016x") + " */\n").encode("ascii")
    harness.ui.text(guest, marker.decode("ascii"))
    dirty = harness.ui.wait_feature(guest, lambda row: row[7] == stage["document_bytes"] + len(marker) and not row[11])
    assert state(guest, harness)["version"] == stage["version"]
    return {**stage, "editor_fingerprint": dirty[16], "document_bytes": dirty[7],
            "saved": False, "protected_prefix_hex": marker.hex()}


# ------------------------=
# FUNC: stop_running_application
# DESC: Uses the existing native console interrupt, waits for resource reclamation, and never edits process memory or synthesizes lifecycle receipts.
# ------------------=
def stop_running_application(guest, harness, language):
    symbol, count, active, stopped = (("INFINITY_PYTHON_SNAPSHOT", 10, (1,), 3) if language
                                     else ("INFINITY_NATIVE_APP_SNAPSHOT", 8, (2,), 4))
    before = harness.read(guest, symbol, count)
    if before[3] not in active:
        return
    guest.launch("command", 5)
    if harness.features(guest)[13]:
        harness.ui.panel_click(guest, "toggle")
    guest.key("esc")
    guest.key("ctrl", "c")
    deadline = time.monotonic() + 30
    while True:
        after = harness.read(guest, symbol, count)
        if after[3] == stopped:
            assert not language or after[7] == after[8], after
            return
        assert time.monotonic() < deadline, after
        time.sleep(.1)


# ------------------------=
# FUNC: python_counter_actions
# DESC: Verifies a genuine generated Python button and an explicitly authorized real LLM hook both change the same numeric application state.
# ------------------=
def python_counter_actions(guest, harness):
    guest.launch("command", 5)
    contract = harness.interaction_state(guest)
    assert contract["instance"] and contract["choice_count"] > 0 and contract["commits"] > 0, contract
    ui = harness.native_ui_state(guest)
    before = calculator_result(ui)
    buttons = [element for element in ui["elements"] if element["kind"] == 3]
    assert before is not None and len(buttons) == 1, ui
    harness.click_native_element(guest, buttons[0]["id"])
    deadline = time.monotonic() + 10
    while calculator_result(harness.native_ui_state(guest)) != before + 1:
        assert time.monotonic() < deadline, harness.native_ui_state(guest)
        time.sleep(.1)
    contract = harness.interaction_state(guest)
    assert contract["applied"] == 0 and contract["delivered"] == 0, contract
    if not harness.features(guest)[13]:
        harness.ui.panel_click(guest, "toggle")
    harness.ui.panel_click(guest, "composer")
    harness.ui.text(guest, "Add one to the counter once using its increment action")
    guest.key("ret")
    deadline = time.monotonic() + 120
    due = time.monotonic() + 20
    while True:
        due = attended_activity(guest, time.monotonic(), due)
        after = harness.interaction_state(guest)
        assert after["instance"] == contract["instance"] and after["rejected"] == contract["rejected"], after
        if after["applied"] > contract["applied"]:
            assert after["applied"] == contract["applied"] + 1
            assert after["delivered"] == contract["delivered"] + 1 and after["last_result"] == 1
            assert after["revision"] > contract["revision"] and after["last_token"] > 0
            assert after["last_choice"] in {choice["id"] for choice in contract["choices"]}
            assert calculator_result(harness.native_ui_state(guest)) == before + 2
            break
        assert time.monotonic() < deadline, {"native_hook_not_applied": after}
        time.sleep(.2)
    harness.ui.panel_click(guest, "toggle")
    guest.screenshot("generated-python-pointer-and-ai-hook")
    return {"before": contract, "after": after, "initial_value": str(before), "final_value": str(before + 2),
            "input_transport": "typed app assistant; real local inference; no microphone claim"}


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
# FUNC: catalog_launch
# DESC: Launches a genuinely generated application by its human-readable name and correlates the owned source with actual native execution, without regenerating code.
# ------------------=
def catalog_launch(guest, harness, stage, cold_boot=False):
    before = harness.read(guest, "INFINITY_GENERATED_APPS_SNAPSHOT", 16)
    assert before[:2] == (0x494e4647454e4150, 1) and before[3] > 0
    name = pathlib.PurePosixPath(stage["path"]).stem
    stem, separator, suffix = name.rpartition("-")
    if separator and len(suffix) >= 8 and all(c in "0123456789abcdefABCDEF" for c in suffix):
        name = stem
    name = name.replace("_", " ").replace("-", " ").lower()[:48]
    assert name and name.isascii()
    native_symbol, size = ("INFINITY_PYTHON_SNAPSHOT", 10) if stage["language"] else ("INFINITY_NATIVE_APP_SNAPSHOT", 8)
    stop_running_application(guest, harness, 0)
    stop_running_application(guest, harness, 1)
    guest.launch("command", 5)
    native_before = harness.read(guest, native_symbol, size)
    goal_before = state(guest, harness)["goal"]
    editor_before = harness.features(guest)[16]
    guest.key("ctrl", "j")
    harness.ui.text(guest, "launch " + name)
    guest.key("ret")
    deadline = time.monotonic() + 40
    while True:
        catalog = harness.read(guest, "INFINITY_GENERATED_APPS_SNAPSHOT", 16)
        assert catalog[13] == before[13], {"catalog_launch_rejected": name, "receipt": catalog}
        if catalog[12] > before[12]:
            break
        assert time.monotonic() < deadline, {"catalog_launch_timeout": name, "receipt": catalog}
        time.sleep(.1)
    assert struct.pack("<2Q", *catalog[6:8]).hex() == stage["source"]
    assert catalog[8] == stage["language"] and catalog[3] == before[3] and catalog[4] == catalog[3]
    if stage["language"]:
        if stage.get("python_live"):
            while not harness.interaction_state(guest)["instance"]:
                assert time.monotonic() < deadline, harness.interaction_state(guest)
                time.sleep(.1)
            instance = harness.interaction_state(guest)["instance"]
            assert cold_boot or instance != stage["context"]
            wait_native_ui(guest, harness, instance)
            native = harness.read(guest, native_symbol, size)
            assert native[2] > native_before[2] and native[3] == 1, native
            observed = {"native": list(native), "hooks": python_counter_actions(guest, harness)}
        else:
            while True:
                native = harness.read(guest, native_symbol, size)
                if native[2] > native_before[2] and native[3] >= 2:
                    break
                assert time.monotonic() < deadline, native
                time.sleep(.1)
            assert native[3:5] == (2, 0) and native[5] > 0 and native[7] == native[8], native
            observed = {"native": list(native)}
    else:
        while True:
            interaction = harness.interaction_state(guest)
            if interaction["instance"] and (cold_boot or interaction["instance"] != stage["context"]):
                break
            assert time.monotonic() < deadline, interaction
            time.sleep(.1)
        wait_native_ui(guest, harness, interaction["instance"])
        assert harness.read(guest, native_symbol, size)[3] == 2
        observed = {"native": list(harness.read(guest, native_symbol, size)),
                    "interaction": interaction, "arithmetic": calculator_cases(guest, harness)}
    assert state(guest, harness)["goal"] == goal_before
    assert harness.features(guest)[16] == editor_before
    return {"name": name, "source": stage["source"], "catalog": list(catalog), "cold_boot": cold_boot,
            "input_transport": "typed desktop AIKit fast path; no microphone claim", **observed}


# ------------------------=
# FUNC: cold_boot_catalog
# DESC: Boots the same private installation without installer media and launches both generated identities using the persisted per-user catalog.
# ------------------=
def cold_boot_catalog(guest, harness, stages):
    before = harness.read(guest, "INFINITY_GENERATED_APPS_SNAPSHOT", 16)
    stop_running_application(guest, harness, 0)
    stop_running_application(guest, harness, 1)
    guest.stop()
    guest.boot(False)
    guest.authenticate()
    guest.launch("command", 5)
    after = harness.read(guest, "INFINITY_GENERATED_APPS_SNAPSHOT", 16)
    assert after[3] == before[3] and after[4] == after[3] and after[3] >= 2, (before, after)
    launches = [catalog_launch(guest, harness, stage, cold_boot=True) for stage in stages]
    return {"before": list(before), "after": list(after), "launches": launches,
            "iso_attached": False, "authenticated_again": True}


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
            python = request(guest, harness,
                "Create a simple Python UIKit counter app. Show a numeric display initially zero and exactly one increment button. "
                "Each click adds one. Publish the actual counter state and an AI increment action using infinity. "
                "Handle pointer and AI actions through the same code, acknowledge AI results, and remain open until closed.",
                1, True, timeout, directory / "01-python", python_live=True)
            result["stages"].append(python)
            wait_native_ui(guest, harness, python["context"])
            result["python_generated_hooks"] = python_counter_actions(guest, harness)
            result["python_registered_named_launch"] = catalog_launch(guest, harness, python)
            stop_running_application(guest, harness, 1)
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
        result["unrelated_dirty_rejected"] = dirty_update_rejection(guest, harness)
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
        result["atomic_source_update"] = atomic_update_history(guest, harness, created, updated)
        dirty_target = current_unsaved_source(guest, harness, updated)
        result["current_unsaved_target"] = dirty_target
        relaunched = request(guest, harness, "Update the calculator app and add a clear button and launch",
                             0, True, timeout, directory / "04-relaunch", dirty_target)
        relaunched["protected_prefix_hex"] = dirty_target["protected_prefix_hex"]
        result["stages"].append(relaunched)
        result["unsaved_source_atomic_update"] = atomic_update_history(guest, harness, dirty_target, relaunched)
        assert relaunched["context"] != running
        wait_native_ui(guest, harness, relaunched["context"])
        result["arithmetic"] = calculator_cases(guest, harness)
        result["c_registered_named_launch"] = catalog_launch(guest, harness, relaunched)
        if previous_python is None:
            result["cold_boot_catalog"] = cold_boot_catalog(guest, harness, [python, relaunched])
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
    if stage.get("python_live"):
        assert native[3] == 1
        hooks = report["source_workflows"]["python_generated_hooks"]
        assert hooks["before"]["instance"] == hooks["after"]["instance"] == stage["context"]
        assert hooks["after"]["applied"] == hooks["before"]["applied"] + 1
        assert hooks["after"]["delivered"] == hooks["before"]["delivered"] + 1
        assert hooks["after"]["last_result"] == 1
        assert decimal.Decimal(hooks["final_value"]) == decimal.Decimal(hooks["initial_value"]) + 2
    else:
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
# DESC: Correlates each observed source revision with immutable native storage, retaining exact unchanged ranges across real AI edits.
# ------------------=
def verify_exports(receipt, workflows):
    final_versions = {stage["source"]: stage for stage in workflows["stages"]}
    assert final_versions
    directory = pathlib.Path(receipt["directory"])
    matched = []
    histories = {}
    for stage in workflows["stages"]:
        object_id = stage["source"]
        matches = [artifact for artifact in receipt["artifacts"] if artifact["source"]
                   and artifact["object"] == object_id and artifact["version"] == stage["version"]]
        assert len(matches) == 1
        artifact = matches[0]
        assert bytes.fromhex(artifact["path_hex"]).decode("ascii") == stage["path"]
        assert artifact["version"] == stage["version"]
        content = (directory / artifact["file"]).read_bytes()
        if stage.get("protected_prefix_hex"):
            assert content.startswith(bytes.fromhex(stage["protected_prefix_hex"])), "AI update discarded an unrelated unsaved user edit"
        fingerprint = 1469598103934665603
        for byte in content:
            fingerprint = ((fingerprint ^ byte) * 1099511628211) & ((1 << 64) - 1)
        assert len(content) == stage["document_bytes"] and fingerprint == stage["editor_fingerprint"]
        histories.setdefault(object_id, []).append((stage["version"], content))
        if object_id not in matched:
            matched.append(object_id)
    unchanged = []
    for object_id, versions in histories.items():
        for (old_version, old), (new_version, new) in zip(versions, versions[1:]):
            assert new_version > old_version and old != new
            prefix = 0
            while prefix < min(len(old), len(new)) and old[prefix] == new[prefix]:
                prefix += 1
            suffix = 0
            while suffix < min(len(old), len(new)) - prefix and old[-1-suffix] == new[-1-suffix]:
                suffix += 1
            unchanged.append({"source": object_id, "from_version": old_version, "to_version": new_version,
                              "unchanged_prefix_bytes": prefix, "unchanged_suffix_bytes": suffix,
                              "removed_middle_bytes": len(old) - prefix - suffix,
                              "inserted_middle_bytes": len(new) - prefix - suffix})
    return {"verified_source_objects": matched, "verified_versions": sum(map(len, histories.values())),
            "immutable_version_diffs": unchanged}
