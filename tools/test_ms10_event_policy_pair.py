"""Behavioral generation/version fencing of real policy-pair acceptance."""
import unittest
from unittest.mock import patch
from ms10_installed_pool_event_gap import change_pair


class PolicyPair(unittest.TestCase):
    # ------------------------=
    # FUNC: test_policy_pair_preserves_version_and_requires_both_commits
    # DESC: Two typed policy completions must advance generation while retaining the exact object/content version.
    # ------------------=
    def test_policy_pair_preserves_version_and_requires_both_commits(self):
        first = {"object": "ab" * 16, "generation": 8, "version": 2, "data": bytes(48) + b"\x02"}
        second = {"object": "ab" * 16, "generation": 9, "version": 2, "data": bytes(48) + b"\x03"}
        with patch("ms10_installed_pool_event_gap.call", side_effect=[first, second]) as typed:
            result = change_pair(None, "ab" * 16, 7, 2)
        self.assertEqual(result["generation"], 9)
        self.assertEqual(typed.call_count, 2)
        second["version"] = 3
        with patch("ms10_installed_pool_event_gap.call", side_effect=[first, second]):
            with self.assertRaises(AssertionError):
                change_pair(None, "ab" * 16, 7, 2)


if __name__ == "__main__":
    unittest.main()
