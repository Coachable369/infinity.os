"""Behavioral tests for bounded offset measurement and exact object selection."""
import struct
import unittest
from unittest.mock import patch
from ms10_installed_transfer_measurement import measure


class Fixture:
    # ------------------------=
    # FUNC: __init__
    # DESC: Creates an advancing deterministic clock and independent object projections.
    # ------------------=
    def __init__(self, finish=True):
        self.now = 0
        self.finish = finish

    # ------------------------=
    # FUNC: launch
    # DESC: Supplies the harness window activation boundary without external mutation.
    # ------------------=
    def launch(self, *_):
        pass

    # ------------------------=
    # FUNC: pause
    # DESC: Advances virtual time without sleeping.
    # ------------------=
    def pause(self, seconds):
        self.now += seconds

    # ------------------------=
    # FUNC: state
    # DESC: Emits receipt-backed state only after three virtual ticks and keeps an unrelated healthy object visible throughout.
    # ------------------=
    def state(self, *_):
        pool = [0] * 256
        pool[18] = 2
        pool[24] = 1
        pool[26:29] = [4, self.now * 64, 256]
        pool[240] = 17
        pool[242] = 19
        pool[32] = 99
        pool[37:39] = [3, 3]
        pool[48] = 17
        pool[52] = 256
        pool[53:55] = [3, 3 if self.finish and self.now >= 3 else 1]
        return pool


class Measurement(unittest.TestCase):
    # ------------------------=
    # FUNC: test_completed_idle_object_cannot_pass_loaded_interaction
    # DESC: A completed object without an observed active window must fail instead of accepting idle UI behavior.
    # ------------------=
    def test_completed_idle_object_cannot_pass_loaded_interaction(self):
        fake = Fixture()
        fake.now = 3
        original = fake.state
        # ------------------------=
        # FUNC: idle
        # DESC: Clears the active transfer while retaining a healthy receipt-backed object.
        # ------------------=
        def idle(*args):
            state = original(*args)
            state[26] = 0
            return state
        with patch("ms10_installed_transfer_measurement.fixture.read_state", idle):
            with self.assertRaises(AssertionError):
                measure(fake, None, struct.pack("<2Q", 17, 0).hex(),
                        clock=lambda: fake.now, pause=fake.pause, during_transfer=lambda: {})

    # ------------------------=
    # FUNC: test_exact_object_and_actual_offset_samples
    # DESC: An unrelated healthy row cannot satisfy protection; acknowledged byte increments produce measured windows.
    # ------------------=
    def test_exact_object_and_actual_offset_samples(self):
        fake = Fixture()
        with patch("ms10_installed_transfer_measurement.fixture.read_state", fake.state):
            result = measure(fake, None, struct.pack("<2Q", 17, 0).hex(),
                             clock=lambda: fake.now, pause=fake.pause)
        self.assertEqual(result["elapsed_seconds"], 3)
        self.assertEqual(result["windows"], [{"bytes": 64, "seconds": 1}] * 3)
        self.assertFalse(result["full_persisted_byte_verification"])

    # ------------------------=
    # FUNC: test_finite_failure_without_healthy_receipt
    # DESC: A stalled target stops at its existing deadline instead of treating progress or another object as completion.
    # ------------------=
    def test_finite_failure_without_healthy_receipt(self):
        fake = Fixture(finish=False)
        with patch("ms10_installed_transfer_measurement.fixture.read_state", fake.state):
            with self.assertRaises(AssertionError):
                measure(fake, None, struct.pack("<2Q", 17, 0).hex(), timeout=4,
                        clock=lambda: fake.now, pause=fake.pause)
        self.assertEqual(fake.now, 4)


if __name__ == "__main__":
    unittest.main()
