// LLVM compatibility boundary: Unix-domain sockets are not native authority.
using namespace llvm;

// ------------------------=
// FUNC: unavailableSocket
// DESC: Returns a typed failure without acquiring any host or guest resource.
// ------------------=
static Error unavailableSocket() {
  return make_error<StringError>(
      std::make_error_code(std::errc::operation_not_supported),
      "Unix socket transport is unavailable on InfinityOS");
}
// ------------------------=
// FUNC: ListeningSocket
// DESC: Constructs only invalid compatibility state; no descriptor is adopted.
// ------------------=
ListeningSocket::ListeningSocket(int, StringRef, int[2])
    : FD(-1), SocketPath(), PipeFD{-1, -1} {}
// ------------------------=
// FUNC: ListeningSocket
// DESC: Moves the always-invalid state without acquiring resources.
// ------------------=
ListeningSocket::ListeningSocket(ListeningSocket &&Other)
    : FD(-1), SocketPath(), PipeFD{-1, -1} { Other.FD = -1; }
// ------------------------=
// FUNC: createUnix
// DESC: Rejects Unix listener creation regardless of path or backlog.
// ------------------=
Expected<ListeningSocket> ListeningSocket::createUnix(StringRef, int) {
  return unavailableSocket();
}
// ------------------------=
// FUNC: accept
// DESC: Rejects accept immediately instead of blocking on an absent transport.
// ------------------=
Expected<std::unique_ptr<raw_socket_stream>>
ListeningSocket::accept(const std::chrono::milliseconds &) {
  return unavailableSocket();
}
// ------------------------=
// FUNC: shutdown
// DESC: Keeps unsupported listener state invalid; there is no acquired resource.
// ------------------=
void ListeningSocket::shutdown() { FD = -1; }
// ------------------------=
// FUNC: ~ListeningSocket
// DESC: Destroys invalid state without unlinking an ObjectStore name.
// ------------------=
ListeningSocket::~ListeningSocket() { shutdown(); }
// ------------------------=
// FUNC: raw_socket_stream
// DESC: Refuses adoption of a supplied descriptor for unsupported socket I/O.
// ------------------=
raw_socket_stream::raw_socket_stream(int) : raw_fd_stream(-1, false) {}
// ------------------------=
// FUNC: ~raw_socket_stream
// DESC: Releases the invalid stream without owning a descriptor.
// ------------------=
raw_socket_stream::~raw_socket_stream() = default;
// ------------------------=
// FUNC: createConnectedUnix
// DESC: Rejects connection creation without interpreting a namespace as a socket.
// ------------------=
Expected<std::unique_ptr<raw_socket_stream>>
raw_socket_stream::createConnectedUnix(StringRef) { return unavailableSocket(); }
// ------------------------=
// FUNC: read
// DESC: Leaves caller bytes untouched and records an explicit unsupported error.
// ------------------=
ssize_t raw_socket_stream::read(char *, size_t, const std::chrono::milliseconds &) {
  error_detected(std::make_error_code(std::errc::operation_not_supported));
  return -1;
}
