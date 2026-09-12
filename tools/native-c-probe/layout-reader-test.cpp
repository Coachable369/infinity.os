#include "clang/Frontend/LayoutOverrideSource.h"
#include "clang/Tooling/Tooling.h"
#include "clang/AST/Decl.h"
#include "llvm/Support/VirtualFileSystem.h"
#include "llvm/Support/MemoryBuffer.h"
#include <cassert>
// ------------------------=
// FUNC: main
// DESC: Exercises the patched parser through numeric record layouts using only an in-memory VFS.
// ------------------=
int main() {
    auto ast = clang::tooling::buildASTFromCode("struct Example { int a; char b; };");
    assert(ast);
    const clang::RecordDecl *record = nullptr;
    for (auto *decl : ast->getASTContext().getTranslationUnitDecl()->decls())
        if (auto *r = llvm::dyn_cast<clang::RecordDecl>(decl))
            if (r->getName() == "Example") record = r;
    assert(record);
    llvm::vfs::InMemoryFileSystem fs;
    assert(fs.addFile("/layout", 0, llvm::MemoryBuffer::getMemBufferCopy(
        "*** Dumping AST Record Layout\nstruct Example\n Size:128\nAlignment:64\nFieldOffsets: [0, 64]")));
    clang::LayoutOverrideSource source("/layout", fs);
    uint64_t size = 0, alignment = 0;
    llvm::DenseMap<const clang::FieldDecl *, uint64_t> fields;
    llvm::DenseMap<const clang::CXXRecordDecl *, clang::CharUnits> bases, vbases;
    assert(source.layoutRecordType(record, size, alignment, fields, bases, vbases));
    assert(size == 128 && alignment == 64 && fields.size() == 2);
    auto f = record->field_begin(); assert(fields[*f++] == 0); assert(fields[*f] == 64);
    clang::LayoutOverrideSource missing("/missing", fs);
    assert(!missing.layoutRecordType(record, size, alignment, fields, bases, vbases));
    llvm::vfs::InMemoryFileSystem next;
    assert(next.addFile("/layout", 0, llvm::MemoryBuffer::getMemBufferCopy(
        "*** Dumping AST Record Layout\r\nstruct Example\r\n [sizeof=32, align=16]\r\nFieldOffsets: [0, 128]\r\n")));
    clang::LayoutOverrideSource edited("/layout", next);
    assert(edited.layoutRecordType(record, size, alignment, fields, bases, vbases));
    assert(size == 256 && alignment == 128 && fields[*f] == 128);
    return 0;
}
