"""Behavioral transport-fixture tests; not InfinityOS distributed acceptance."""
import socket
import struct
import unittest
from ms10_ethernet_hub import EthernetHub


class HubTests(unittest.TestCase):
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
