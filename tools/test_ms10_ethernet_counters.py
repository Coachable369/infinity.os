"""Actual frame counting remains available after bounded trace retention fills."""
import unittest
from ms10_ethernet_hub import EthernetHub


class Counters(unittest.TestCase):
    # ------------------------=
    # FUNC: test_counters_are_independent_of_bounded_payload_free_trace
    # DESC: Counts received bytes, forwarding copies and native DATA headers without retaining payload or inventing retransmissions.
    # ------------------=
    def test_counters_are_independent_of_bounded_payload_free_trace(self):
        hub = EthernetHub(metadata_limit=1).start()
        try:
            frame = bytearray(60)
            frame[12:14] = b"\x08\x00"
            frame[14] = 0x45
            frame[23] = 17
            frame[42:50] = b"IN9A0001"
            frame[50] = 11
            for _ in range(3):
                hub.observe_metadata(frame, 2)
            observed = hub.traffic_snapshot()
            self.assertEqual(observed["frames"], 3)
            self.assertEqual(observed["bytes"], 180)
            self.assertEqual(observed["forwarded_copies"], 6)
            self.assertEqual(observed["native_data_frames"], 3)
            self.assertEqual(len(hub.metadata), 1)
            self.assertEqual(hub.metadata_dropped, 2)
        finally:
            hub.close()


if __name__ == "__main__":
    unittest.main()
