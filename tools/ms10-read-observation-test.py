"""Behavioral regression tests of the installed acceptance read-completion oracle."""
import importlib.util
import pathlib
import struct
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location("distribution", pathlib.Path(__file__).with_name("ms10-installed-distribution.py"))
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)
REQUEST = (1 << 63) | 17
OBJECT = "12" * 16


class Guest:
    # ------------------------=
    # FUNC: __init__
    # DESC: Creates binary observations with a selectable stale, failed or correctly correlated completion.
    # ------------------=
    def __init__(self, stale=False, completed=REQUEST, error=0):
        self.calls = 0
        self.stale = stale
        self.completed = completed
        self.error = error

    # ------------------------=
    # FUNC: launch
    # DESC: Keeps this oracle test independent of UI launch mechanics already covered elsewhere.
    # ------------------=
    def launch(self, *args):
        pass

    # ------------------------=
    # FUNC: command
    # DESC: Advances the simulated protocol once per submission without inspecting human command text.
    # ------------------=
    def command(self, value):
        self.calls += 1
        return self.state()

    # ------------------------=
    # FUNC: state
    # DESC: Returns a native-shaped storage response, including an intentionally stale payload before consumption when requested.
    # ------------------=
    def state(self):
        state = [0] * 512
        state[110] = int(self.stale or self.calls >= 2)
        raw = bytearray(136)
        struct.pack_into("<HHI", raw, 0, 1, 4, 0x3002)
        raw[8:24] = bytes.fromhex(OBJECT)
        struct.pack_into("<QQ", raw, 32, 2, 2)
        raw[72:76] = b"data"
        state[111:128] = struct.unpack("<17Q", raw)
        return state

    # ------------------------=
    # FUNC: pool
    # DESC: Projects admission and exact completion request identities independently of payload validity.
    # ------------------=
    def pool(self, symbol):
        state = [0] * 256
        state[16] = REQUEST if self.calls else 0
        state[30] = self.completed if self.calls >= 2 else 0
        state[251] = self.error
        return state


class ReadObservation(unittest.TestCase):
    # ------------------------=
    # FUNC: test_correlated_consumption_passes
    # DESC: Accepts only the exact completed request with matching binary identity, version and bytes.
    # ------------------=
    def test_correlated_consumption_passes(self):
        guest = Guest()
        with patch.object(MODULE.fixture, "read_state", lambda g, s: g.pool(s)):
            MODULE.remote_read(guest, OBJECT, [0, 0, 2, 2], b"data")
        self.assertEqual(guest.calls, 2)

    # ------------------------=
    # FUNC: test_stale_admission_rejected
    # DESC: A matching old payload cannot pass merely because its bytes and object identity match.
    # ------------------=
    def test_stale_admission_rejected(self):
        guest = Guest(stale=True)
        with patch.object(MODULE.fixture, "read_state", lambda g, s: g.pool(s)):
            with self.assertRaises(AssertionError):
                MODULE.remote_read(guest, OBJECT, [0, 0, 2, 2], b"data")
        self.assertEqual(guest.calls, 1)

    # ------------------------=
    # FUNC: test_failed_completion_rejected
    # DESC: Correlation alone cannot hide the native terminal error.
    # ------------------=
    def test_failed_completion_rejected(self):
        guest = Guest(error=5)
        with patch.object(MODULE.fixture, "read_state", lambda g, s: g.pool(s)):
            with self.assertRaises(AssertionError):
                MODULE.remote_read(guest, OBJECT, [0, 0, 2, 2], b"data")
        self.assertEqual(guest.calls, 2)

    # ------------------------=
    # FUNC: test_other_request_cannot_pass
    # DESC: A valid-looking payload belonging to another request expires the bounded wait instead of passing.
    # ------------------=
    def test_other_request_cannot_pass(self):
        guest = Guest(completed=REQUEST - 1)
        with patch.object(MODULE.fixture, "read_state", lambda g, s: g.pool(s)):
            with patch.object(MODULE.time, "monotonic", side_effect=[0, 0, 91]):
                with self.assertRaises(AssertionError):
                    MODULE.remote_read(guest, OBJECT, [0, 0, 2, 2], b"data")
        self.assertEqual(guest.calls, 2)


if __name__ == "__main__":
    unittest.main()
