"""Host boundary test of the exact prompt source and production Python SDK.

This is not an installed-runtime acceptance test.
"""
import importlib.util
import pathlib
import sys
import types

ROOT = pathlib.Path(__file__).resolve().parents[1]
calls = []
messages = iter(({"kind": 1, "id": 1}, {"kind": 3, "id": 2, "token": 7},
                 {"kind": 3, "id": 99, "token": 8}, {"kind": 2, "id": 0}))

# ------------------------=
# FUNC: record
# DESC: Captures typed SDK operations rather than rendered text or source patterns.
# ------------------=
def record(operation):
    # ------------------------=
    # FUNC: invoke
    # DESC: Records one intercepted native ABI call.
    # ------------------=
    def invoke(*args):
        calls.append((operation, args))
    return invoke

# ------------------------=
# FUNC: load
# DESC: Loads real implementation modules without starting host window services.
# ------------------=
def load(name, path):
    spec = importlib.util.spec_from_file_location(name, ROOT / path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module

native = types.ModuleType("_infinity_app")
for name in ("window", "state", "choice", "text_action", "publish", "done", "yield_native", "element"):
    setattr(native, name, record(name))
native.event = lambda: next(messages)
sys.modules[native.__name__] = native
ui = load("infinity", "sdk/python/infinity.py")
example = load("foundation", "sdk/python/examples/generation_counter.py")
ui.run(example.render, example.handle)
assert example.value == 0
assert [args for op, args in calls if op == "done"] == [(7, True), (8, False)]
assert sum(op == "publish" for op, args in calls) == 3
assert sum(op == "yield_native" for op, args in calls) == 3
accepted = calls.index(("done", (7, True)))
assert calls[accepted - 1][0] == "publish"
assert not example.handle({"kind": ui.CLOSE, "id": 1})
assert example.value == 0
