#include <infinity/llvm_seed_file.h>
#include <cassert>
// ------------------------=
// FUNC: main
// DESC: Tests first-line seed values, current versions and missing-file failure without host file I/O.
// ------------------=
int main() {
    llvm::vfs::InMemoryFileSystem fs;
    assert(!infinityos::readLayoutSeed(fs, "/seed"));
    assert(fs.addFile("/seed", 0, llvm::MemoryBuffer::getMemBufferCopy("123\nignored")));
    auto seed = infinityos::readLayoutSeed(fs, "/seed");
    assert(seed && *seed == "123");
    assert(fs.addFile("/empty", 0, llvm::MemoryBuffer::getMemBufferCopy("")));
    auto empty = infinityos::readLayoutSeed(fs, "/empty");
    assert(empty && empty->empty());
    assert(fs.addFile("/crlf", 0, llvm::MemoryBuffer::getMemBufferCopy("x\r\n")));
    auto crlf = infinityos::readLayoutSeed(fs, "/crlf");
    assert(crlf && *crlf == "x\r");
    llvm::vfs::InMemoryFileSystem edited;
    assert(edited.addFile("/seed", 0, llvm::MemoryBuffer::getMemBufferCopy("456")));
    auto current = infinityos::readLayoutSeed(edited, "/seed");
    assert(current && *current == "456" && *seed == "123");
    return 0;
}
