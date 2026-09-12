#include "llvm/Support/raw_socket_stream.h"
#include "llvm/Support/Error.h"
#include <cassert>
#include <infinity/llvm_socket_unavailable.inc>

// ------------------------=
// FUNC: main
// DESC: Verifies unsupported socket operations fail with typed errors and no data changes.
// ------------------=
int main() {
  for (auto Path : {"", "/home/default/socket", "/system/socket"}) {
    auto Listener = llvm::ListeningSocket::createUnix(Path, 1);
    assert(!Listener);
    assert(llvm::errorToErrorCode(Listener.takeError()) ==
           std::errc::operation_not_supported);
    auto Client = llvm::raw_socket_stream::createConnectedUnix(Path);
    assert(!Client);
    assert(llvm::errorToErrorCode(Client.takeError()) ==
           std::errc::operation_not_supported);
  }
  llvm::raw_socket_stream Stream(-1);
  char Bytes[] = {1, 2, 3};
  assert(Stream.read(Bytes, sizeof(Bytes), std::chrono::milliseconds(-1)) == -1);
  assert(Bytes[0] == 1 && Bytes[1] == 2 && Bytes[2] == 3);
  assert(Stream.error() == std::errc::operation_not_supported);
  Stream.clear_error();
  return 0;
}
