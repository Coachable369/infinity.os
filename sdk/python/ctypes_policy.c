#include <Python.h>
#include <ffi.h>
#include <dlfcn.h>

_Static_assert(sizeof(_Bool) == SIZEOF__BOOL, "Native Python bool ABI mismatch");
_Static_assert(sizeof(long double) == SIZEOF_LONG_DOUBLE, "Native Python long-double ABI mismatch");
_Static_assert(sizeof(void *) == 8 && sizeof(long) == 8, "Native Python LP64 ABI mismatch");
_Static_assert(RTLD_LOCAL == 0 && RTLD_GLOBAL == 4, "Native scoped-library visibility flags mismatch");

// ------------------------=
// FUNC: ffi_closure_free
// DESC: Guards the unreachable upstream cleanup path; executable callback construction is explicitly rejected before allocation.
// ------------------=
void ffi_closure_free(void *allocation) {
    if (allocation != NULL) {
        Py_FatalError("Unexpected executable ctypes closure allocation");
    }
}
