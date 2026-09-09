"""Actual frame counting remains available after bounded trace retention fills."""
import unittest
import struct
from ms10_ethernet_hub import EthernetHub


class Counters(unittest.TestCase):
    # ------------------------=
    # FUNC: data_frame
    # DESC: Builds a canonical encrypted-envelope-shaped fixture without pretending its dummy ciphertext is authenticated.
    # ------------------=
    @staticmethod
    def data_frame(sequence=1, session=1, changed=0):
        frame = bytearray(42 + 154)
        frame[12:14] = b"\x08\x00"
        frame[14] = 0x45
        frame[23] = 17
        struct.pack_into("!H", frame, 16, len(frame) - 14)
        struct.pack_into("!H", frame, 38, len(frame) - 34)
        frame[42:51] = b"IN9A0001\x0b"
        frame[51] = 1
        frame[42+16:42+48] = bytes([2]) * 32
        frame[42+48:42+80] = bytes([3]) * 32
        frame[42+80:42+112] = bytes([4]) * 32
        frame[42+112:42+128] = bytes([session]) * 16
        struct.pack_into("<Q", frame, 42+128, sequence)
        frame[-1] = changed
        return frame

    # ------------------------=
    # FUNC: test_duplicate_sequence_reconnect_and_changed_ciphertext
    # DESC: Counts only exact repeated DATA for the same sender/session/sequence, not new sequences or reconnected sessions.
    # ------------------=
    def test_duplicate_sequence_reconnect_and_changed_ciphertext(self):
        hub = EthernetHub().start()
        try:
            for frame in (self.data_frame(), self.data_frame(), self.data_frame(sequence=2),
                          self.data_frame(session=2), self.data_frame(changed=1)):
                hub.observe_metadata(frame, 1)
            result = hub.traffic_snapshot()
            self.assertEqual(result["observed_wire_retransmissions"], 1)
            self.assertEqual(result["changed_data_sequence_reuse"], 1)
            self.assertEqual(len(hub.data_sequences), 2)
        finally:
            hub.close()

    # ------------------------=
    # FUNC: test_invalid_envelope_and_bounded_sequence_retention
    # DESC: Rejects truncated canonical headers and caps sequence memory; evicted old packets never become fabricated retries.
    # ------------------=
    def test_invalid_envelope_and_bounded_sequence_retention(self):
        hub = EthernetHub().start()
        try:
            hub.observe_metadata(self.data_frame()[:-1], 1)
            self.assertEqual(len(hub.data_sequences), 0)
            for sequence in range(65):
                hub.observe_metadata(self.data_frame(sequence=sequence), 1)
            hub.observe_metadata(self.data_frame(sequence=0), 1)
            self.assertEqual(hub.traffic_snapshot()["observed_wire_retransmissions"], 0)
            self.assertEqual(hub.traffic_snapshot()["data_tracking_evictions"], 2)
            self.assertEqual(len(next(iter(hub.data_sequences.values()))), 64)
        finally:
            hub.close()

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
