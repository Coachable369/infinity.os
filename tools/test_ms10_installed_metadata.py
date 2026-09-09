"""Behavioral acceptance-harness correlation tests; no rendered text oracle."""
import struct
import unittest
from ms10_installed_metadata import invoke, read_path_ready


class Guest:
    # ------------------------=
    # FUNC: __init__
    # DESC: Builds a fake typed projection with a stale successful response from another request.
    # ------------------=
    def __init__(self, error=0, admit=True):
        self.pool = [0] * 256
        self.pool[252] = self.pool[253] = (3 << 62) + 7
        self.commands = []
        self.error, self.admit = error, admit
        raw = bytearray(136)
        struct.pack_into("<HHI", raw, 0, 1, 3, 0x3002)
        raw[72:75] = b"abc"
        self.state = [0] * 512
        self.state[110] = 1
        self.state[111:128] = struct.unpack("<17Q", raw)

    # ------------------------=
    # FUNC: command
    # DESC: Advances the fake admitted request only on its second collection, retaining stale data before then.
    # ------------------=
    def command(self, command):
        self.commands.append(command)
        if len(self.commands) == 1 and self.admit:
            self.pool[252] += 1
        if len(self.commands) == 3:
            self.pool[253] = self.pool[252]
            self.pool[254] = self.error
        return self.state


class MetadataCorrelation(unittest.TestCase):
    # ------------------------=
    # FUNC: test_namespace_readiness_records_non_admission
    # DESC: Eventual read-only lookup records its initial miss and requires a later exact correlated payload.
    # ------------------=
    def test_namespace_readiness_records_non_admission(self):
        guest = DelayedGuest()
        result = read_path_ready(guest, "/Shared/Example", "00" * 16, b"abc", lambda: guest.pool[:])
        self.assertEqual([attempt["admitted"] for attempt in result["readiness_attempts"]], [False, True])
        self.assertEqual(result["collections"], 2)

    # ------------------------=
    # FUNC: test_stale_success_requires_exact_consumed_request
    # DESC: A previous successful payload cannot complete the new request; submission occurs only once.
    # ------------------=
    def test_stale_success_requires_exact_consumed_request(self):
        guest = Guest()
        result = invoke(guest, "operation", lambda: guest.pool[:])
        self.assertEqual(result["collections"], 2)
        self.assertEqual(result["data"], b"abc")
        self.assertEqual(len(guest.commands), 3)
        self.assertEqual(guest.commands[1], guest.commands[2])

    # ------------------------=
    # FUNC: test_failed_completion_cannot_use_stale_success
    # DESC: A typed correlated error fails even while an older successful payload remains present.
    # ------------------=
    def test_failed_completion_cannot_use_stale_success(self):
        guest = Guest(error=9)
        with self.assertRaises(AssertionError):
            invoke(guest, "operation", lambda: guest.pool[:])

    # ------------------------=
    # FUNC: test_non_admission_does_not_retry
    # DESC: An operation lacking a new request identity stops immediately without retransmission.
    # ------------------=
    def test_non_admission_does_not_retry(self):
        guest = Guest(admit=False)
        with self.assertRaises(AssertionError):
            invoke(guest, "operation", lambda: guest.pool[:])
        self.assertEqual(len(guest.commands), 1)


class DelayedGuest(Guest):
    # ------------------------=
    # FUNC: command
    # DESC: Leaves the first lookup unadmitted, then admits once and publishes only on the second collection.
    # ------------------=
    def command(self, command):
        self.commands.append(command)
        if len(self.commands) == 2:
            self.pool[252] += 1
        if len(self.commands) == 4:
            self.pool[253] = self.pool[252]
        return self.state


if __name__ == "__main__":
    unittest.main()
