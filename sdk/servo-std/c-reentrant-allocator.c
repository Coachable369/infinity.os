#include <reent.h>
#include <stdlib.h>
#include <errno.h>

// ------------------------=
// FUNC: _malloc_r
// DESC: Keeps newlib allocation on the native heap and records allocation failure in the caller's state.
// ------------------=
void *_malloc_r(struct _reent *r, size_t size) {
    void *p = malloc(size);
    if (!p && r) r->_errno = ENOMEM;
    return p;
}
// ------------------------=
// FUNC: _calloc_r
// DESC: Delegates overflow-safe zeroed allocation to the governed native heap.
// ------------------=
void *_calloc_r(struct _reent *r, size_t count, size_t size) {
    void *p = calloc(count, size);
    if (!p && r) r->_errno = ENOMEM;
    return p;
}
// ------------------------=
// FUNC: _realloc_r
// DESC: Preserves failure ownership and reports nonzero resize exhaustion.
// ------------------=
void *_realloc_r(struct _reent *r, void *pointer, size_t size) {
    void *p = realloc(pointer, size);
    if (!p && size && r) r->_errno = ENOMEM;
    return p;
}
// ------------------------=
// FUNC: _free_r
// DESC: Releases through the shared native allocation header, never a second libc heap.
// ------------------=
void _free_r(struct _reent *r, void *pointer) { (void)r; free(pointer); }
