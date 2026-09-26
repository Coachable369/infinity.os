#include "MmapFaultHandler.h"
#include <cassert>
static int visits;
// ------------------------=
// FUNC: file_scope
// DESC: Unsupported file-backed access must return failure without evaluating the body.
// ------------------=
static int file_scope() {
    MMAP_FAULT_HANDLER_BEGIN_HANDLE(nullptr)
    ++visits;
    MMAP_FAULT_HANDLER_CATCH(-17)
    return 0;
}
// ------------------------=
// FUNC: buffer_scope
// DESC: Owned buffers execute normally without claiming Unix signal fault recovery.
// ------------------=
static int buffer_scope(int* value) {
    MMAP_FAULT_HANDLER_BEGIN_BUFFER(value, sizeof(*value))
    *value = 29;
    ++visits;
    MMAP_FAULT_HANDLER_CATCH(-17)
    return 0;
}
// ------------------------=
// FUNC: main
// DESC: Checks observable file rejection and owned-buffer mutation in the staged native macros.
// ------------------=
int main() {
    assert(file_scope() == -17 && visits == 0);
    int value = 0;
    assert(buffer_scope(&value) == 0 && value == 29 && visits == 1);
}
