#!/usr/bin/env python3
"""Typed child adapter tests; native process execution is verified in the VM."""
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import types
import unittest

ROOT = Path(__file__).resolve().parents[2]


class Native(types.ModuleType):
    # ------------------------=
    # FUNC: __init__
    # DESC: Creates observable native lifecycle receipts without executing any host commands.
    # ------------------=
    def __init__(self):
        super().__init__("_infinity_process")
        self.records = []
        self.released = []
        self.cancelled = []
        self.ready = True
        self.exit = 0
        self.output = b"first\nsecond\n"
        self.errors = b"diagnostic\n"

    # ------------------------=
    # FUNC: start
    # DESC: Captures only the adapter's bounded repository-local launch record and returns an opaque child identity.
    # ------------------=
    def start(self, path):
        path = Path(os.fsdecode(path))
        assert path.is_relative_to(ROOT)
        self.records.append(json.loads(path.read_bytes()))
        return len(self.records)

    # ------------------------=
    # FUNC: poll
    # DESC: Returns actual fixture status independently from the generated output content.
    # ------------------=
    def poll(self, identity):
        return (self.exit, len(self.output), len(self.errors)) if self.ready else None

    # ------------------------=
    # FUNC: read
    # DESC: Supplies precisely the completed output range requested by the shipping adapter.
    # ------------------=
    def read(self, identity, stream, offset, count):
        return (self.errors if stream else self.output)[offset:offset + count]

    # ------------------------=
    # FUNC: release
    # DESC: Records exact receipt cleanup for stale-child protection.
    # ------------------=
    def release(self, identity):
        self.released.append(identity)

    # ------------------------=
    # FUNC: cancel
    # DESC: Records cancellation without inferring completion until a later lifecycle receipt.
    # ------------------=
    def cancel(self, identity):
        self.cancelled.append(identity)


class ChildTests(unittest.TestCase):
    # ------------------------=
    # FUNC: setUp
    # DESC: Loads the real adapter against a new typed native boundary for each test.
    # ------------------=
    def setUp(self):
        self.native = Native()
        sys.modules["_infinity_process"] = self.native
        spec = importlib.util.spec_from_file_location("native_process_adapter", ROOT / "sdk/python/subprocess.py")
        self.adapter = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.adapter)

    # ------------------------=
    # FUNC: test_scoped_record_and_captured_streams
    # DESC: Verifies actual argv/env/cwd marshalling, line boundaries, exit code and receipt cleanup.
    # ------------------=
    def test_scoped_record_and_captured_streams(self):
        process = self.adapter.Popen(["/system/python", "/tmp/hook.py", "build_wheel"],
            cwd="/tmp/project", env={"PYTHONPATH": "/tmp/environment"}, stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT, encoding="utf-8")
        self.assertEqual(self.native.records, [{"argv": process.args, "cwd": "/tmp/project",
                         "env": {"PYTHONPATH": "/tmp/environment"}}])
        self.assertEqual(process.stdout.readline(), "first\n")
        self.assertEqual(process.stdout.read(), "second\ndiagnostic\n")
        self.assertEqual(process.wait(), 0)
        self.assertEqual(self.native.released, [1])
        self.assertIsNone(process._record)

    # ------------------------=
    # FUNC: test_failure_and_cancel_use_real_status
    # DESC: Ensures output cannot turn a failed child into success and cancellation targets only the owned child.
    # ------------------=
    def test_failure_and_cancel_use_real_status(self):
        self.native.ready = False
        process = self.adapter.Popen(["/system/python", "-c", "raise ValueError()"],
                                     stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        self.assertIsNone(process.poll())
        process.kill()
        self.assertEqual(self.native.cancelled, [1])
        self.assertIsNone(process.poll())
        self.native.ready, self.native.exit = True, 7
        self.assertEqual(process.communicate(), (self.native.output, self.native.errors))
        self.assertEqual(process.returncode, 7)
        self.assertEqual(self.native.released, [1])

    # ------------------------=
    # FUNC: test_ambient_execution_and_descriptors_rejected
    # DESC: Rejects shells, other executables, inherited descriptors and pre-execution hooks before native admission.
    # ------------------=
    def test_ambient_execution_and_descriptors_rejected(self):
        for args, options in ((["/bin/sh"], {}), (["clang"], {}),
                (["/system/python", "-c", "pass"], {"shell": True}),
                (["/system/python", "-c", "pass"], {"stdout": 99}),
                (["/system/python", "-c", "pass"], {"pass_fds": (9,)})):
            with self.assertRaises((FileNotFoundError, NotImplementedError)):
                self.adapter.Popen(args, **options)
        self.assertEqual(self.native.records, [])

    # ------------------------=
    # FUNC: test_stdin_never_silently_discards_data
    # DESC: Verifies direct binary/text pipe writes reject nonempty input and missing pipes remain absent.
    # ------------------=
    def test_stdin_never_silently_discards_data(self):
        for text, empty, payload in ((False, b'', b'input'), (True, '', 'input')):
            process = self.adapter.Popen(['/system/python', '-c', 'pass'],
                stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=text)
            self.assertEqual(process.stdin.write(empty), 0)
            with self.assertRaises(NotImplementedError):
                process.stdin.write(payload)
            with self.assertRaises(NotImplementedError):
                process.communicate(payload)
            process.stdin.close()
            with self.assertRaises(ValueError):
                process.stdin.write(empty)
            process.wait()
        process = self.adapter.Popen(['/system/python', '-c', 'pass'],
                                    stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        self.assertIsNone(process.stdin)
        process.wait()

    # ------------------------=
    # FUNC: test_launch_record_authority_is_confined_before_execution
    # DESC: Exercises the shipping child's validator without executing code, denying traversal and unimplemented interpreter flag semantics.
    # ------------------=
    def test_launch_record_authority_is_confined_before_execution(self):
        spec = importlib.util.spec_from_file_location('native_child_entry_test', ROOT / 'sdk/python/child_entry.py')
        entry = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(entry)
        valid = {'argv': ['/system/python', '-u', '-B', '/tmp/hook.py', 'build_wheel'],
                 'env': {'PYTHONPATH': '/tmp/build-env/site'}, 'cwd': '/tmp/source'}
        self.assertEqual(entry._validate_record(valid),
                         (['/tmp/hook.py', 'build_wheel'], valid['env'], '/tmp/source'))
        for changes in ({'cwd': '/tmp/../system'}, {'cwd': '/system'},
                {'env': {'PYTHONPATH': '/tmp/../packages'}}, {'env': {'PYTHONPATH': '/packages'}},
                {'env': {'BAD=KEY': 'value'}}, {'env': {'KEY': 'bad\0value'}},
                {'argv': ['/system/python', '/tmp/../packages/hook.py']},
                {'argv': ['/system/python', '-I', '/tmp/hook.py']},
                {'argv': ['/system/python', '-m']},
                {'argv': ['/bin/sh', '/tmp/hook.py']}):
            with self.assertRaises((ValueError, PermissionError)):
                entry._validate_record(dict(valid, **changes))


if __name__ == "__main__":
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1" or not Path(tempfile.gettempdir()).is_relative_to(ROOT):
        raise SystemExit("Use ./build-kit run python3 tools/native-python/process-test.py")
    unittest.main()
