# NetworkKit — native TCP/IP clients

Python and C share InfinityOS's capability-scoped IPv4 DNS/TCP/TLS actor. There
is no host-network proxy or new network framework. Supported: outbound TCP,
DNS resolution, bounded reads/writes and TLS with certificate/hostname checking.
Unsupported: UDP, IPv6, listeners/accept, Unix sockets and disabled TLS verification.
The current bridge supports one active channel per application invocation.

## Quick start

Copy `../Examples/Python/network_head.py` to Documents. In Command Window run
`python --allow-network /home/YOUR_USER/documents/network_head.py` with the actual
saved path. Enter a hostname you are authorized to contact, such as example.com.
The sample opens verified TLS on port 443, sends HTTP HEAD and displays up to
1024 response bytes. This demonstrates TCP/TLS, not a full HTTP client/parser.

For C, copy `../Examples/C/network_head.c`, compile with the native `cc` command
or editor, then launch the resulting ELF using
`run --allow-network /absolute/path/to/program.elf`. Use the actual output path
reported by the compiler. Ordinary editor Run and ordinary ELF launch do not
grant network access. A denied/offline network profile remains denied even with
the launch flag. Applications and AI-generated code cannot self-grant permission.

## Python reference

`from infinity_network import connect`

`connect(host, port, timeout=10.0, tls=False)` returns the native socket or a
verified TLS wrapper. Use `with` to close on all paths. Host is an ASCII hostname
or IPv4 literal of 1–253 bytes, port 1–65535, timeout >0 and <=120 seconds.
TLS uses system trust with the supplied hostname; handshake failure closes the
underlying socket and propagates the exception. Native socket operations support
`send`, `sendall`, `recv`, `recv_into`, timeout and close. Reads may be short; empty
bytes mean EOF. `send` can be short; `sendall` handles that loop. Timeouts,
permission denial and certificate failures remain errors, never fake responses.

## C reference

Include `<infinity/network.h>`. All calls return a nonnegative result on success
or negative ABI error. The header wraps existing native services:

| API | Behavior |
| --- | --- |
| `infinity_network_open(host,length,port,resolve_only)` | Begin connection (0) or DNS-only query (1); returns handle |
| `infinity_network_command(handle,operation,bytes,count)` | Queue read (0), write (1), or verified TLS upgrade (2) |
| `infinity_network_poll(handle,&status)` | Read state/count/address; -11 means pending |
| `infinity_network_receive(handle,buffer,capacity)` | Copy completed read bytes, returning actual count |
| `infinity_network_close(handle)` | Cancel and release this generation-fenced channel |

Commands/read copies are capped at 16384 bytes. TLS command bytes are the hostname,
1–253 bytes, not a URL. Poll for CONNECTED (2) after open; COMPLETE (4) after a
command; RESOLVED (7) for DNS-only. CLOSED (5) and FAILED (6) are terminal. Check
negative results, yield while pending, bound retries and dispatch UI events in GUI
apps. Only issue a new command after completion. After a short write advance the
offset by `status.count` and queue the remainder. A completed read count of zero
means EOF. Do not interpret a successful queue call as completed network I/O.

The example caps polling rather than promising a wall-clock timeout; the native
actor also has transport deadlines. GUI apps should use this asynchronous API
from their event loop, not block the interface in the example's terminal loop.

## Security, lifecycle and troubleshooting

Capabilities are checked against the active session and current network profile;
revocation/profile changes stop access. Close on success, error and cancellation.
The OS also closes the channel at app exit. Common negative C errors: -13 denied,
-11 pending/busy, -129 certificate failure, -110 timeout, -100 unavailable,
-113 resolution, -22 invalid request, -9 stale/closed handle. Do not retry a bad
certificate by turning off verification. Check trusted system time and root
certificates. Never send credentials over plaintext TCP.

SDK tests exercise public wrapper arguments, rejection and error propagation;
existing transport tests cover the native socket/TLS state machine. An external
website's availability and installed NIC behavior require separate runtime tests.

## Native SSH terminal

Command Window supports `ssh user@host [-p port]` for an outbound, password-authenticated
line-mode shell. Verify the Ed25519 host fingerprint, approve with Y, then use
the private password prompt. Escape disconnects. This is a local user terminal
command, not a Python/C SSH API or permission for generated apps to handle credentials.
It does not support full-screen VT applications, SFTP, forwarding, or remote key
authentication. Host trust is confirmed per connection.

Inbound SSH accepts the existing native account password or a registered key.
Remote sessions are query-only and cannot acquire the local desktop session.
For the default test VM, connect from the host with
`ssh -p 22222 YOUR_USER@127.0.0.1`. Never place passwords in generated commands,
source code, logs, or application arguments.
