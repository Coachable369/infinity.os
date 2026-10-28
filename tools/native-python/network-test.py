#!/usr/bin/env python3
"""Behavioral transport-adapter checks, not native TLS or installed-VM proof."""
import importlib.util
import io
import os
from pathlib import Path
import sys
import types
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]


class Actor:
    class CertificateError(OSError):
        pass
    # ------------------------=
    # FUNC: __init__
    # DESC: Models the typed native bridge contract without opening host sockets.
    # ------------------=
    def __init__(self):
        self.requests = []
        self.closed = []
        self.failure = None
        self.payload = bytes(range(256)) * 300
        self.status = 200
        self.head = b"HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nTransfer-Encoding: chunked\r\n\r\n"
        self.short_region = None
        self.deny_body = False

    # ------------------------=
    # FUNC: begin
    # DESC: Records the marshaled request and returns an opaque generation.
    # ------------------=
    def begin(self, url, headers):
        self.requests.append((url, headers))
        return len(self.requests)

    # ------------------------=
    # FUNC: poll
    # DESC: Returns numeric response metadata or a distinct native failure.
    # ------------------=
    def poll(self, handle):
        if self.failure == 'certificate':
            raise self.CertificateError('Certificate verification failed')
        if self.failure:
            raise OSError(self.failure, "native failure")
        return self.status, len(self.head), len(self.payload)

    # ------------------------=
    # FUNC: read
    # DESC: Preserves the broker's bounded-region read contract.
    # ------------------=
    def read(self, handle, region, offset, count):
        assert count <= 65536
        if self.deny_body and region == 1:
            raise PermissionError(13, 'Authority revoked')
        payload = (self.head if region == 0 else self.payload)[offset:offset + count]
        return payload[:-1] if region == self.short_region else payload

    # ------------------------=
    # FUNC: close
    # DESC: Records exact-handle cancellation and completion cleanup.
    # ------------------=
    def close(self, handle):
        self.closed.append(handle)


class TransportTest(unittest.TestCase):
    # ------------------------=
    # FUNC: setUp
    # DESC: Imports the shipping adapter against a controlled typed actor contract.
    # ------------------=
    def setUp(self):
        self.actor = Actor()
        sys.modules["_infinity_https"] = self.actor
        spec = importlib.util.spec_from_file_location("native_python_https_test", ROOT / "sdk/python/network.py")
        self.network = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.network)

    # ------------------------=
    # FUNC: test_body_is_complete_and_framing_is_normalized
    # DESC: Verifies multi-chunk exact payload transfer and removes already-decoded HTTP framing.
    # ------------------=
    def test_body_is_complete_and_framing_is_normalized(self):
        status, headers, body = self.network.get("https://pypi.org/simple/", [("Accept-Encoding", "gzip")])
        self.assertEqual(status, 200)
        self.assertEqual(body.read(), self.actor.payload)
        self.assertEqual(dict(headers)["Content-Length"], str(len(self.actor.payload)))
        self.assertNotIn("Transfer-Encoding", dict(headers))
        self.assertEqual(self.actor.closed, [1])
        self.assertEqual(self.actor.requests[0][1], b"Accept-Encoding: identity\r\n")

    # ------------------------=
    # FUNC: test_certificate_failure_never_releases_a_body
    # DESC: Ensures certificate rejection propagates as its distinct error and closes the exact request.
    # ------------------=
    def test_certificate_failure_never_releases_a_body(self):
        self.actor.failure = 'certificate'
        with self.assertRaises(self.actor.CertificateError):
            self.network.get("https://pypi.org/simple/")
        self.assertEqual(self.actor.closed, [1])

    # ------------------------=
    # FUNC: test_unsafe_urls_and_headers_never_reach_actor
    # DESC: Rejects TLS downgrade, embedded credentials, unsupported ports and header injection before side effects.
    # ------------------=
    def test_unsafe_urls_and_headers_never_reach_actor(self):
        for url in ("http://pypi.org/simple/", "https://user:secret@pypi.org/simple/", "https://pypi.org:444/"):
            with self.assertRaises(ValueError):
                self.network.get(url)
        with self.assertRaises(ValueError):
            self.network.get("https://pypi.org/", [("X-Header", "ok\r\nBad: 1")])
        self.assertEqual(self.actor.requests, [])

    # ------------------------=
    # FUNC: test_truncated_response_and_revocation_never_release_body
    # DESC: Confirms incomplete headers/body and permissions lost after metadata cause hard failure plus exact transaction cleanup.
    # ------------------=
    def test_truncated_response_and_revocation_never_release_body(self):
        for region in (0, 1):
            self.actor.short_region = region
            with self.assertRaises(OSError):
                self.network.get('https://pypi.org/simple/')
        self.actor.short_region = None
        self.actor.deny_body = True
        with self.assertRaises(PermissionError):
            self.network.get('https://pypi.org/simple/')
        self.assertEqual(self.actor.closed, [1, 2, 3])

    # ------------------------=
    # FUNC: test_upstream_requests_response_and_security_policy
    # DESC: Exercises real upstream PipSession with the native adapter including streamed bytes and strict verification policy.
    # ------------------=
    def test_upstream_requests_response_and_security_policy(self):
        wheel = ROOT / "third_party/python/Python-3.13.16/Lib/ensurepip/_bundled/pip-26.2.1-py3-none-any.whl"
        sys.path.insert(0, str(wheel))
        self.network.configure_pip()
        from pip._internal.network.session import PipSession
        from pip._internal.exceptions import ConnectionFailedError
        from pip._vendor.requests.exceptions import SSLError, ConnectionError
        from pip._vendor.requests import Request
        session = PipSession()
        session.trust_env = False
        response = session.get("https://pypi.org/simple/", stream=True)
        self.assertEqual(b"".join(response.iter_content(16384)), self.actor.payload)
        self.assertEqual(response.status_code, 200)
        self.assertEqual(self.actor.closed, [1])
        with self.assertRaises(SSLError):
            session.get_adapter("https://").send(Request("GET", "https://pypi.org/").prepare(), verify=False)
        with self.assertRaises(ConnectionFailedError):
            session.get("https://pypi.org/", verify=False)
        with self.assertRaises(ConnectionError):
            session.get_adapter("http://").send(Request("GET", "http://pypi.org/").prepare())
        with self.assertRaises(ConnectionFailedError):
            session.get("http://pypi.org/")
        self.assertEqual(len(self.actor.requests), 1)

    # ------------------------=
    # FUNC: test_frozen_pip_child_entry_materialization
    # DESC: Executes the materialized bootstrap against typed module hooks, verifying configuration order, actual upstream entry selection, reuse and cleanup without host pip execution.
    # ------------------=
    def test_frozen_pip_child_entry_materialization(self):
        events = []
        module = types.ModuleType('_infinity_network')
        module.configure_pip = lambda: events.append(('configure',))
        with patch.dict(sys.modules, {'_infinity_network': module}):
            path = Path(self.network._native_pip_runner())
            self.assertTrue(path.is_relative_to(ROOT))
            self.assertEqual(self.network._native_pip_runner(), str(path))
            with patch('runpy.run_module', side_effect=lambda *a, **kw: events.append((a, kw))):
                exec(compile(path.read_bytes(), str(path), 'exec'), {'__name__': '__main__'})
            self.assertEqual(events, [('configure',), (('pip',), {'run_name': '__main__', 'alter_sys': True})])
        self.network._remove_runner(str(path))
        self.assertFalse(path.exists())
        self.network._remove_runner(str(path))


if __name__ == "__main__":
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Use ./build-kit run python3 tools/native-python/network-test.py")
    unittest.main()
