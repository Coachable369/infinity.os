"""Import-compatible socket boundary for the offline native Python profile.

No network authority is granted to this process. Constants describe the usual
protocol identifiers; every operation which would acquire a socket fails.
"""
import errno

AF_UNSPEC = 0
AF_INET = 2
AF_INET6 = 10
SOCK_STREAM = 1
SOCK_DGRAM = 2
SOL_SOCKET = 1
SO_KEEPALIVE = 9
SO_REUSEADDR = 2
IPPROTO_TCP = 6
IPPROTO_IPV6 = 41
TCP_NODELAY = 1
IPV6_V6ONLY = 26
AI_PASSIVE = 1
AI_NUMERICHOST = 4
AI_ADDRCONFIG = 32
EAI_NONAME = -2
SHUT_RDWR = 2
has_ipv6 = False
error = OSError
timeout = TimeoutError
_GLOBAL_DEFAULT_TIMEOUT = object()


class gaierror(OSError):
    """Name resolution is unavailable to this execution profile."""


class herror(OSError):
    """Host lookup is unavailable to this execution profile."""


# ------------------------=
# FUNC: unavailable
# DESC: Rejects an unavailable network operation without creating any resource.
# ------------------=
def unavailable(*args, **kwargs):
    raise OSError(errno.ENOTSUP, "Network access is unavailable in native Python's offline package profile")


class socket:
    # ------------------------=
    # FUNC: __new__
    # DESC: Refuses socket construction instead of returning a simulated socket.
    # ------------------=
    def __new__(cls, *args, **kwargs):
        return unavailable()


SocketType = socket
create_connection = create_server = socketpair = fromfd = unavailable
getaddrinfo = getnameinfo = gethostbyname = gethostbyaddr = unavailable
gethostname = getfqdn = getservbyname = getprotobyname = unavailable
setdefaulttimeout = unavailable
inet_aton = inet_ntoa = inet_pton = inet_ntop = unavailable


# ------------------------=
# FUNC: getdefaulttimeout
# DESC: Reports the absence of a configured default timeout, without opening a socket.
# ------------------=
def getdefaulttimeout():
    return None
