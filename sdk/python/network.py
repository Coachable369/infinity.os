"""Native authenticated HTTPS transport for the upstream pip resolver.

This adapter does not pretend to implement Python's socket or SSL APIs. The
kernel TLS service validates the chain, hostname, validity dates and handshake
using its installed trust roots before releasing any response bytes.
"""
import io
import errno
import time
from urllib.parse import urlsplit
import _infinity_https

_runner_path = None


# ------------------------=
# FUNC: _remove_runner
# DESC: Removes the process-private pip entry file on normal exit; native transaction teardown also reclaims it after cancellation.
# ------------------=
def _remove_runner(path):
    import os
    try:
        os.unlink(path)
    except FileNotFoundError:
        pass


# ------------------------=
# FUNC: _native_pip_runner
# DESC: Materializes one real Python entry file for upstream build-dependency subprocesses; execution still uses the isolated native child Task and frozen upstream pip.
# ------------------=
def _native_pip_runner():
    global _runner_path
    if _runner_path is None:
        import os
        import tempfile
        import atexit
        descriptor, path = tempfile.mkstemp(prefix='native-pip-entry-', suffix='.py')
        try:
            with os.fdopen(descriptor, 'w', encoding='utf-8') as stream:
                stream.write("import runpy\nfrom _infinity_network import configure_pip\n"
                    "configure_pip()\nrunpy.run_module('pip', run_name='__main__', alter_sys=True)\n")
        except BaseException:
            _remove_runner(path)
            raise
        _runner_path = path
        atexit.register(_remove_runner, path)
    return _runner_path


# ------------------------=
# FUNC: get
# DESC: Performs one bounded HTTPS GET through the native capability broker, preserving certificate failures and always releasing the response.
# ------------------=
def get(url, headers=(), timeout=30):
    parsed = urlsplit(url)
    if parsed.scheme != "https" or not parsed.hostname or parsed.username or parsed.password or parsed.port not in (None, 443):
        raise ValueError("Only authenticated HTTPS on port 443 is available")
    # Request framing and content decoding belong to the native actor. Caller
    # input cannot override Host, inject hop-by-hop fields, or request gzip.
    outgoing = []
    for name, value in headers:
        if name.lower() in ("host", "connection", "transfer-encoding", "content-length", "accept-encoding"):
            continue
        if any(c in name + value for c in "\r\n\0"):
            raise ValueError("Invalid HTTPS header")
        outgoing.append(f"{name}: {value}\r\n")
    outgoing.append("Accept-Encoding: identity\r\n")
    encoded = "".join(outgoing).encode("latin-1")
    deadline = time.monotonic() + min(max(float(timeout), 0.01), 30)
    handle = None
    try:
        while handle is None:
            try:
                handle = _infinity_https.begin(url.encode("ascii"), encoded)
            except OSError as error:
                if error.errno != errno.EAGAIN:
                    raise
                if time.monotonic() >= deadline:
                    raise TimeoutError("Native HTTPS service is busy") from error
                time.sleep(0.01)
        while True:
            metadata = _infinity_https.poll(handle)
            if metadata is not None:
                break
            if time.monotonic() >= deadline:
                raise TimeoutError("Native HTTPS request timed out")
            time.sleep(0.01)
        status, head_size, body_size = metadata
        head = _infinity_https.read(handle, 0, 0, head_size)
        if len(head) != head_size:
            raise OSError('Truncated native HTTPS response header')
        body = io.BytesIO()
        for offset in range(0, body_size, 65536):
            expected = min(65536, body_size - offset)
            part = _infinity_https.read(handle, 1, offset, expected)
            if len(part) != expected:
                raise OSError("Truncated native HTTPS response")
            body.write(part)
        body.seek(0)
        fields = []
        for line in head.decode("latin-1").split("\r\n")[1:]:
            if not line:
                break
            name, separator, value = line.partition(":")
            if not separator:
                raise OSError("Malformed native HTTPS response header")
            if name.lower() not in ("transfer-encoding", "content-length"):
                fields.append((name, value.strip()))
        # Chunk framing was removed by the authenticated native HTTP decoder.
        fields.append(("Content-Length", str(body_size)))
        return status, fields, body
    finally:
        if handle is not None:
            _infinity_https.close(handle)


# ------------------------=
# FUNC: configure_pip
# DESC: Mounts the native HTTPS adapter on real upstream PipSession instances without changing resolution, wheel hashes or install semantics.
# ------------------=
def configure_pip():
    from pip._vendor.requests.adapters import BaseAdapter
    from pip._vendor.requests.models import Response
    from pip._vendor.requests.structures import CaseInsensitiveDict
    from pip._vendor.requests.exceptions import ConnectionError, SSLError, Timeout
    from pip._vendor.urllib3.response import HTTPResponse
    from pip._vendor.urllib3._collections import HTTPHeaderDict
    from pip._internal.network.session import PipSession
    from pip._internal.cli import index_command
    from pip._internal.build_env import installer

    class NativeHTTPSAdapter(BaseAdapter):
        # ------------------------=
        # FUNC: send
        # DESC: Uses verified native TLS for one resolver/download request and lets upstream Requests own redirect and cookie processing.
        # ------------------=
        def send(self, request, stream=False, timeout=None, verify=True, cert=None, proxies=None):
            if verify is not True or cert or proxies:
                raise SSLError("Native HTTPS requires system trust verification; custom certificates, proxies and insecure overrides are unsupported", request=request)
            if request.method != "GET" or request.body:
                raise ConnectionError("The native package transport permits only HTTPS GET", request=request)
            duration = timeout[1] if isinstance(timeout, tuple) else timeout
            try:
                status, headers, body = get(request.url, request.headers.items(), duration or 30)
            except TimeoutError as error:
                raise Timeout(str(error), request=request) from error
            except _infinity_https.CertificateError as error:
                raise SSLError(str(error), request=request) from error
            except OSError as error:
                raise ConnectionError(str(error), request=request) from error
            except ValueError as error:
                raise ConnectionError(str(error), request=request) from error
            raw_headers = HTTPHeaderDict()
            for name, value in headers:
                raw_headers.add(name, value)
            response = Response()
            response.status_code = status
            response.headers = CaseInsensitiveDict(raw_headers)
            response.raw = HTTPResponse(body=body, headers=raw_headers, status=status,
                preload_content=False, decode_content=False)
            response.url = request.url
            response.request = request
            response.connection = self
            return response

        # ------------------------=
        # FUNC: close
        # DESC: Keeps no ambient socket or kernel response loan between Requests calls.
        # ------------------=
        def close(self):
            pass

    original_init = PipSession.__init__

    # ------------------------=
    # FUNC: initialize
    # DESC: Installs the secure adapter after upstream PipSession establishes its ordinary local-file and security policy.
    # ------------------=
    def initialize(self, *args, **kwargs):
        original_init(self, *args, **kwargs)
        self.mount("https://", NativeHTTPSAdapter())
        # A redirect may not escape authenticated TLS. File:// remains upstream's
        # local wheel adapter; HTTP always reaches the native adapter and fails.
        self.mount("http://", NativeHTTPSAdapter())

    # ------------------------=
    # FUNC: native_trust_context
    # DESC: Leaves trust-root ownership in the kernel actor instead of creating an unused OpenSSL context or claiming an SSL socket module exists.
    # ------------------=
    def native_trust_context():
        return None

    PipSession.__init__ = initialize
    index_command._create_truststore_ssl_context = native_trust_context
    # Frozen pip has no adjacent on-disk __pip-runner__.py. Keep upstream's
    # subprocess build-dependency installer, supplying a real private entry file.
    installer.get_runnable_pip = _native_pip_runner
