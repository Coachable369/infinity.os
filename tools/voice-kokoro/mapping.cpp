#include "core/file_mapping.h"
#include <utility>
extern "C" const unsigned char *native_resource(const char *, size_t *);
namespace kokopop {
// ------------------------=
// FUNC: FileMapping
// DESC: Borrows exact immutable packaged resources without filesystem mapping or host access.
// ------------------=
FileMapping::FileMapping(const std::string &path) { _data = native_resource(path.c_str(), &_size); }
// ------------------------=
// FUNC: FileMapping
// DESC: Transfers a borrowed resource extent without changing its lifetime.
// ------------------=
FileMapping::FileMapping(FileMapping &&other) noexcept { *this = std::move(other); }
// ------------------------=
// FUNC: operator=
// DESC: Moves a borrowed immutable resource and invalidates the previous handle.
// ------------------=
FileMapping &FileMapping::operator=(FileMapping &&other) noexcept {
    if (this != &other) { close(); _data = other._data; _size = other._size; other.close(); }
    return *this;
}
// ------------------------=
// FUNC: ~FileMapping
// DESC: Releases the handle but never frees immutable System Generation bytes.
// ------------------=
FileMapping::~FileMapping() { close(); }
// ------------------------=
// FUNC: close
// DESC: Invalidates a private borrowed handle.
// ------------------=
void FileMapping::close() { _data = nullptr; _size = 0; }
// ------------------------=
// FUNC: advise_random
// DESC: Resident immutable resource reads need no OS page-cache advice.
// ------------------=
void FileMapping::advise_random() {}
}
