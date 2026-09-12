#ifndef INFINITY_LLVM_SEED_FILE_H
#define INFINITY_LLVM_SEED_FILE_H
#include "llvm/Support/VirtualFileSystem.h"
#include "llvm/Support/MemoryBuffer.h"
#include "llvm/Support/ErrorOr.h"
#include <string>
namespace infinityos {
// ------------------------=
// FUNC: readLayoutSeed
// DESC: Reads the first seed line through the caller's filesystem authority, without C++ file streams.
// ------------------=
inline llvm::ErrorOr<std::string> readLayoutSeed(llvm::vfs::FileSystem &fs,
                                                llvm::StringRef path) {
    auto buffer = fs.getBufferForFile(path);
    if (!buffer) return buffer.getError();
    return (*buffer)->getBuffer().split('\n').first.str();
}
}
#endif
