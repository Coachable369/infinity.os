"""IPv4 client sockets backed by InfinityOS's capability-scoped TCP actor.

This is the platform backend for CPython's unchanged socket.py wrapper. Unsupported
server, datagram, descriptor duplication and OS-select operations fail explicitly.
"""
import _infinity_socket as _native
import time as _time
import errno as _errno

AF_UNSPEC = 0
AF_INET = 2
AF_INET6 = 10
SOCK_STREAM = 1
SOCK_DGRAM = 2
IPPROTO_IP = 0
IPPROTO_TCP = 6
IPPROTO_UDP = 17
SOL_SOCKET = 1
SO_TYPE = 3
SO_ERROR = 4
SO_KEEPALIVE = 9
SO_REUSEADDR = 2
TCP_NODELAY = 1
SHUT_RD = 0
SHUT_WR = 1
SHUT_RDWR = 2
AI_PASSIVE = 1
AI_CANONNAME = 2
AI_NUMERICHOST = 4
AI_NUMERICSERV = 1024
NI_NUMERICHOST = 1
NI_NUMERICSERV = 2
EAI_NONAME = -2
EAI_FAMILY = -6
EAI_SOCKTYPE = -7
error = OSError
timeout = TimeoutError
has_ipv6 = False
_default_timeout = None
_next_id = 4096
_handles = {}

class gaierror(OSError):
    pass
class herror(OSError):
    pass

# ------------------------=
# FUNC: _unsupported
# DESC: Rejects unimplemented socket semantics rather than returning a fabricated success.
# ------------------=
def _unsupported(*args, **kwargs):
    raise OSError(_errno.ENOTSUP, "Socket operation is not implemented on InfinityOS")

# ------------------------=
# FUNC: inet_aton
# DESC: Parses a strict numeric IPv4 address without issuing a network request.
# ------------------=
def inet_aton(address):
    parts = address.split('.')
    if len(parts) != 4 or any(not p.isascii() or not p.isdigit() for p in parts):
        raise OSError(_errno.EINVAL, "Invalid IPv4 address")
    values = tuple(int(p) for p in parts)
    if any(v > 255 for v in values):
        raise OSError(_errno.EINVAL, "Invalid IPv4 address")
    return bytes(values)

# ------------------------=
# FUNC: inet_ntoa
# DESC: Formats four actual network-order address bytes.
# ------------------=
def inet_ntoa(address):
    if len(address) != 4:
        raise OSError(_errno.EINVAL, "IPv4 addresses contain four bytes")
    return '.'.join(str(b) for b in address)

# ------------------------=
# FUNC: inet_pton
# DESC: Performs the supported IPv4 text conversion while explicitly rejecting unsupported families.
# ------------------=
def inet_pton(family, address):
    if family != AF_INET:
        raise OSError(_errno.EAFNOSUPPORT, "Only IPv4 is available")
    return inet_aton(address)

# ------------------------=
# FUNC: inet_ntop
# DESC: Formats IPv4 addresses with the same family validation as socket creation.
# ------------------=
def inet_ntop(family, address):
    if family != AF_INET:
        raise OSError(_errno.EAFNOSUPPORT, "Only IPv4 is available")
    return inet_ntoa(address)

# ------------------------=
# FUNC: _wait
# DESC: Polls real native progress, yielding between slices and preserving nonblocking and deadline semantics.
# ------------------=
def _wait(ticket, wanted, seconds):
    deadline = None if seconds is None else _time.monotonic() + seconds
    while True:
        result = _native.poll(ticket)
        if result is not None and result[0] in wanted:
            return result
        if result is not None and result[0] in (5, 6):
            raise ConnectionError(_errno.ECONNRESET, "Native connection closed")
        if seconds == 0:
            raise BlockingIOError(_errno.EAGAIN, "Socket operation is pending")
        if deadline is not None and _time.monotonic() >= deadline:
            _native.close(ticket)
            raise TimeoutError(_errno.ETIMEDOUT, "Socket operation timed out")
        _time.sleep(0.001)

# ------------------------=
# FUNC: gethostbyname
# DESC: Resolves a hostname through authorized native DNS and releases its exact query on all outcomes.
# ------------------=
def gethostbyname(host):
    if isinstance(host, bytes):
        host = host.decode('ascii')
    try:
        return inet_ntoa(inet_aton(host))
    except OSError:
        pass
    name = host.encode('idna')
    ticket = _native.begin(1, name, 53)
    try:
        return inet_ntoa(_wait(ticket, (7,), 30.0)[2])
    finally:
        try:
            _native.close(ticket)
        except OSError:
            pass

# ------------------------=
# FUNC: getaddrinfo
# DESC: Returns a typed IPv4 TCP endpoint from actual DNS or a validated numeric address.
# ------------------=
def getaddrinfo(host, port, family=0, type=0, proto=0, flags=0):
    if family not in (AF_UNSPEC, AF_INET):
        raise gaierror(EAI_FAMILY, "Only IPv4 is available")
    if type not in (0, SOCK_STREAM) or proto not in (0, IPPROTO_TCP):
        raise gaierror(EAI_SOCKTYPE, "Only TCP streams are available")
    if host is None or flags & AI_PASSIVE:
        raise gaierror(EAI_NONAME, "Listening sockets are unavailable")
    if isinstance(port, str) and not port.isdigit():
        port = getservbyname(port)
    port = int(port)
    if not 0 <= port <= 65535:
        raise OverflowError("Port must be between 0 and 65535")
    address = inet_ntoa(inet_aton(host)) if flags & AI_NUMERICHOST else gethostbyname(host)
    return [(AF_INET, SOCK_STREAM, IPPROTO_TCP, str(host) if flags & AI_CANONNAME else '', (address, port))]

# ------------------------=
# FUNC: getservbyname
# DESC: Resolves standard TCP service names without suggesting a nonexistent native service database.
# ------------------=
def getservbyname(name, proto='tcp'):
    if proto != 'tcp' or name not in ('http', 'https', 'ssh'):
        raise OSError(_errno.ENOENT, "Service is not in the native registry")
    return {'http': 80, 'https': 443, 'ssh': 22}[name]

# ------------------------=
# FUNC: getdefaulttimeout
# DESC: Reads the process-local default socket timeout.
# ------------------=
def getdefaulttimeout():
    return _default_timeout

# ------------------------=
# FUNC: setdefaulttimeout
# DESC: Sets future sockets' timeout without changing existing connections.
# ------------------=
def setdefaulttimeout(value):
    global _default_timeout
    if value is not None and value < 0:
        raise ValueError("Timeout cannot be negative")
    _default_timeout = None if value is None else float(value)

class socket:
    __slots__ = ('_id', '_state')

    # ------------------------=
    # FUNC: __init__
    # DESC: Creates a process-local socket descriptor or transfers an existing native channel without connecting implicitly.
    # ------------------=
    def __init__(self, family=AF_INET, type=SOCK_STREAM, proto=0, fileno=None):
        global _next_id
        if fileno is not None:
            if fileno not in _handles:
                raise OSError(_errno.EBADF, "Unknown native socket descriptor")
            self._id, self._state = fileno, _handles[fileno]
            return
        if family != AF_INET or type != SOCK_STREAM or proto not in (0, IPPROTO_TCP):
            raise OSError(_errno.EAFNOSUPPORT, "Only IPv4 TCP client sockets are available")
        self._id = _next_id
        _next_id += 1
        self._state = {'ticket': None, 'peer': None, 'timeout': _default_timeout,
                       'pending': None, 'connected': False, 'family': family, 'type': type, 'proto': proto}
        _handles[self._id] = self._state

    # ------------------------=
    # FUNC: family
    # DESC: Reports the actual admitted address family.
    # ------------------=
    @property
    def family(self):
        return self._state['family']

    # ------------------------=
    # FUNC: type
    # DESC: Reports the actual admitted stream type.
    # ------------------=
    @property
    def type(self):
        return self._state['type']

    # ------------------------=
    # FUNC: proto
    # DESC: Reports the constructor's TCP protocol selector.
    # ------------------=
    @property
    def proto(self):
        return self._state['proto']

    # ------------------------=
    # FUNC: fileno
    # DESC: Returns the process-local opaque socket descriptor, never a kernel pointer or host descriptor.
    # ------------------=
    def fileno(self):
        return self._id

    # ------------------------=
    # FUNC: detach
    # DESC: Transfers descriptor ownership for CPython SSLSocket without closing its native transport.
    # ------------------=
    def detach(self):
        descriptor, self._id = self._id, -1
        return descriptor

    # ------------------------=
    # FUNC: close
    # DESC: Idempotently cancels the exact native channel and erases this descriptor's process-local state.
    # ------------------=
    def close(self):
        if self._id < 0:
            return
        state = _handles.pop(self._id, None)
        self._id = -1
        if state is not None and state['ticket'] is not None:
            try:
                _native.close(state['ticket'])
            except OSError as error:
                if error.errno not in (_errno.EBADF, _errno.EACCES):
                    raise
            state['ticket'] = None
            state['connected'] = False

    # ------------------------=
    # FUNC: _check
    # DESC: Rejects closed or disconnected descriptors before queueing network effects.
    # ------------------=
    def _check(self, connected=True):
        if self._id < 0 or self._id not in _handles:
            raise OSError(_errno.EBADF, "Socket is closed")
        if connected and not self._state['connected']:
            raise OSError(_errno.ENOTCONN, "Socket is not connected")

    # ------------------------=
    # FUNC: connect
    # DESC: Opens and awaits a real authorized TCP connection, retaining pending state for nonblocking retries.
    # ------------------=
    def connect(self, address):
        self._check(False)
        host, port = address
        if self._state['connected']:
            raise OSError(_errno.EISCONN, "Socket is already connected")
        if self._state['ticket'] is None:
            host = host.decode('ascii') if isinstance(host, bytes) else host
            self._state['ticket'] = _native.begin(0, host.encode('idna'), int(port))
            self._state['peer'] = (host, int(port))
        result = _wait(self._state['ticket'], (2,), self._state['timeout'])
        self._state['peer'] = (inet_ntoa(result[2]), int(port))
        self._state['connected'] = True

    # ------------------------=
    # FUNC: connect_ex
    # DESC: Preserves conventional errno-returning connect behavior for clients that do not want exceptions.
    # ------------------=
    def connect_ex(self, address):
        try:
            self.connect(address)
            return 0
        except OSError as error:
            return error.errno or _errno.EIO

    # ------------------------=
    # FUNC: _perform
    # DESC: Buffers one native operation and returns its actual completion without replaying a nonblocking pending write.
    # ------------------=
    def _perform(self, operation, data=b'', count=0):
        self._check()
        state = self._state
        signature = (operation, bytes(data), count)
        if state['pending'] is None:
            _native.command(state['ticket'], operation, data, count)
            state['pending'] = signature
        elif state['pending'] != signature:
            raise BlockingIOError(_errno.EAGAIN, "Another socket operation is pending")
        result = _wait(state['ticket'], (4,), state['timeout'])
        state['pending'] = None
        return _native.receive(state['ticket'], count) if operation == 0 else result[1]

    # ------------------------=
    # FUNC: recv
    # DESC: Returns up to the requested actual stream bytes, preserving short reads and EOF.
    # ------------------=
    def recv(self, size, flags=0):
        if flags or size < 0:
            raise ValueError("Unsupported receive flags or negative size")
        return self._perform(0, count=min(size, 16384))

    # ------------------------=
    # FUNC: recv_into
    # DESC: Copies real received bytes into a writable Python buffer and returns the exact count.
    # ------------------=
    def recv_into(self, buffer, nbytes=0, flags=0):
        view = memoryview(buffer).cast('B')
        size = len(view) if nbytes == 0 else nbytes
        if size < 0 or size > len(view):
            raise ValueError("Receive buffer is too small")
        data = socket.recv(self, size, flags)
        view[:len(data)] = data
        return len(data)

    # ------------------------=
    # FUNC: send
    # DESC: Sends one bounded stream chunk and reports the actual native short-write count.
    # ------------------=
    def send(self, data, flags=0):
        if flags:
            raise ValueError("Unsupported send flags")
        data = bytes(memoryview(data).cast('B')[:16384])
        return self._perform(1, data, len(data))

    # ------------------------=
    # FUNC: sendall
    # DESC: Handles partial writes until all bytes are actually accepted by the stream.
    # ------------------=
    def sendall(self, data, flags=0):
        data = memoryview(data).cast('B')
        while data:
            count = socket.send(self, data, flags)
            if count == 0:
                raise ConnectionError(_errno.EPIPE, "Socket closed during write")
            data = data[count:]

    # ------------------------=
    # FUNC: getpeername
    # DESC: Returns the endpoint of the established connection, not a placeholder address.
    # ------------------=
    def getpeername(self):
        self._check()
        return self._state['peer']

    # ------------------------=
    # FUNC: getsockopt
    # DESC: Exposes real socket type and connected-state error information while rejecting unsupported options.
    # ------------------=
    def getsockopt(self, level, option, *args):
        self._check(False)
        if args:
            return _unsupported()
        if level == SOL_SOCKET and option == SO_TYPE:
            return SOCK_STREAM
        if level == IPPROTO_TCP and option == TCP_NODELAY:
            return 1  # Native TcpStream always disables Nagle before connect.
        if level == SOL_SOCKET and option == SO_ERROR:
            if self._state['ticket'] is not None:
                result = _native.poll(self._state['ticket'])
                if result is None or result[0] in (1, 3):
                    return _errno.EAGAIN
            return 0
        return _unsupported()

    # ------------------------=
    # FUNC: settimeout
    # DESC: Applies a per-operation timeout and preserves the standard blocking/nonblocking distinction.
    # ------------------=
    def settimeout(self, value):
        if value is not None and value < 0:
            raise ValueError("Timeout cannot be negative")
        self._state['timeout'] = None if value is None else float(value)

    # ------------------------=
    # FUNC: gettimeout
    # DESC: Returns this descriptor's configured timeout.
    # ------------------=
    def gettimeout(self):
        return self._state['timeout']

    # ------------------------=
    # FUNC: setblocking
    # DESC: Selects blocking or nonblocking operation through the same native pending-state machine.
    # ------------------=
    def setblocking(self, value):
        self.settimeout(None if value else 0.0)

    # ------------------------=
    # FUNC: getblocking
    # DESC: Reports whether operations wait for native completion.
    # ------------------=
    def getblocking(self):
        return self.gettimeout() != 0

    # ------------------------=
    # FUNC: setsockopt
    # DESC: Acknowledges the native transport's already-enforced no-delay policy; unsupported options and attempts to enable Nagle remain explicit errors.
    # ------------------=
    def setsockopt(self, level, option, value, *args):
        self._check(False)
        if not args and level == IPPROTO_TCP and option == TCP_NODELAY and value == 1:
            return None
        raise OSError(_errno.ENOPROTOOPT, 'Native socket option is not supported')

    bind = listen = _accept = getsockname = shutdown = _unsupported

SocketType = socket
dup = socketpair = gethostname = gethostbyaddr = getnameinfo = _unsupported

# ------------------------=
# FUNC: close
# DESC: Closes an opaque native socket descriptor without invoking host file-descriptor APIs.
# ------------------=
def close(descriptor):
    socket(fileno=descriptor).close()

# ------------------------=
# FUNC: ntohs
# DESC: Converts a 16-bit network-order integer on InfinityOS's supported little-endian native targets.
# ------------------=
def ntohs(value):
    return int.from_bytes(value.to_bytes(2, 'little'), 'big')

# ------------------------=
# FUNC: ntohl
# DESC: Converts a 32-bit network-order integer on InfinityOS's supported little-endian native targets.
# ------------------=
def ntohl(value):
    return int.from_bytes(value.to_bytes(4, 'little'), 'big')

htons = ntohs
htonl = ntohl
