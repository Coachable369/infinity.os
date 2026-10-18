"""Bounded real-input native application cases; no source or status-prose oracle."""
import decimal
import json
import time

CONTRACT_FIELDS = {"instance", "revision", "agent_turn", "terminal", "choice_count",
                   "schema_count", "commits", "delivered", "applied", "rejected"}


# ------------------------=
# FUNC: bounded_integer
# DESC: Validates numeric test bounds without treating booleans as IDs or durations.
# ------------------=
def bounded_integer(value, minimum, maximum):
    if type(value) is not int or not minimum <= value <= maximum:
        raise ValueError(f"Expected integer within {minimum}..{maximum}")
    return value


# ------------------------=
# FUNC: element_map
# DESC: Indexes actual generated semantic elements and rejects duplicate IDs.
# ------------------=
def element_map(snapshot):
    elements = snapshot["ui"]["elements"]
    result = {element["id"]: element for element in elements}
    assert len(result) == len(elements)
    return result


# ------------------------=
# FUNC: resolve_identifier
# DESC: Resolves a unique live control by kind, numeric value or measured grid position without fixed generated source IDs.
# ------------------=
def resolve_identifier(snapshot, selector):
    if type(selector) is int:
        return bounded_integer(selector, 1, 0xffffffff)
    if not isinstance(selector, dict) or "kind" not in selector or set(selector) - {"kind", "number", "grid"}:
        raise ValueError("Control selector needs kind and optionally number or grid")
    kind = bounded_integer(selector["kind"], 1, 3)
    if "number" in selector and "grid" in selector:
        raise ValueError("Choose a numeric or geometric selector, not both")
    matches = [element for element in element_map(snapshot).values() if element["kind"] == kind]
    if "number" in selector:
        number = decimal.Decimal(str(selector["number"]))
        if not number.is_finite():
            raise ValueError("Numeric selector must be finite")
        numeric = []
        for element in matches:
            try:
                value = decimal.Decimal(element["value"].strip())
                if value.is_finite() and value == number:
                    numeric.append(element)
            except decimal.InvalidOperation:
                pass
        matches = numeric
    if "grid" in selector:
        grid = selector["grid"]
        if not isinstance(grid, dict) or set(grid) - {"rows", "columns", "row", "column", "bounds"} or not {"rows", "columns", "row", "column"} <= set(grid):
            raise ValueError("Grid selector needs dimensions and zero-based row/column")
        rows = bounded_integer(grid["rows"], 1, 96)
        columns = bounded_integer(grid["columns"], 1, 96)
        if rows * columns > 96:
            raise ValueError("Grid exceeds native UI element capacity")
        row = bounded_integer(grid["row"], 0, rows - 1)
        column = bounded_integer(grid["column"], 0, columns - 1)
        if "bounds" in grid:
            bounds = grid["bounds"]
            if not isinstance(bounds, list) or len(bounds) != 4:
                raise ValueError("Grid bounds need logical x, y, width, height")
            left, top, width, height = [bounded_integer(value, 0, 65535) for value in bounds]
            if not width or not height:
                raise ValueError("Grid bounds must be nonempty")
            matches = [element for element in matches
                       if left * 2 <= element["x"] * 2 + element["width"] < (left + width) * 2
                       and top * 2 <= element["y"] * 2 + element["height"] < (top + height) * 2]
        horizontal = sorted({element["x"] * 2 + element["width"] for element in matches})
        vertical = sorted({element["y"] * 2 + element["height"] for element in matches})
        occupied = {(element["x"] * 2 + element["width"], element["y"] * 2 + element["height"]) for element in matches}
        if len(matches) != rows * columns or len(occupied) != len(matches) or len(horizontal) != columns or len(vertical) != rows:
            raise LookupError("Live controls do not form the declared complete grid")
        matches = [element for element in matches
                   if element["x"] * 2 + element["width"] == horizontal[column]
                   and element["y"] * 2 + element["height"] == vertical[row]]
    if len(matches) != 1:
        raise LookupError("Live selector must resolve exactly one control")
    return bounded_integer(matches[0]["id"], 1, 0xffffffff)


# ------------------------=
# FUNC: expected_state
# DESC: Checks actual numeric controls, board cells and typed contract fields against independent case expectations.
# ------------------=
def expected_state(snapshot, expected, captures):
    allowed = {"numbers", "board", "contract"}
    if not isinstance(expected, dict) or set(expected) - allowed or not any(expected.values()):
        raise ValueError("Expectation must contain numbers, board or typed contract fields")
    if "numbers" in expected and (not isinstance(expected["numbers"], list) or len(expected["numbers"]) > 96):
        raise ValueError("Numeric expectations must be a bounded list")
    elements = element_map(snapshot)
    for entry in expected.get("numbers", []):
        if set(entry) != {"id", "equals"}:
            raise ValueError("Numeric expectation needs id and equals")
        try:
            identifier = resolve_identifier(snapshot, entry["id"])
        except LookupError:
            return False
        wanted = decimal.Decimal(str(entry["equals"]))
        if not wanted.is_finite():
            raise ValueError("Numeric expectation must be finite")
        element = elements.get(identifier)
        if element is None:
            return False
        try:
            actual = decimal.Decimal(element["value"].strip())
        except decimal.InvalidOperation:
            return False
        if not actual.is_finite() or actual != wanted:
            return False
    if "board" in expected:
        board = expected["board"]
        if not isinstance(board, dict) or set(board) - {"ids", "cells", "counts"} or "ids" not in board or not {"cells", "counts"} & set(board):
            raise ValueError("Board expectation needs IDs and cell values or state counts")
        if not isinstance(board["ids"], list) or not 1 <= len(board["ids"]) <= 96:
            raise ValueError("Board IDs must be a bounded list")
        if "cells" in board and (not isinstance(board["cells"], list) or len(board["ids"]) != len(board["cells"])):
            raise ValueError("Board expectation needs equally sized semantic IDs and cells")
        try:
            identifiers = [resolve_identifier(snapshot, selector) for selector in board["ids"]]
        except LookupError:
            return False
        if len(set(identifiers)) != len(identifiers):
            raise ValueError("Board expectation repeats a cell ID")
        for identifier, cell in zip(identifiers, board.get("cells", [])):
            if not isinstance(cell, str) or len(cell.encode("utf-8")) > 8:
                raise ValueError("Board cells must be short piece/state values, not status prose")
            if identifier not in elements or elements[identifier]["value"] != cell:
                return False
        if "counts" in board:
            counts = board["counts"]
            if not isinstance(counts, dict) or not 1 <= len(counts) <= 96:
                raise ValueError("Board counts must describe bounded short state codes")
            for cell, count in counts.items():
                if not isinstance(cell, str) or len(cell.encode("utf-8")) > 8:
                    raise ValueError("Board counts cannot use status prose")
                bounded_integer(count, 0, 96)
            if sum(counts.values()) != len(identifiers):
                raise ValueError("Board counts must account for every selected cell")
            observed = {}
            for identifier in identifiers:
                if identifier not in elements:
                    return False
                value = elements[identifier]["value"]
                observed[value] = observed.get(value, 0) + 1
            if any(observed.get(cell, 0) != count for cell, count in counts.items()) or set(observed) - set(counts):
                return False
    for field, comparison in expected.get("contract", {}).items():
        if field not in CONTRACT_FIELDS or not isinstance(comparison, dict) or len(comparison) != 1:
            raise ValueError("Unknown contract field or comparison")
        operator, wanted = next(iter(comparison.items()))
        if operator in ("after", "unchanged_since"):
            wanted = captures[wanted]["contract"][field]
            operator = "gt" if operator == "after" else "eq"
        elif operator not in ("eq", "gte", "gt", "lte") or type(wanted) not in (int, bool):
            raise ValueError("Contract comparisons accept typed scalar values or named captures")
        actual = snapshot["contract"][field]
        if not {"eq": actual == wanted, "gte": actual >= wanted,
                "gt": actual > wanted, "lte": actual <= wanted}[operator]:
            return False
    return True


# ------------------------=
# FUNC: selected_state
# DESC: Extracts actual requested UI values and contract scalars for rejection/unchanged-state checks.
# ------------------=
def selected_state(snapshot, identifiers, contract_fields):
    elements = element_map(snapshot)
    if not identifiers and not contract_fields:
        raise ValueError("Unchanged check must name UI IDs or typed contract fields")
    if any(field not in CONTRACT_FIELDS for field in contract_fields):
        raise ValueError("Unknown unchanged contract field")
    resolved = [resolve_identifier(snapshot, selector) for selector in identifiers]
    return {"ui": {str(identifier): elements[identifier]["value"] for identifier in resolved},
            "contract": {field: snapshot["contract"][field] for field in contract_fields}}


# ------------------------=
# FUNC: run
# DESC: Executes bounded real pointer actions and independent typed-state expectations while preserving exact before/after evidence.
# ------------------=
def run(guest, case_path, evidence, read_ui, read_contract, click):
    case = json.loads(case_path.read_text())
    if set(case) != {"version", "steps"} or case["version"] != 1 or not isinstance(case["steps"], list) or not 1 <= len(case["steps"]) <= 64:
        raise ValueError("Application case requires version 1 and 1..64 steps")
    evidence.mkdir(exist_ok=False)
    (evidence / "case.json").write_text(json.dumps(case, indent=2) + "\n")
    started = time.monotonic()
    captures = {}
    results = []
    inputs = 0
    checks = 0
    report = {"passed": False, "scope": "declared_case_only", "evidence": str(evidence), "inputs": 0, "checks": 0, "steps": results}
    next_activity = time.monotonic() + 10
    try:
        for index, step in enumerate(case["steps"]):
            if time.monotonic() - started > 1200:
                raise AssertionError("Application case exceeded its 1200 second bound")
            if not isinstance(step, dict):
                raise ValueError("Application step must be an object")
            action = step.get("action")
            before = {"ui": read_ui(guest), "contract": read_contract(guest)}
            if not captures:
                captures["initial"] = before
            record = {"index": index, "action": action, "before": before}
            results.append(record)
            if action == "capture":
                if set(step) != {"action", "name"} or not isinstance(step["name"], str) or not 1 <= len(step["name"]) <= 40 or step["name"] in captures:
                    raise ValueError("Capture requires a fresh bounded name")
                captures[step["name"]] = before
            elif action == "click":
                if set(step) != {"action", "id"}:
                    raise ValueError("Click requires exactly a semantic ID or live selector")
                record["resolved_id"] = resolve_identifier(before, step["id"])
                click(guest, record["resolved_id"])
                inputs += 1
                record["after"] = {"ui": read_ui(guest), "contract": read_contract(guest)}
            elif action == "expect":
                if set(step) - {"action", "state", "within_seconds", "save_as"} or "state" not in step:
                    raise ValueError("Expect requires a typed state expectation")
                seconds = bounded_integer(step.get("within_seconds", 10), 1, 180)
                deadline = time.monotonic() + seconds
                while True:
                    current = {"ui": read_ui(guest), "contract": read_contract(guest)}
                    record["after"] = current
                    if expected_state(current, step["state"], captures):
                        break
                    if time.monotonic() >= deadline:
                        raise AssertionError({"expectation_failed": index, "expected": step["state"]})
                    if time.monotonic() >= next_activity:
                        guest.qmp("input-send-event", {"events": [{"type": "rel", "data": {"axis": "x", "value": 1}}]})
                        next_activity = time.monotonic() + 10
                    time.sleep(.2)
                checks += 1
                record["expected"] = step["state"]
                if "save_as" in step:
                    name = step["save_as"]
                    if not isinstance(name, str) or not 1 <= len(name) <= 40 or name in captures:
                        raise ValueError("save_as requires a fresh bounded capture name")
                    captures[name] = current
            elif action == "expect_unchanged":
                if set(step) - {"action", "from", "ui_ids", "contract_fields", "for_seconds"} or "from" not in step:
                    raise ValueError("Unchanged check requires a named baseline")
                seconds = bounded_integer(step.get("for_seconds", 2), 1, 30)
                identifiers = step.get("ui_ids", [])
                fields = step.get("contract_fields", [])
                baseline = selected_state(captures[step["from"]], identifiers, fields)
                deadline = time.monotonic() + seconds
                while True:
                    current = {"ui": read_ui(guest), "contract": read_contract(guest)}
                    record["after"] = current
                    assert selected_state(current, identifiers, fields) == baseline, {"unexpected_state_change": index}
                    if time.monotonic() >= deadline:
                        break
                    if time.monotonic() >= next_activity:
                        guest.qmp("input-send-event", {"events": [{"type": "rel", "data": {"axis": "x", "value": 1}}]})
                        next_activity = time.monotonic() + 10
                    time.sleep(.2)
                checks += 1
            else:
                raise ValueError("Supported actions: capture, click, expect, expect_unchanged")
            record["passed"] = True
            guest.screenshot(f"application-case-{index:02d}")
            (evidence / f"step-{index:02d}.json").write_text(json.dumps(record, indent=2) + "\n")
        assert inputs > 0 and checks >= 2, "Behavioral case needs actual input and at least two independent assertions"
        report["passed"] = True
        return report
    except BaseException as error:
        report["failure"] = repr(error)
        raise
    finally:
        report.update(inputs=inputs, checks=checks, elapsed_seconds=time.monotonic() - started)
        (evidence / "result.json").write_text(json.dumps(report, indent=2) + "\n")
