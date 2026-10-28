"""Behavioral checks for installed-workflow evidence gates, not VM acceptance."""
import copy
import importlib.util
import json
import os
import pathlib
import tempfile

ROOT = pathlib.Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("source_workflows", ROOT / "tools/native-source-workflow-installed.py")
workflow = importlib.util.module_from_spec(spec)
spec.loader.exec_module(workflow)


# ------------------------=
# FUNC: attended_input_boundaries
# DESC: Verifies timed pointer-only attendance begins after the prompt wait, stays on the frame margin and cannot bypass a locked session.
# ------------------=
def attended_input_boundaries():
    class Guest:
        # ------------------------=
        # FUNC: __init__
        # DESC: Captures real-input API arguments without injecting a guest state change.
        # ------------------=
        def __init__(self):
            self.events = []
            self.locked = False

        # ------------------------=
        # FUNC: state
        # DESC: Supplies display dimensions and either an authenticated or locked screen state.
        # ------------------=
        def state(self):
            value = [0] * 16
            value[4] = 9 if self.locked else 5
            value[11:13] = [1024, 768]
            return value

        # ------------------------=
        # FUNC: click
        # DESC: Records whether the shared pointer driver was told to move without button events.
        # ------------------=
        def click(self, x, y, press):
            self.events.append((x, y, press))

    guest = Guest()
    assert workflow.attended_activity(guest, 19, 20) == 20 and not guest.events
    assert workflow.attended_activity(guest, 20, 20) == 40
    assert guest.events == [(520, 24, False)]
    guest.locked = True
    try:
        workflow.attended_activity(guest, 40, 40)
    except AssertionError:
        pass
    else:
        raise AssertionError("Locked session received unattended input")
    assert len(guest.events) == 1


# ------------------------=
# FUNC: republished_receipts
# DESC: Accepts identical typed data despite fresh publication sequence numbers, but rejects changing receipt contents and mismatched path bytes.
# ------------------=
def republished_receipts():
    class Receipt:
        # ------------------------=
        # FUNC: __init__
        # DESC: Selects stable versus changing payloads and tracks bounded snapshot reads.
        # ------------------=
        def __init__(self, changing=False, corrupt_path=False):
            self.reads = 0
            self.changing = changing
            self.corrupt_path = corrupt_path

        # ------------------------=
        # FUNC: read
        # DESC: Republishes a coherent native receipt with a new even sequence on every read.
        # ------------------=
        def read(self, *args):
            self.reads += 1
            data = [0] * 24
            data[:3] = [0x494e465352433031, 1, self.reads * 2]
            data[3] = self.reads if self.changing else 7
            data[17] = 1 if self.corrupt_path else 0
            data[23] = data[2]
            return tuple(data)

        # ------------------------=
        # FUNC: diagnostic_bytes
        # DESC: Supplies the real empty-path bytes for the receipt consistency check.
        # ------------------=
        def diagnostic_bytes(self, *args):
            return bytes(95)

    stable = Receipt()
    assert workflow.state(None, stable)["goal"] == 7 and stable.reads == 2
    for rejected in [Receipt(changing=True), Receipt(corrupt_path=True)]:
        try:
            workflow.state(None, rejected)
        except AssertionError:
            pass
        else:
            raise AssertionError("Inconsistent receipt capture was accepted")
        assert rejected.reads == 40


# ------------------------=
# FUNC: delayed_native_publication
# DESC: Proves a launch receipt alone or another context's presentation cannot satisfy the real UI readiness gate.
# ------------------=
def delayed_native_publication():
    class Publication:
        # ------------------------=
        # FUNC: __init__
        # DESC: Supplies a finite sequence of observable context and presentation transitions.
        # ------------------=
        def __init__(self):
            self.index = 0

        # ------------------------=
        # FUNC: interaction_state
        # DESC: Models an old context followed by the actual newly launched context.
        # ------------------=
        def interaction_state(self, guest):
            return {"instance": 7 if self.index == 0 else 8}

        # ------------------------=
        # FUNC: native_ui_state
        # DESC: Models the old surface, then uninitialized new surface, then a real new publication.
        # ------------------=
        def native_ui_state(self, guest):
            self.index += 1
            return {"active": self.index != 2, "elements": [self.index]}

    publication = Publication()
    assert workflow.wait_native_ui(None, publication, 8)["elements"] == [3]
    assert publication.index == 3


# ------------------------=
# FUNC: main
# DESC: Rejects incomplete, stale and unauthorized-launch receipts and verifies exact persisted-byte correlation through actual artifact files.
# ------------------=
def main():
    assert os.environ.get("INFINITY_BUILD_KIT_ACTIVE") == "1", "Use ./build-kit run"
    republished_receipts()
    attended_input_boundaries()
    delayed_native_publication()
    display = {"kind": 2, "id": 3, "value": "12"}
    status = {"kind": 1, "id": 0x7fffffff, "value": "12"}
    for element in [display, status]:
        assert workflow.calculator_result({"elements": [element]}) == 12
    for elements in [[], [display, display], [{**status, "id": 4}],
                     [{**status, "value": "Result: 12"}], [{**display, "value": "NaN"}]]:
        assert workflow.calculator_result({"elements": elements}) is None
    receipt = {"goal": 2, "plan_state": 5, "active_goal": False, "completed_tasks": 1, "tasks": 1,
               "source": "01" * 16, "artifact": "02" * 16, "bound_source": "01" * 16, "version": 2,
               "saved": True, "document_bytes": 9, "language": 1, "context": 0}
    assert workflow.terminal_receipt(receipt, 1, 1, False)
    assert not workflow.terminal_receipt(receipt, 2, 1, False)
    assert not workflow.terminal_receipt(receipt, 1, 0, False)
    assert not workflow.terminal_receipt(receipt, 1, 1, True)
    for field, value in [("plan_state", 3), ("active_goal", True), ("completed_tasks", 0),
                         ("source", "00" * 16), ("artifact", "00" * 16),
                         ("bound_source", "03" * 16), ("version", 0), ("saved", False),
                         ("document_bytes", 0), ("context", 99)]:
        changed = {**receipt, field: value}
        assert not workflow.terminal_receipt(changed, 1, 1, False)
    launched = {**receipt, "tasks": 2, "completed_tasks": 2, "context": 99}
    assert workflow.terminal_receipt(launched, 1, 1, True)
    destination = pathlib.Path(tempfile.mkdtemp(prefix="source-workflow-evidence-", dir=ROOT / "build"))
    content = b"print(1)\n"
    (destination / "source.py").write_bytes(content)
    fingerprint = 1469598103934665603
    for byte in content:
        fingerprint = ((fingerprint ^ byte) * 1099511628211) & ((1 << 64) - 1)
    path = "/home/default/documents/hello.py"
    exports = {"directory": str(destination), "artifacts": [{"source": True, "object": receipt["source"],
               "version": 2, "path_hex": path.encode().hex(), "file": "source.py"}]}
    stages = {"stages": [{**receipt, "path": path, "editor_fingerprint": fingerprint}]}
    assert workflow.verify_exports(exports, stages)["verified_source_objects"] == [receipt["source"]]
    marker = b"# local-note: 5729\n"
    updated_content = marker + b"print(3)\n"
    (destination / "source-v3.py").write_bytes(updated_content)
    updated_fingerprint = 1469598103934665603
    for byte in updated_content:
        updated_fingerprint = ((updated_fingerprint ^ byte) * 1099511628211) & ((1 << 64) - 1)
    history = copy.deepcopy(exports)
    history["artifacts"].append({**history["artifacts"][0], "version": 3, "file": "source-v3.py", "current": True})
    history["artifacts"][0]["current"] = False
    updated_stage = {**stages["stages"][0], "version": 3, "document_bytes": len(updated_content),
                     "editor_fingerprint": updated_fingerprint, "protected_prefix_hex": marker.hex()}
    history_stages = {"stages": stages["stages"] + [updated_stage]}
    verified = workflow.verify_exports(history, history_stages)
    assert verified["verified_versions"] == 2 and len(verified["immutable_version_diffs"]) == 1
    wrong_marker = {**updated_stage, "protected_prefix_hex": b"# a different user's edit\n".hex()}
    try:
        workflow.verify_exports(history, {"stages": [wrong_marker]})
    except AssertionError:
        pass
    else:
        raise AssertionError("Discarded unrelated editor bytes were accepted")
    missing_history = {**history, "artifacts": history["artifacts"][1:]}
    try:
        workflow.verify_exports(missing_history, history_stages)
    except AssertionError:
        pass
    else:
        raise AssertionError("Unverified historical editor revision was accepted")
    previous = destination / "previous.json"
    completed = {**stages["stages"][0], **launched,
                 "native_receipt": [0x494e46505954484e, 1, 8, 2, 0, 2, 0, 64, 64, 8]}
    report = {"architecture": "aarch64", "source_workflows": {"stages": [completed]}, "stored_artifacts": exports}
    previous.write_text(json.dumps(report))
    carried = workflow.verified_python_evidence(previous)
    assert not carried["executed_in_current_guest"] and carried["stage"] == completed
    c_stage = {**completed, "language": 0, "path": "/home/default/documents/source.c",
               "native_receipt": [0x494e46434c414e47, 1, 6, 3, 9, 10, 20, 0, 0, 6]}
    c_exports = copy.deepcopy(exports)
    c_exports["artifacts"][0]["path_hex"] = c_stage["path"].encode().hex()
    c_report = {"architecture": "aarch64", "source_workflows": {"stages": [c_stage]},
                "stored_artifacts": c_exports, "failure": "UI publication failed"}
    previous.write_text(json.dumps(c_report))
    carried = workflow.verified_c_source_evidence(previous)
    assert not carried["created_in_current_guest"] and not carried["creation_ui_verified"]
    assert carried["stage"] == c_stage
    c_report["source_workflows"]["stages"][0]["native_receipt"][3] = 4
    previous.write_text(json.dumps(c_report))
    try:
        workflow.verified_c_source_evidence(previous)
    except AssertionError:
        pass
    else:
        raise AssertionError("Failed native compilation was accepted as saved-app evidence")
    for field, value in [("native_receipt", [0x494e46505954484e, 1, 8, 2, 1, 2, 0, 64, 64, 8]),
                         ("saved", False), ("bound_source", "03" * 16)]:
        changed = copy.deepcopy(report)
        changed["source_workflows"]["stages"][0][field] = value
        previous.write_text(json.dumps(changed))
        try:
            workflow.verified_python_evidence(previous)
        except AssertionError:
            pass
        else:
            raise AssertionError("Unverified prior native stage was accepted")
    for field, value in [("version", 3), ("path_hex", b"/other.py".hex())]:
        changed = copy.deepcopy(exports)
        changed["artifacts"][0][field] = value
        try:
            workflow.verify_exports(changed, stages)
        except AssertionError:
            pass
        else:
            raise AssertionError("Mismatched persisted source was accepted")
    (destination / "source.py").write_bytes(b"print(2)\n")
    try:
        workflow.verify_exports(exports, stages)
    except AssertionError:
        pass
    else:
        raise AssertionError("Different persisted source bytes were accepted")
    print("Installed source-workflow evidence gates: PASS (not native VM acceptance)")


if __name__ == "__main__":
    main()
