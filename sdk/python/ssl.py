"""CPython ssl.py platform backend using InfinityOS's real TLS 1.3 service.

System trust, DNS hostname and certificate validity are mandatory. The exact
certifi 2026.06.17 bundle selects a verified, narrower native root intersection.
This is not OpenSSL: server TLS, arbitrary custom roots, client certificates, negotiated ALPN, MemoryBIO and TLS
versions below 1.3 are explicitly unsupported, not silently emulated.
"""
import os as _os
import errno as _errno
import hashlib as _hashlib
import _infinity_socket as _native
from _socket import socket as _socket

OPENSSL_VERSION = 'InfinityOS embedded-tls 0.19.0 (not OpenSSL)'
OPENSSL_VERSION_NUMBER = 0
OPENSSL_VERSION_INFO = (0, 0, 0, 0, 0)
_OPENSSL_API_VERSION = (0, 0, 0, 0, 0)
_DEFAULT_CIPHERS = 'TLS_AES_128_GCM_SHA256'
HAS_SNI = HAS_ECDH = HAS_TLSv1_3 = True
HAS_NPN = HAS_ALPN = HAS_SSLv2 = HAS_SSLv3 = False
HAS_TLSv1 = HAS_TLSv1_1 = HAS_TLSv1_2 = HAS_PSK = False
PROTOCOL_TLS = 2
PROTOCOL_TLS_CLIENT = 16
PROTOCOL_TLS_SERVER = 17
PROTO_MINIMUM_SUPPORTED = -2
PROTO_MAXIMUM_SUPPORTED = -1
PROTO_SSLv3 = 0x300
PROTO_TLSv1 = 0x301
PROTO_TLSv1_1 = 0x302
PROTO_TLSv1_2 = 0x303
PROTO_TLSv1_3 = 0x304
CERT_NONE = 0
CERT_OPTIONAL = 1
CERT_REQUIRED = 2
VERIFY_DEFAULT = 0
VERIFY_X509_STRICT = 0x20
VERIFY_X509_TRUSTED_FIRST = 0x8000
VERIFY_X509_PARTIAL_CHAIN = 0x80000
OP_NO_SSLv2 = 0
OP_NO_SSLv3 = 0x02000000
OP_NO_TLSv1 = 0x04000000
OP_NO_TLSv1_1 = 0x10000000
OP_NO_TLSv1_2 = 0x08000000
OP_NO_TLSv1_3 = 0x20000000
OP_NO_COMPRESSION = 0x20000
OP_NO_TICKET = 0x4000
OP_NO_RENEGOTIATION = 0x40000000
OP_CIPHER_SERVER_PREFERENCE = 0x400000
OP_SINGLE_DH_USE = OP_SINGLE_ECDH_USE = 0
OP_ALL = 0
SSL_ERROR_SSL = 1
SSL_ERROR_WANT_READ = 2
SSL_ERROR_WANT_WRITE = 3
SSL_ERROR_WANT_X509_LOOKUP = 4
SSL_ERROR_SYSCALL = 5
SSL_ERROR_ZERO_RETURN = 6
SSL_ERROR_WANT_CONNECT = 7
SSL_ERROR_EOF = 8
SSL_ERROR_INVALID_ERROR_CODE = 10
ALERT_DESCRIPTION_CLOSE_NOTIFY = 0
ALERT_DESCRIPTION_HANDSHAKE_FAILURE = 40
ALERT_DESCRIPTION_BAD_CERTIFICATE = 42
ALERT_DESCRIPTION_CERTIFICATE_EXPIRED = 45
ALERT_DESCRIPTION_UNKNOWN_CA = 48
ALERT_DESCRIPTION_PROTOCOL_VERSION = 70
ENCODING_DER = 2
HOSTFLAG_NEVER_CHECK_SUBJECT = 0x20
_CERTIFI_SHA256 = bytes((187,199,233,192,29,117,81,187,138,21,155,93,237,217,137,184,238,60,225,5,175,245,34,182,142,177,176,27,248,84,202,176))

class SSLError(OSError):
    library = 'InfinityOS TLS'
    reason = None
class SSLZeroReturnError(SSLError):
    pass
class SSLWantReadError(SSLError):
    pass
class SSLWantWriteError(SSLError):
    pass
class SSLSyscallError(SSLError):
    pass
class SSLEOFError(SSLError):
    pass
class SSLCertVerificationError(SSLError):
    verify_code = 1
    verify_message = 'Native certificate chain, hostname or validity verification failed'

# ------------------------=
# FUNC: _unsupported
# DESC: Rejects TLS features not provided by the native service instead of returning compatibility placeholders.
# ------------------=
def _unsupported(*args, **kwargs):
    raise NotImplementedError('This TLS operation is not supported by the InfinityOS client backend')

# ------------------------=
# FUNC: _translate
# DESC: Preserves certificate rejection and nonblocking states as the standard SSL exception families.
# ------------------=
def _translate(error, writing=False):
    if isinstance(error, _native.CertificateError):
        raise SSLCertVerificationError(SSL_ERROR_SSL, SSLCertVerificationError.verify_message) from error
    if error.errno == _errno.EAGAIN:
        cls = SSLWantWriteError if writing else SSLWantReadError
        raise cls(SSL_ERROR_WANT_WRITE if writing else SSL_ERROR_WANT_READ, 'TLS operation pending') from error
    if isinstance(error, (TimeoutError, PermissionError)):
        raise error
    raise SSLError(SSL_ERROR_SYSCALL, str(error)) from error

class MemoryBIO:
    __new__ = staticmethod(_unsupported)
class SSLSession:
    __new__ = staticmethod(_unsupported)

class _SSLContext:
    # ------------------------=
    # FUNC: __new__
    # DESC: Creates a real client TLS policy bound to strict system roots and TLS 1.3, never a server or insecure context.
    # ------------------=
    def __new__(cls, protocol=PROTOCOL_TLS_CLIENT):
        if protocol not in (PROTOCOL_TLS, PROTOCOL_TLS_CLIENT):
            raise NotImplementedError('InfinityOS currently supports client TLS 1.3 only')
        self = super().__new__(cls)
        self._protocol = protocol
        self._trust = 0
        self._options = OP_NO_SSLv3 | OP_NO_TLSv1 | OP_NO_TLSv1_1 | OP_NO_TLSv1_2 | OP_NO_COMPRESSION | OP_NO_RENEGOTIATION | OP_NO_TICKET
        self._verify_flags = VERIFY_X509_STRICT | VERIFY_X509_TRUSTED_FIRST | VERIFY_X509_PARTIAL_CHAIN
        self.post_handshake_auth = None
        return self

    # ------------------------=
    # FUNC: protocol
    # DESC: Reports the selected native client policy.
    # ------------------=
    @property
    def protocol(self):
        return self._protocol

    # ------------------------=
    # FUNC: check_hostname
    # DESC: Reports the always-enforced native DNS identity check.
    # ------------------=
    @property
    def check_hostname(self):
        return True

    # ------------------------=
    # FUNC: check_hostname
    # DESC: Retains mandatory native hostname checks even when a caller offers to perform its own additional check.
    # ------------------=
    @check_hostname.setter
    def check_hostname(self, value):
        # Some HTTP clients request caller-side verification. Native security
        # cannot be weakened: the getter still reports True and the service
        # always verifies the SAN before releasing any application bytes.
        bool(value)

    # ------------------------=
    # FUNC: _host_flags
    # DESC: Reports that the real native verifier checks subject alternative names without a legacy common-name fallback.
    # ------------------=
    @property
    def _host_flags(self):
        return HOSTFLAG_NEVER_CHECK_SUBJECT

    # ------------------------=
    # FUNC: _host_flags
    # DESC: Refuses to advertise a legacy common-name fallback that the native certificate verifier does not implement.
    # ------------------=
    @_host_flags.setter
    def _host_flags(self, value):
        if value != HOSTFLAG_NEVER_CHECK_SUBJECT:
            raise ValueError('InfinityOS requires certificate subject alternative names')

    # ------------------------=
    # FUNC: verify_mode
    # DESC: Exposes mandatory chain verification as CERT_REQUIRED.
    # ------------------=
    @property
    def verify_mode(self):
        return CERT_REQUIRED

    # ------------------------=
    # FUNC: verify_mode
    # DESC: Rejects attempts to weaken the native certificate policy.
    # ------------------=
    @verify_mode.setter
    def verify_mode(self, value):
        if value != CERT_REQUIRED:
            raise ValueError('InfinityOS requires verified TLS certificates')

    # ------------------------=
    # FUNC: verify_flags
    # DESC: Reports strict RFC certificate validation and trusted-anchor chain termination used by webpki.
    # ------------------=
    @property
    def verify_flags(self):
        return self._verify_flags

    # ------------------------=
    # FUNC: verify_flags
    # DESC: Admits only flags whose semantics are actually enforced by the native verifier.
    # ------------------=
    @verify_flags.setter
    def verify_flags(self, value):
        supported = VERIFY_X509_STRICT | VERIFY_X509_TRUSTED_FIRST | VERIFY_X509_PARTIAL_CHAIN
        if value & ~supported:
            raise NotImplementedError('Requested certificate verification policy is unavailable')
        self._verify_flags = supported

    # ------------------------=
    # FUNC: options
    # DESC: Reports protocols and compression that the native service actually disables.
    # ------------------=
    @property
    def options(self):
        return self._options

    # ------------------------=
    # FUNC: options
    # DESC: Rejects disabling the only supported TLS version while preserving immutable native security restrictions.
    # ------------------=
    @options.setter
    def options(self, value):
        if value & OP_NO_TLSv1_3 or value & ~(self._options | OP_CIPHER_SERVER_PREFERENCE):
            raise ValueError('Requested TLS options are unsupported')
        self._options |= value

    # ------------------------=
    # FUNC: minimum_version
    # DESC: Reports the actual TLS 1.3 minimum.
    # ------------------=
    @property
    def minimum_version(self):
        return PROTO_TLSv1_3

    # ------------------------=
    # FUNC: minimum_version
    # DESC: Accepts a less restrictive minimum without implying older protocols can be negotiated.
    # ------------------=
    @minimum_version.setter
    def minimum_version(self, value):
        if value != PROTO_MINIMUM_SUPPORTED and value not in (PROTO_TLSv1, PROTO_TLSv1_1, PROTO_TLSv1_2, PROTO_TLSv1_3):
            raise ValueError('Requested TLS minimum version is unsupported')

    # ------------------------=
    # FUNC: maximum_version
    # DESC: Reports the actual TLS 1.3 maximum.
    # ------------------=
    @property
    def maximum_version(self):
        return PROTO_TLSv1_3

    # ------------------------=
    # FUNC: maximum_version
    # DESC: Rejects version ceilings that cannot produce a secure native handshake.
    # ------------------=
    @maximum_version.setter
    def maximum_version(self, value):
        if value not in (PROTO_MAXIMUM_SUPPORTED, PROTO_TLSv1_3):
            raise ValueError('InfinityOS requires TLS 1.3')

    # ------------------------=
    # FUNC: set_default_verify_paths
    # DESC: Selects the native service's installed Mozilla trust-anchor set; no mutable user trust store is read.
    # ------------------=
    def set_default_verify_paths(self):
        self._trust = 0
        return None

    # ------------------------=
    # FUNC: load_verify_locations
    # DESC: Selects only the exact pinned certifi trust intersection by content, rejecting changed or arbitrary bundles and directories without broadening trust.
    # ------------------=
    def load_verify_locations(self, cafile=None, capath=None, cadata=None):
        if capath is not None or (cafile is None) == (cadata is None):
            raise NotImplementedError('Only the exact supported certifi PEM bundle is available')
        if cafile is not None:
            with open(_os.fspath(cafile), 'rb') as stream:
                content = stream.read(512 * 1024 + 1)
        elif isinstance(cadata, str):
            content = cadata.encode('ascii')
        else:
            content = bytes(cadata)
        if len(content) > 512 * 1024 or _hashlib.sha256(content).digest() != _CERTIFI_SHA256:
            raise NotImplementedError('This custom trust bundle is not supported by native TLS')
        self._trust = 1

    # ------------------------=
    # FUNC: set_ciphers
    # DESC: Accepts only the cipher suite implemented and negotiated by the real TLS service.
    # ------------------=
    def set_ciphers(self, ciphers):
        if ciphers != _DEFAULT_CIPHERS:
            raise NotImplementedError('Only TLS_AES_128_GCM_SHA256 is implemented')

    # ------------------------=
    # FUNC: _wrap_socket
    # DESC: Attaches a verified TLS state machine to a real native stream without issuing HTTP or bypassing socket semantics.
    # ------------------=
    def _wrap_socket(self, sock, server_side, server_hostname, owner=None, session=None):
        if server_side or session is not None:
            raise NotImplementedError('Server TLS and session resumption are unavailable')
        if not server_hostname:
            raise ValueError('A verified server hostname is required')
        return _SSLSocket(self, sock, server_hostname)

    # ------------------------=
    # FUNC: _set_alpn_protocols
    # DESC: Allows only HTTP/1.1 fallback without advertising or fabricating ALPN negotiation; rejects protocols that require negotiation.
    # ------------------=
    def _set_alpn_protocols(self, protocols):
        if bytes(protocols) not in (b'', b'\x08http/1.1'):
            raise NotImplementedError('Native TLS supports HTTP/1.1 fallback, not ALPN negotiation')

    load_cert_chain = set_default_verify_paths_custom = _unsupported
    _wrap_bio = _set_npn_protocols = cert_store_stats = get_ca_certs = _unsupported

class _SSLSocket:
    # ------------------------=
    # FUNC: __init__
    # DESC: Retains the real socket and requested verified hostname until handshake completion.
    # ------------------=
    def __init__(self, context, sock, hostname):
        self.context, self._sock, self.server_hostname = context, sock, hostname
        self.owner, self.server_side = sock, False
        self._ready = False
        self._trust = context._trust
        self.session, self.session_reused = None, False

    # ------------------------=
    # FUNC: do_handshake
    # DESC: Performs the actual native certificate-validated handshake and marks readiness only after successful completion.
    # ------------------=
    def do_handshake(self):
        if self._ready:
            return
        name = self.server_hostname.encode('idna')
        try:
            self._sock._perform(3 if self._trust else 2, name, len(name))
        except OSError as error:
            _translate(error)
        self._ready = True

    # ------------------------=
    # FUNC: read
    # DESC: Returns real decrypted stream bytes or fills the supplied writable buffer with an exact count.
    # ------------------=
    def read(self, size=1024, buffer=None):
        self.do_handshake()
        try:
            if buffer is not None:
                return _socket.recv_into(self._sock, buffer, size)
            return _socket.recv(self._sock, size)
        except OSError as error:
            _translate(error)

    # ------------------------=
    # FUNC: write
    # DESC: Encrypts and sends a real bounded socket write, preserving short-write and retry semantics.
    # ------------------=
    def write(self, data):
        self.do_handshake()
        try:
            return _socket.send(self._sock, data)
        except OSError as error:
            _translate(error, True)

    # ------------------------=
    # FUNC: version
    # DESC: Reports TLS 1.3 only after an authenticated handshake actually succeeds.
    # ------------------=
    def version(self):
        return 'TLSv1.3' if self._ready else None

    # ------------------------=
    # FUNC: cipher
    # DESC: Reports the only cipher suite the native TLS 1.3 state machine can negotiate after verified success.
    # ------------------=
    def cipher(self):
        return ('TLS_AES_128_GCM_SHA256', 'TLSv1.3', 128) if self._ready else None

    # ------------------------=
    # FUNC: compression
    # DESC: Reports the absence of TLS compression in the actual native protocol implementation.
    # ------------------=
    def compression(self):
        return None

    # ------------------------=
    # FUNC: selected_alpn_protocol
    # DESC: Reports no negotiated ALPN because the backend does not advertise ALPN support.
    # ------------------=
    def selected_alpn_protocol(self):
        return None

    # ------------------------=
    # FUNC: shared_ciphers
    # DESC: Preserves standard client behavior, which does not expose the server's shared-cipher list.
    # ------------------=
    def shared_ciphers(self):
        return None

    getpeercert = get_verified_chain = get_unverified_chain = get_channel_binding = _unsupported
    pending = shutdown = verify_client_post_handshake = _unsupported

# ------------------------=
# FUNC: txt2obj
# DESC: Exposes the two actual extended-key-usage OIDs required by the standard SSL purpose enum.
# ------------------=
def txt2obj(value, name=False):
    values = ((129, 'serverAuth', 'TLS Web Server Authentication', '1.3.6.1.5.5.7.3.1'),
              (130, 'clientAuth', 'TLS Web Client Authentication', '1.3.6.1.5.5.7.3.2'))
    for row in values:
        if value == row[3] or name and value in row[1:3]:
            return row
    raise ValueError('Unknown certificate purpose')

# ------------------------=
# FUNC: nid2obj
# DESC: Maps the supported standard certificate-purpose identifiers to their real OIDs.
# ------------------=
def nid2obj(value):
    if value not in (129, 130):
        raise ValueError('Unknown certificate purpose')
    return txt2obj('1.3.6.1.5.5.7.3.' + ('1' if value == 129 else '2'))

# ------------------------=
# FUNC: RAND_bytes
# DESC: Uses the interpreter's existing native boot-entropy syscall rather than a pseudo-random fallback.
# ------------------=
def RAND_bytes(count):
    return _os.urandom(count)

# ------------------------=
# FUNC: RAND_status
# DESC: Verifies that native entropy is actually available before reporting readiness.
# ------------------=
def RAND_status():
    try:
        return len(_os.urandom(1)) == 1
    except OSError:
        return False

RAND_add = _unsupported

# ------------------------=
# FUNC: get_default_verify_paths
# DESC: Reports that trust is managed by the native service, not an OpenSSL CA file or mutable directory.
# ------------------=
def get_default_verify_paths():
    return ('INFINITY_TLS_CA_FILE', '', 'INFINITY_TLS_CA_PATH', '')
