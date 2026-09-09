"""Behavioral completion fencing for explicitly durable installed Pool approvals."""
import struct
import unittest
from ms10_installed_durable_grants import decode_approval, grant


# ------------------------=
# FUNC: completion
# DESC: Builds an independent canonical NodeOperation response fixture.
# ------------------=
def completion():
    payload = bytes([17]) * 32 + struct.pack("<QQQIIIIH6x", (1 << 63) | 7, 0, 0,
                                          0xd022, 1, 0xe050, 3, 1)
    state = [0] * 512
    state[73:77] = [12, 4, 5, 1]
    state[77:87] = struct.unpack("<10Q", payload)
    return state


class DurableGrants(unittest.TestCase):
    # ------------------------=
    # FUNC: test_exact_completion_and_rejections
    # DESC: Rejects stale, failed, wrong-peer, wrong-operation, ephemeral and finite responses independently of rendered text.
    # ------------------=
    def test_exact_completion_and_rejections(self):
        state = completion()
        self.assertEqual(decode_approval(state, 11, "11" * 32, "pool-metadata"), (1 << 63) | 7)
        cases = [(73, 11), (76, 2), (77, 0), (81, 7), (83, 100),
                 (85, (3 << 32) | 0x3002), (86, 2)]
        for index, value in cases:
            changed = state.copy()
            changed[index] = value
            with self.subTest(index=index), self.assertRaises(AssertionError):
                decode_approval(changed, 11, "11" * 32, "pool-metadata")

    # ------------------------=
    # FUNC: test_single_submission_waits_for_new_completion
    # DESC: A stale successful response cannot satisfy the wait; failed fresh completion is surfaced without resubmission.
    # ------------------=
    def test_single_submission_waits_for_new_completion(self):
        class Guest:
            # ------------------------=
            # FUNC: __init__
            # DESC: Tracks command submissions independently of approval completion.
            # ------------------=
            def __init__(self):
                self.submissions = 0

            # ------------------------=
            # FUNC: state
            # DESC: Returns the previous committed response.
            # ------------------=
            def state(self):
                result = completion()
                result[73] = 11
                return result

            # ------------------------=
            # FUNC: command
            # DESC: Counts actual admission attempts without using command prose as an oracle.
            # ------------------=
            def command(self, command):
                self.submissions += 1

            # ------------------------=
            # FUNC: wait
            # DESC: Evaluates the state predicate against old and new binary completions.
            # ------------------=
            def wait(self, predicate, description, timeout):
                assert not predicate(self.state())
                result = completion()
                result[76] = 2
                assert predicate(result)
                return result

        guest = Guest()
        with self.assertRaises(AssertionError):
            grant(guest, "11" * 32, "pool-metadata")
        self.assertEqual(guest.submissions, 1)
