"""Bounded versioned JSON diagnostics for the native application console."""
import json
import sys

DEBUG, INFO, WARNING, ERROR = range(4)

# ------------------------=
# FUNC: log
# DESC: Emits one bounded structured diagnostic without configuring global logging or swallowing sink failures.
# ------------------=
def log(level, component, message, sink=None):
    if type(level) is not int or level not in range(4):
        raise ValueError("Unknown severity")
    for value, limit in ((component, 32), (message, 256)):
        if not isinstance(value, str) or len(value.encode("utf-8")) > limit:
            raise ValueError("Invalid or oversized log field")
    record = json.dumps(dict(version=1, level=level, component=component, message=message),
                        ensure_ascii=False, separators=(",", ":")) + "\n"
    target = sys.stderr if sink is None else sink
    written = target.write(record)
    if written != len(record):
        raise OSError("Incomplete log write")
    target.flush()
