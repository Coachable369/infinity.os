"""Behavioral transport-fixture tests; not InfinityOS distributed acceptance."""
import socket
import struct
import unittest
from ms10_ethernet_hub import EthernetHub


class HubTests(unittest.TestCase):
    # ------------------------=
    # FUNC: test_metadata_excludes_payload_and_is_bounded
    # DESC: Checks the explicit public-header projection and truncation bound without retaining application or cryptographic bytes.
    # ------------------=
    def test_metadata_excludes_payload_and_is_bounded(self):
        self.hub.metadata_limit = 1
        frame = bytearray(80)
        frame[12:14] = b"\x08\x00"
        frame[14] = 0x45
        frame[23] = 17
        frame[26:30] = bytes((10, 42, 0, 1))
        frame[30:34] = bytes((10, 42, 0, 2))
        struct.pack_into("!HH", frame, 34, 49154, 49153)
        frame[42:50] = b"IN9A0001"
        frame[50] = 5
        self.hub.observe_metadata(frame, 3)
        first = self.hub.metadata[0]
        self.assertEqual(len(first), 8)
        self.assertEqual(first["wire_kind"], 5)
        self.assertEqual(first["source_ip"], [10, 42, 0, 1])
        self.assertEqual(first["destination_port"], 49153)
        self.assertEqual(first["forwarded"], 3)
        self.assertTrue(all(not isinstance(value, (bytes, bytearray)) for value in first.values()))
        frame[51:] = bytes([0xA7] * 29)
        self.hub.observe_metadata(frame, 3)
        self.assertEqual(len(self.hub.metadata), 1)
        self.assertEqual(self.hub.metadata_dropped, 1)

    # ------------------------=
    # FUNC: setUp
    # DESC: Starts an isolated bounded Ethernet fixture for each behavior.
    # ------------------=
    def setUp(self):
        self.hub = EthernetHub().start()
        self.peers = []

    # ------------------------=
    # FUNC: tearDown
    # DESC: Closes only sockets and forwarding thread owned by this test.
    # ------------------=
    def tearDown(self):
        for peer in self.peers:
            peer.close()
        self.hub.close()

    # ------------------------=
    # FUNC: connect
    # DESC: Attaches one independent framed Ethernet endpoint.
    # ------------------=
    def connect(self):
        peer = socket.create_connection(("127.0.0.1", self.hub.port), timeout=2)
        peer.settimeout(2)
        self.peers.append(peer)
        return peer

    # ------------------------=
    # FUNC: receive
    # DESC: Reads exact framed bytes without using timing or text as an oracle.
    # ------------------=
    def receive(self, peer, count):
        result = bytearray()
        while len(result) < count:
            part = peer.recv(count - len(result))
            self.assertTrue(part)
            result.extend(part)
        return bytes(result)

    # ------------------------=
    # FUNC: test_three_peers_fragmented_frame_and_reconnect
    # DESC: Verifies broadcast fanout, fragmented framing and replacement of a departed node.
    # ------------------=
    def test_three_peers_fragmented_frame_and_reconnect(self):
        a, b, c = (self.connect() for _ in range(3))
        frame = struct.pack("!I", 64) + bytes(range(64))
        # C's frame acknowledges acceptance of all previously connected peers.
        c.sendall(frame)
        self.assertEqual(self.receive(a, len(frame)), frame)
        self.assertEqual(self.receive(b, len(frame)), frame)
        a.sendall(frame[:2])
        a.sendall(frame[2:19])
        a.sendall(frame[19:] + frame)
        self.assertEqual(self.receive(b, len(frame) * 2), frame * 2)
        self.assertEqual(self.receive(c, len(frame) * 2), frame * 2)
        c.close()
        d = self.connect()
        d.sendall(frame)
        self.assertEqual(self.receive(a, len(frame)), frame)
        self.assertEqual(self.receive(b, len(frame)), frame)

    # ------------------------=
    # FUNC: test_invalid_frame_disconnects_only_sender
    # DESC: Rejects oversized input while valid participants continue exchanging frames.
    # ------------------=
    def test_invalid_frame_disconnects_only_sender(self):
        a, b, bad = (self.connect() for _ in range(3))
        bad.sendall(struct.pack("!I", EthernetHub.MAX_FRAME + 1))
        self.assertEqual(bad.recv(1), b"")
        frame = struct.pack("!I", 14) + bytes(range(14))
        b.sendall(frame)
        self.assertEqual(self.receive(a, len(frame)), frame)


if __name__ == "__main__":
    unittest.main()
