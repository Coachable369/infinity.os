"""Behavioral tests of the case runner itself, not generated-app acceptance."""
import copy
import importlib.util
import json
import pathlib
import tempfile

ROOT = pathlib.Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("application_case", ROOT / "tools/native-application-case.py")
case = importlib.util.module_from_spec(spec)
spec.loader.exec_module(case)


class Receiver:
    # ------------------------=
    # FUNC: __init__
    # DESC: Initializes a typed runner fixture; it is not generated application code or VM evidence.
    # ------------------=
    def __init__(self):
        self.state = {"ui": {"elements": [{"id": 20, "kind": 2, "value": "0"},
                                             {"id": 1, "kind": 3, "value": "X"},
                                             {"id": 2, "kind": 3, "value": ""}]},
                      "contract": {"revision": 1, "applied": 0, "agent_turn": 0}}
        self.clicks = []

    # ------------------------=
    # FUNC: screenshot
    # DESC: Implements the runner evidence sink without claiming actual framebuffer capture.
    # ------------------=
    def screenshot(self, name):
        pass

    # ------------------------=
    # FUNC: ui
    # DESC: Returns a detached snapshot so expected-state tests cannot mutate the simulated receiver.
    # ------------------=
    def ui(self):
        return copy.deepcopy(self.state["ui"])

    # ------------------------=
    # FUNC: contract
    # DESC: Returns detached typed counters used to exercise relative assertions.
    # ------------------=
    def contract(self):
        return copy.deepcopy(self.state["contract"])

    # ------------------------=
    # FUNC: click
    # DESC: Records dispatch and performs one observable fixture transition; the other input is deliberately rejected.
    # ------------------=
    def click(self, identifier):
        self.clicks.append(identifier)
        if identifier == 7:
            self.state["ui"]["elements"][0]["value"] = "1.0"
            self.state["ui"]["elements"][2]["value"] = "O"
            self.state["contract"].update(revision=2, applied=1)


# ------------------------=
# FUNC: rejects
# DESC: Requires invalid or false assertions to fail without interpreting diagnostic prose.
# ------------------=
def rejects(operation, error_type):
    try:
        operation()
    except error_type:
        return
    raise AssertionError("Invalid case was accepted")


# ------------------------=
# FUNC: main
# DESC: Verifies numeric/board expectations, revision advancement, rejection stability, bounded cases and failure receipts.
# ------------------=
def main():
    receiver = Receiver()
    initial = copy.deepcopy(receiver.state)
    assert case.expected_state(initial, {"numbers": [{"id": 20, "equals": 0}]}, {})
    assert not case.expected_state(initial, {"numbers": [{"id": 20, "equals": 5}]}, {})
    assert case.expected_state(initial, {"board": {"ids": [1, 2], "cells": ["X", ""]}}, {})
    assert not case.expected_state(initial, {"board": {"ids": [1, 2], "cells": ["X", "O"]}}, {})
    rejects(lambda: case.expected_state(initial, {"numbers": []}, {}), ValueError)
    rejects(lambda: case.expected_state(initial, {"status_text": "passed"}, {}), ValueError)
    rejects(lambda: case.expected_state(initial, {"numbers": [{"id": 20, "equals": "NaN"}]}, {}), ValueError)
    rejects(lambda: case.bounded_integer(True, 1, 2), ValueError)
    assert case.resolve_identifier(initial, {"kind": 2}) == 20
    rejects(lambda: case.resolve_identifier(initial, {"kind": 3}), LookupError)
    grid = {"ui": {"elements": [
        {"id": 100 + index * 7, "kind": 3, "value": "", "x": index % 3 * 40,
         "y": index // 3 * 40, "width": 32, "height": 32} for index in range(9)
    ]}, "contract": {}}
    selectors = [{"kind": 3, "grid": {"rows": 3, "columns": 3, "row": row, "column": column}}
                 for row in range(3) for column in range(3)]
    assert [case.resolve_identifier(grid, selector) for selector in selectors] == [100 + index * 7 for index in range(9)]
    assert case.expected_state(grid, {"board": {"ids": selectors, "cells": [""] * 9}}, {})
    assert case.expected_state(grid, {"board": {"ids": selectors, "counts": {"": 9, "X": 0, "O": 0}}}, {})
    assert not case.expected_state(grid, {"board": {"ids": selectors, "counts": {"": 7, "X": 1, "O": 1}}}, {})
    rejects(lambda: case.expected_state(grid, {"board": {"ids": selectors, "counts": {"": 8}}}, {}), ValueError)
    grid["ui"]["elements"][0]["value"] = "X"
    grid["ui"]["elements"][4]["value"] = "O"
    assert case.expected_state(grid, {"board": {"ids": selectors, "counts": {"": 7, "X": 1, "O": 1}}}, {})
    for element in grid["ui"]["elements"]:
        element["id"] += 500
    grid["ui"]["elements"].reverse()
    assert [case.resolve_identifier(grid, selector) for selector in selectors] == [600 + index * 7 for index in range(9)]
    grid["ui"]["elements"].append({"id": 9, "kind": 3, "value": "7", "x": 500, "y": 500, "width": 32, "height": 32})
    assert case.resolve_identifier(grid, {"kind": 3, "number": 7}) == 9
    rejects(lambda: case.resolve_identifier(grid, selectors[0]), LookupError)
    selectors[0]["grid"]["bounds"] = [0, 0, 120, 120]
    assert case.resolve_identifier(grid, selectors[0]) == 600
    grid["ui"]["elements"][0]["x"] = 79
    rejects(lambda: case.resolve_identifier(grid, selectors[0]), LookupError)
    work = pathlib.Path(tempfile.mkdtemp(prefix="application-case-", dir=ROOT / "build"))
    script = {"version": 1, "steps": [
        {"action": "expect", "state": {"numbers": [{"id": 20, "equals": 0}]}},
        {"action": "click", "id": 7},
        {"action": "expect", "state": {"numbers": [{"id": 20, "equals": 1}],
            "board": {"ids": [1, 2], "cells": ["X", "O"]},
            "contract": {"revision": {"after": "initial"}, "applied": {"gte": 1}}}, "save_as": "accepted"},
        {"action": "click", "id": 9},
        {"action": "expect_unchanged", "from": "accepted", "ui_ids": [20, 1, 2],
            "contract_fields": ["revision", "applied"], "for_seconds": 1},
    ]}
    path = work / "case.json"
    path.write_text(json.dumps(script))
    result = case.run(receiver, path, work / "passed", Receiver.ui, Receiver.contract, Receiver.click)
    assert result["passed"] and result["inputs"] == 2 and result["checks"] == 3
    assert receiver.clicks == [7, 9]
    assert json.loads((work / "passed/result.json").read_text())["passed"]
    script["steps"] = [{"action": "expect", "within_seconds": 1,
                        "state": {"numbers": [{"id": 20, "equals": 99}]}}]
    path.write_text(json.dumps(script))
    rejects(lambda: case.run(receiver, path, work / "failed", Receiver.ui, Receiver.contract, Receiver.click), AssertionError)
    failed = json.loads((work / "failed/result.json").read_text())
    assert not failed["passed"] and not failed["checks"]
    print(json.dumps({"case_runner_verified": True, "generated_app_acceptance": False, "evidence": str(work)}))


if __name__ == "__main__":
    main()
