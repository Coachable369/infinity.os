"""Structured failure observation tests without VM interaction."""
import unittest
from types import SimpleNamespace
from unittest.mock import patch
from ms10_installed_failure_evidence import safe_state, capture_final


class Evidence(unittest.TestCase):
    # ------------------------=
    # FUNC: test_pending_request_and_idle_transition_are_preserved
    # DESC: Captures actual request/offset and subsequent idle state without inferring successful transfer.
    # ------------------=
    def test_pending_request_and_idle_transition_are_preserved(self):
        main, pool = [0]*512, [0]*256
        main[10], main[26], main[61] = 17, 2, 26
        pool[26:30] = [3, 8128, 32768, 901]
        active = safe_state(main, pool)
        self.assertEqual((active["phase"], active["request"], active["offset"]), (3, 901, 8128))
        pool[26:30] = [0, 0, 0, 0]
        idle = safe_state(main, pool)
        self.assertEqual(idle["phase"], 0)
        self.assertEqual(idle["transport_error"], 26)
        self.assertNotIn("success", idle)

    # ------------------------=
    # FUNC: test_capture_failure_does_not_suppress_other_guest_or_hub
    # DESC: One unavailable controller is recorded without losing remaining final observations.
    # ------------------=
    def test_capture_failure_does_not_suppress_other_guest_or_hub(self):
        guests = [SimpleNamespace(number=n, process=SimpleNamespace(poll=lambda: None)) for n in (1, 2)]
        hub = SimpleNamespace(traffic_snapshot=lambda: {"frames": 25})
        with patch("ms10_installed_failure_evidence.capture", side_effect=[RuntimeError("unavailable"), {"clock": 17}]):
            result = capture_final(guests, hub, None)
        self.assertEqual(result["hub"]["frames"], 25)
        self.assertIn("capture_error", result["guests"][0])
        self.assertEqual(result["guests"][1]["clock"], 17)
