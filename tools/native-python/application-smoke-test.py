#!/usr/bin/env python3
"""Host wrapper-construction checks only; candidate and guest runner never execute."""
from pathlib import Path
import ast
import hashlib
import importlib.util
import os


# ------------------------=
# FUNC: main
# DESC: Checks exact source binding, literal input preservation, bounded rejection and non-execution during wrapper creation.
# ------------------=
def main():
    assert os.environ.get("INFINITY_BUILD_KIT_ACTIVE") == "1"
    spec = importlib.util.spec_from_file_location(
        "smoke_generator", Path(__file__).with_name("application-smoke.py"))
    generator = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(generator)
    # Invalid source is intentional: construction must neither compile nor run it.
    source = b"raise RuntimeError('must never execute on host')\ninvalid python syntax\\"
    inputs = ["a'\"\\b", "quit"]
    script, binding = generator.wrapper(source, inputs)
    tree = ast.parse(script)
    literals = [ast.literal_eval(node.value) for node in tree.body[:3]]
    assert literals == [source, hashlib.sha256(source).hexdigest(), "a'\"\\b\nquit\n"]
    assert binding["candidate_sha256"] == hashlib.sha256(source).hexdigest()
    assert binding["wrapper_sha256"] == hashlib.sha256(script).hexdigest()
    assert binding["candidate_bytes"] == len(source)
    assert binding["stdin_lines"] == inputs
    assert not binding["candidate_executed_on_host"]
    assert binding["guest_execution_pending"] and not binding["domain_behavior_verified"]
    for candidate, lines in (
        (b"", []), (b"", ["line"] * 16), (b"", ["a\nb"]),
        (b"", ["a\rb"]), (b"", ["x" * 257]), (b"", [None]),
        (b"x" * (generator.SOURCE_LIMIT + 1), ["ok"]),
    ):
        try:
            generator.wrapper(candidate, lines)
        except ValueError:
            pass
        else:
            raise AssertionError("Unbounded or malformed smoke input admitted")


if __name__ == "__main__":
    main()
