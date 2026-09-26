#include <reent.h>
#include <errno.h>
#include <stdlib.h>
extern void *malloc(size_t);
extern void *calloc(size_t, size_t);
extern void *realloc(void *, size_t);
extern void free(void *);

// ------------------------=
// FUNC: infinity_c_allocator_test
// DESC: Exercises the real C ABI including zero-fill, overflow, alignment and failed resize ownership.
// ------------------=
int infinity_c_allocator_test(void) {
    unsigned char *p = calloc(17, 3);
    if (!p || ((size_t)p & 15)) return 1;
    for (size_t i = 0; i < 51; ++i) if (p[i]) return 2;
    for (size_t i = 0; i < 51; ++i) p[i] = (unsigned char)(i + 1);
    if (realloc(p, (size_t)-1)) return 3;
    unsigned char *q = realloc(p, 101);
    if (!q) return 4;
    for (size_t i = 0; i < 51; ++i) if (q[i] != i + 1) return 5;
    free(q);
    if (calloc((size_t)-1, 2)) return 6;
    free(0);
    p = malloc(7);
    if (!p) return 7;
    free(p);
    struct _reent state = {0};
    if (_calloc_r(&state, (size_t)-1, 2) || state._errno != ENOMEM) return 8;
    p = _malloc_r(&state, 9);
    if (!p) return 9;
    p[0] = 42;
    if (_realloc_r(&state, p, (size_t)-1) || p[0] != 42) return 10;
    _free_r(&state, p);
    return 0;
}
