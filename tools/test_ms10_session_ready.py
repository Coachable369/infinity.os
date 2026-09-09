"""Exact peer/session predicate for already established installed links."""
import importlib.util
import pathlib
import struct
import unittest

SPEC = importlib.util.spec_from_file_location("distribution", pathlib.Path(__file__).with_name("ms10-installed-distribution.py"))
D = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(D)


class SessionPredicate(unittest.TestCase):
    # ------------------------=
    # FUNC: test_exact_live_transaction_not_count_increment
    # DESC: Accepts an existing established peer and rejects wrong-peer, expired, pending and empty transaction states.
    # ------------------=
    def test_exact_live_transaction_not_count_increment(self):
        peer = "ab" * 32
        state = [0] * 512
        state[32:36] = struct.unpack("<4Q", bytes.fromhex(peer))
        state[10], state[26], state[56], state[58], state[64] = 100, 3, 11, 200, 7
        self.assertTrue(D.session_ready(state, peer))
        self.assertFalse(D.session_ready(state, "cd" * 32))
        for slot, value in ((26, 0), (56, 10), (58, 100), (64, 0)):
            changed = state.copy()
            changed[slot] = value
            self.assertFalse(D.session_ready(changed, peer))


if __name__ == "__main__":
    unittest.main()
