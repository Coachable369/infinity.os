"""Focused installed pairing target and preservation behavior."""
import importlib.util
import pathlib
import struct
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location("pair_target", pathlib.Path(__file__).with_name("ms10-installed-pairing-trace.py"))
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class PairTargets(unittest.TestCase):
    # ------------------------=
    # FUNC: test_target_validation
    # DESC: Only distinct installed endpoints may be requested.
    # ------------------=
    def test_target_validation(self):
        self.assertEqual(MODULE.validate_pair([2, 4]), (2, 4))
        for pair in ([2, 2], [0, 2], [1, 5], [1]):
            with self.assertRaises(AssertionError): MODULE.validate_pair(pair)

    # ------------------------=
    # FUNC: test_reset_allows_unrelated_live_sessions_and_preserves_trust
    # DESC: Revocation invalidates only the selected peer while two unrelated trusted sessions remain live.
    # ------------------=
    def test_reset_allows_unrelated_live_sessions_and_preserves_trust(self):
        state = [0] * 512
        state[24], state[26] = 3, 2
        peers = [bytes([i])*32 for i in (1, 2, 3)]
        state[32:36] = struct.unpack("<4Q", peers[0])
        for row, peer in enumerate(peers):
            data = bytearray(128)
            data[:32], data[85] = peer, 3
            state[128+row*16:144+row*16] = struct.unpack("<16Q", data)
        class Guest:
            # ------------------------=
            # FUNC: state
            # DESC: Returns the current independently encoded trust projection.
            # ------------------=
            def state(self): return state.copy()
            # ------------------------=
            # FUNC: launch
            # DESC: Leaves the selected peer unchanged when opening Console.
            # ------------------=
            def launch(self, *args): pass
            # ------------------------=
            # FUNC: command
            # DESC: Applies successive typed revoke/unblock fixture outcomes without touching other peer rows.
            # ------------------=
            def command(self, value):
                state[20] += 1
                data = bytearray(struct.pack("<16Q", *state[128:144]))
                data[85] = 5 if state[20] == 1 else 1
                state[128:144] = struct.unpack("<16Q", data)
            # ------------------------=
            # FUNC: wait
            # DESC: Requires the actual fixture state to satisfy the helper's transition predicate.
            # ------------------=
            def wait(self, predicate, label, timeout):
                assert predicate(state)
                return state.copy()
        before = MODULE.other_trust(state, peers[0].hex())
        with patch.object(MODULE.DISTRIBUTION, "select", side_effect=lambda guest, peer: guest.state()):
            MODULE.reset_peer(Guest(), peers[0].hex())
        self.assertEqual(MODULE.other_trust(state, peers[0].hex()), before)
        self.assertEqual(state[26], 2)
