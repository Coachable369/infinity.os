"""Bounded local Ethernet fixture for independently installed QEMU guests.

Forwards QEMU socket-netdev frames only; supplies no guest networking service.
"""
import selectors
import socket
import struct
import threading
import time


class EthernetHub:
    MAX_FRAME = 65535
    MAX_QUEUE = 256 * 1024

    # ------------------------=
    # FUNC: __init__
    # DESC: Binds a private loopback fixture with a bounded number of participants.
    # ------------------=
    def __init__(self, maximum=4, metadata_limit=0):
        assert 2 <= maximum <= 4
        assert 0 <= metadata_limit <= 8192
        self.metadata_limit = metadata_limit
        self.metadata = []
        self.metadata_dropped = 0
        self.maximum = maximum
        self.selector = selectors.DefaultSelector()
        self.listener = socket.socket()
        self.listener.bind(("127.0.0.1", 0))
        self.listener.listen(maximum)
        self.listener.setblocking(False)
        self.port = self.listener.getsockname()[1]
        self.selector.register(self.listener, selectors.EVENT_READ)
        self.clients = {}
        self.stopping = threading.Event()
        self.thread = threading.Thread(target=self.run, daemon=True)
        self.error = None

    # ------------------------=
    # FUNC: start
    # DESC: Starts Ethernet forwarding without starting or mutating guests.
    # ------------------=
    def start(self):
        self.thread.start()
        return self

    # ------------------------=
    # FUNC: drop
    # DESC: Disconnects only an invalid or over-budget fixture participant.
    # ------------------=
    def drop(self, connection):
        self.selector.unregister(connection)
        self.clients.pop(connection)
        connection.close()

    # ------------------------=
    # FUNC: observe_metadata
    # DESC: Retains only bounded public IPv4/UDP headers and protocol kind, never cryptographic or application payload bytes.
    # ------------------=
    def observe_metadata(self, frame, forwarded):
        if not self.metadata_limit or len(frame) < 42 or frame[12:14] != b"\x08\x00":
            return
        header = (frame[14] & 15) * 4
        udp = 14 + header
        if header < 20 or len(frame) < udp + 8 or frame[23] != 17:
            return
        if len(self.metadata) >= self.metadata_limit:
            self.metadata_dropped += 1
            return
        payload = udp + 8
        kind = frame[payload + 8] if len(frame) > payload + 8 and frame[payload:payload+8] == b"IN9A0001" else None
        self.metadata.append({"timestamp_ns": time.monotonic_ns(), "source_ip": list(frame[26:30]),
                              "destination_ip": list(frame[30:34]),
                              "source_port": struct.unpack_from("!H", frame, udp)[0],
                              "destination_port": struct.unpack_from("!H", frame, udp+2)[0],
                              "wire_kind": kind, "length": len(frame), "forwarded": forwarded})

    # ------------------------=
    # FUNC: service
    # DESC: Reassembles bounded QEMU frames and forwards complete frames to other participants.
    # ------------------=
    def service(self, connection, events):
        incoming, outgoing = self.clients[connection]
        if events & selectors.EVENT_READ:
            data = connection.recv(8192)
            if not data:
                self.drop(connection)
                return
            incoming.extend(data)
            while len(incoming) >= 4:
                length = struct.unpack_from("!I", incoming)[0]
                if not 14 <= length <= self.MAX_FRAME:
                    self.drop(connection)
                    return
                if len(incoming) < length + 4:
                    break
                frame = bytes(incoming[:length + 4])
                del incoming[:length + 4]
                forwarded = 0
                for peer, (_, queue) in list(self.clients.items()):
                    if peer is connection:
                        continue
                    if len(queue) + len(frame) > self.MAX_QUEUE:
                        self.drop(peer)
                        continue
                    queue.extend(frame)
                    self.selector.modify(peer, selectors.EVENT_READ | selectors.EVENT_WRITE)
                    forwarded += 1
                self.observe_metadata(frame[4:], forwarded)
        if events & selectors.EVENT_WRITE and outgoing:
            sent = connection.send(outgoing)
            del outgoing[:sent]
        self.selector.modify(connection, selectors.EVENT_READ | (selectors.EVENT_WRITE if outgoing else 0))

    # ------------------------=
    # FUNC: run
    # DESC: Services bounded participants; malformed frames and dead peers cannot stop forwarding.
    # ------------------=
    def run(self):
        try:
            while not self.stopping.is_set():
                for key, events in self.selector.select(.05):
                    connection = key.fileobj
                    if connection is self.listener:
                        peer, _ = self.listener.accept()
                        peer.setblocking(False)
                        if len(self.clients) >= self.maximum:
                            peer.close()
                        else:
                            self.clients[peer] = (bytearray(), bytearray())
                            self.selector.register(peer, selectors.EVENT_READ)
                    elif connection in self.clients:
                        try:
                            self.service(connection, events)
                        except BlockingIOError:
                            pass
                        except (ConnectionError, OSError):
                            if connection in self.clients:
                                self.drop(connection)
        except BaseException as error:
            self.error = error

    # ------------------------=
    # FUNC: close
    # DESC: Stops only the owned fixture and releases its bounded network buffers.
    # ------------------=
    def close(self):
        self.stopping.set()
        self.thread.join(timeout=2)
        assert not self.thread.is_alive()
        for connection in list(self.clients):
            self.drop(connection)
        self.selector.close()
        self.listener.close()
        assert self.error is None, self.error
