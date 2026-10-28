#!/usr/bin/env python3
"""Behavioral Python socket/SSL protocol adapter tests.

The actor fixture verifies Python buffering and contracts, not TLS cryptography
or installed networking. Real TLS certificate tests live in http/tests/tls.rs;
installed stdlib HTTPS acceptance is exercised by pip-installed.py --online.
"""
import importlib.util
import pathlib
import sys
import types
import unittest
import errno
import zipfile

ROOT = pathlib.Path(__file__).resolve().parents[2]

class Actor(types.ModuleType):
    class CertificateError(OSError):
        pass
    # ------------------------=
    # FUNC: __init__
    # DESC: Creates a typed socket actor fixture with observable requests, bounded bytes and explicit failure injection.
    # ------------------=
    def __init__(self):
        super().__init__('_infinity_socket')
        self.ticket = 0
        self.state = 0
        self.count = 0
        self.incoming = b''
        self.completed = b''
        self.writes = []
        self.commands = []
        self.closed = []
        self.failure = None
        self.denied = False
        self.pending = False

    # ------------------------=
    # FUNC: begin
    # DESC: Admits fixture transactions only when authority is enabled and records distinct DNS versus TCP states.
    # ------------------=
    def begin(self, mode, host, port):
        if self.denied:
            raise PermissionError(13, 'Denied')
        self.ticket += 1
        self.state = 7 if mode else 2
        self.count = 0
        return self.ticket

    # ------------------------=
    # FUNC: poll
    # DESC: Publishes typed completion state or an injected native errno without manufacturing a successful TLS result.
    # ------------------=
    def poll(self, ticket):
        if self.failure == 'certificate':
            raise self.CertificateError('Certificate verification failed')
        if self.failure is not None:
            raise OSError(self.failure, 'Fixture transport failure')
        return (3 if self.pending else self.state, self.count, bytes((93, 184, 216, 34)))

    # ------------------------=
    # FUNC: command
    # DESC: Records protocol commands and forces partial writes and short reads to test standard wrappers accurately.
    # ------------------=
    def command(self, ticket, operation, data, count):
        self.commands.append((ticket, operation, bytes(data), count))
        self.state = 4
        if operation == 0:
            self.completed = self.incoming[:min(count, 7)]
            self.incoming = self.incoming[len(self.completed):]
            self.count = len(self.completed)
        elif operation == 1:
            self.count = min(count, 3)
            self.writes.append(bytes(data[:self.count]))
        else:
            self.count = 0

    # ------------------------=
    # FUNC: receive
    # DESC: Returns only the actor's completed plaintext bytes, never the requested capacity.
    # ------------------=
    def receive(self, ticket, count):
        return self.completed

    # ------------------------=
    # FUNC: close
    # DESC: Records exact ticket release for ownership and failure cleanup assertions.
    # ------------------=
    def close(self, ticket):
        self.closed.append(ticket)
        self.state = 5

ACTOR = Actor()
sys.modules['_infinity_socket'] = ACTOR

# ------------------------=
# FUNC: load
# DESC: Loads the actual platform adapters and unchanged upstream wrappers for observable API behavior.
# ------------------=
def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module

BACKEND = load('_socket', ROOT / 'sdk/python/socket.py')
SOCKET = load('socket', ROOT / 'third_party/python/Python-3.13.16/Lib/socket.py')
TLS_BACKEND = load('_ssl', ROOT / 'sdk/python/ssl.py')
SSL = load('ssl', ROOT / 'third_party/python/Python-3.13.16/Lib/ssl.py')

class SocketTests(unittest.TestCase):
    # ------------------------=
    # FUNC: setUp
    # DESC: Resets fixture transport state so no test can inherit a successful handshake or retained payload.
    # ------------------=
    def setUp(self):
        ACTOR.__init__()
        BACKEND._handles.clear()

    # ------------------------=
    # FUNC: test_real_wrapper_buffering
    # DESC: Exercises unchanged CPython makefile buffering across short native reads and verifies sendall handles short writes.
    # ------------------=
    def test_real_wrapper_buffering(self):
        with SOCKET.create_connection(('example.test', 443), timeout=1) as sock:
            self.assertEqual(sock.getpeername(), ('93.184.216.34', 443))
            sock.sendall(b'0123456789')
            self.assertEqual(b''.join(ACTOR.writes), b'0123456789')
            ACTOR.incoming = b'first line\nsecond line\n'
            with sock.makefile('rb') as stream:
                self.assertEqual(stream.readline(), b'first line\n')
                self.assertEqual(stream.readline(), b'second line\n')
                self.assertEqual(stream.read(), b'')
        self.assertEqual(len(ACTOR.closed), 2)  # DNS and connected socket.

    # ------------------------=
    # FUNC: test_ssl_wrapper_uses_native_handshake
    # DESC: Verifies standard SSLContext and SSLSocket delegate to the TLS command before plaintext I/O and never duplicate ownership.
    # ------------------=
    def test_ssl_wrapper_uses_native_handshake(self):
        context = SSL.create_default_context()
        self.assertEqual(context.verify_mode, SSL.CERT_REQUIRED)
        self.assertTrue(context.check_hostname)
        raw = SOCKET.create_connection(('example.test', 443), timeout=1)
        with context.wrap_socket(raw, server_hostname='example.test') as secure:
            self.assertEqual(raw.fileno(), -1)
            self.assertEqual(ACTOR.commands[0][1:3], (2, b'example.test'))
            self.assertEqual(secure.version(), 'TLSv1.3')
            secure.sendall(b'abcde')
            self.assertEqual(b''.join(ACTOR.writes), b'abcde')
            ACTOR.incoming = b'content'
            self.assertEqual(secure.recv(100), b'content')
        self.assertEqual(len(ACTOR.closed), 2)

    # ------------------------=
    # FUNC: test_certificate_and_permission_failures
    # DESC: Checks that real native errno classes become certificate or permission exceptions and cannot be weakened through SSL options.
    # ------------------=
    def test_certificate_and_permission_failures(self):
        context = SSL.create_default_context()
        context.check_hostname = False
        self.assertTrue(context.check_hostname)
        with self.assertRaises(ValueError):
            context.verify_mode = SSL.CERT_NONE
        raw = SOCKET.create_connection(('example.test', 443), timeout=1)
        ACTOR.failure = 'certificate'
        with self.assertRaises(SSL.SSLCertVerificationError):
            context.wrap_socket(raw, server_hostname='wrong.test')
        self.assertEqual(len(ACTOR.closed), 2)
        ACTOR.failure = None
        ACTOR.denied = True
        with self.assertRaises(PermissionError):
            SOCKET.create_connection(('example.test', 443))

    # ------------------------=
    # FUNC: test_http11_fallback_does_not_fabricate_alpn
    # DESC: Accepts a client's HTTP/1.1 fallback while exposing no negotiated protocol and rejecting HTTP/2 or mixed ALPN requests.
    # ------------------=
    def test_http11_fallback_does_not_fabricate_alpn(self):
        context = SSL.create_default_context()
        context.set_alpn_protocols(['http/1.1'])
        self.assertFalse(SSL.HAS_ALPN)
        with context.wrap_socket(SOCKET.create_connection(('example.test', 443)),
                                 server_hostname='example.test') as secure:
            self.assertIsNone(secure.selected_alpn_protocol())
        for protocols in (['h2'], ['h2', 'http/1.1'], ['invalid']):
            with self.assertRaises(NotImplementedError):
                context.set_alpn_protocols(protocols)

    # ------------------------=
    # FUNC: test_exact_bundle_selects_isolated_restricted_trust
    # DESC: Verifies exact bundle bytes select a narrower kernel handshake policy, mutation and arbitrary roots fail closed, and another context remains unchanged.
    # ------------------=
    def test_exact_bundle_selects_isolated_restricted_trust(self):
        wheel = ROOT / 'third_party/python/Python-3.13.16/Lib/ensurepip/_bundled/pip-26.2.1-py3-none-any.whl'
        with zipfile.ZipFile(wheel) as bundle:
            pem = bundle.read('pip/_vendor/certifi/cacert.pem')
        context = SSL.create_default_context(cadata=pem.decode('ascii'))
        self.assertEqual(context._trust, 1)
        self.assertEqual(SSL.create_default_context()._trust, 0)
        for bad in (pem[:-1] + bytes((pem[-1] ^ 1,)), b'arbitrary certificates'):
            with self.assertRaises(NotImplementedError):
                context.load_verify_locations(cadata=bad)
            self.assertEqual(context._trust, 1)
        with self.assertRaises(NotImplementedError):
            context.load_verify_locations(capath='/tmp')
        candidate = ROOT / 'build/native-python-trust/socket-test.pem'
        candidate.parent.mkdir(parents=True, exist_ok=True)
        candidate.write_bytes(pem)
        from_file = SSL.create_default_context(cafile=candidate)
        self.assertEqual(from_file._trust, 1)
        candidate.write_bytes(b'changed')
        with self.assertRaises(NotImplementedError):
            SSL.create_default_context(cafile=candidate)
        candidate.unlink()
        for selected, operation in ((context, 3), (SSL.create_default_context(), 2)):
            with selected.wrap_socket(SOCKET.create_connection(('example.test', 443)),
                                      server_hostname='example.test'):
                self.assertEqual(ACTOR.commands[-1][1], operation)

    # ------------------------=
    # FUNC: test_nonblocking_does_not_replay_write
    # DESC: Confirms repeated polling of the same pending operation does not submit or duplicate its bytes.
    # ------------------=
    def test_nonblocking_does_not_replay_write(self):
        with SOCKET.create_connection(('example.test', 443)) as sock:
            sock.setblocking(False)
            ACTOR.pending = True
            with self.assertRaises(BlockingIOError):
                sock.send(b'abc')
            self.assertEqual(len(ACTOR.commands), 1)
            ACTOR.pending = False
            self.assertEqual(sock.send(b'abc'), 3)
            self.assertEqual(len(ACTOR.commands), 1)
            self.assertEqual(b''.join(ACTOR.writes), b'abc')

    # ------------------------=
    # FUNC: test_timeout_cancels_exact_socket
    # DESC: Verifies a deadline closes the actual pending channel instead of claiming an empty successful response.
    # ------------------=
    def test_timeout_cancels_exact_socket(self):
        with SOCKET.create_connection(('example.test', 443)) as sock:
            sock.settimeout(0.001)
            ACTOR.pending = True
            with self.assertRaises(TimeoutError):
                sock.recv(100)
            self.assertIn(sock._state['ticket'], ACTOR.closed)

    # ------------------------=
    # FUNC: test_unsupported_not_reported_available
    # DESC: Rejects server, datagram and memory-BIO operations while preserving honest backend feature metadata.
    # ------------------=
    def test_unsupported_not_reported_available(self):
        self.assertFalse(SSL.HAS_TLSv1_2)
        self.assertFalse(SSL.HAS_ALPN)
        with self.assertRaises(OSError):
            SOCKET.socket(SOCKET.AF_INET, SOCKET.SOCK_DGRAM)
        with self.assertRaises(NotImplementedError):
            SSL.SSLContext(SSL.PROTOCOL_TLS_SERVER)
        with self.assertRaises(NotImplementedError):
            SSL.MemoryBIO()

    # ------------------------=
    # FUNC: test_public_errno_and_certificate_types
    # DESC: Checks target errno contracts and ensures an unrelated numeric error cannot masquerade as certificate verification failure.
    # ------------------=
    def test_public_errno_and_certificate_types(self):
        raw = SOCKET.socket()
        raw.setsockopt(SOCKET.IPPROTO_TCP, SOCKET.TCP_NODELAY, 1)
        self.assertEqual(raw.getsockopt(SOCKET.IPPROTO_TCP, SOCKET.TCP_NODELAY), 1)
        with self.assertRaises(OSError):
            raw.setsockopt(SOCKET.IPPROTO_TCP, SOCKET.TCP_NODELAY, 0)
        with self.assertRaises(OSError) as caught:
            raw.recv(1)
        self.assertEqual(caught.exception.errno, errno.ENOTCONN)
        raw.close()
        with self.assertRaises(SSL.SSLError) as caught:
            TLS_BACKEND._translate(OSError(129, 'Unrelated public errno'))
        self.assertNotIsInstance(caught.exception, SSL.SSLCertVerificationError)
        with self.assertRaises(TimeoutError):
            TLS_BACKEND._translate(TimeoutError(errno.ETIMEDOUT, 'Native deadline'))

if __name__ == '__main__':
    unittest.main()
