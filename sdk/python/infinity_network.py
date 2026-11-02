"""IPv4 TCP clients backed by InfinityOS socket and verified TLS adapters."""
import socket
import ssl

# ------------------------=
# FUNC: connect
# DESC: Opens a finite-timeout client stream and optionally authenticates TLS using the system trust policy.
# ------------------=
def connect(host, port, timeout=10.0, tls=False):
    if not isinstance(host, str) or not host or len(host.encode("ascii")) > 253:
        raise ValueError("Invalid hostname")
    if type(port) is not int or not 1 <= port <= 65535:
        raise ValueError("Invalid port")
    if not 0 < timeout <= 120:
        raise ValueError("Timeout must be between zero and 120 seconds")
    stream = socket.create_connection((host, port), timeout)
    try:
        return ssl.create_default_context().wrap_socket(stream, server_hostname=host) if tls else stream
    except BaseException:
        stream.close()
        raise
