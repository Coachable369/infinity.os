#ifndef INFINITY_NATIVE_ERROR_H
#define INFINITY_NATIVE_ERROR_H
#include <errno.h>
// ------------------------=
// FUNC: infinity_native_error
// DESC: Translates the stable native ABI errors into newlib errno values, which are not Linux numbers.
// ------------------=
static inline int infinity_native_error(int status) {
    switch (status) {
        case 0: return 0;
        case 4: return EINTR;
        case 11: return EAGAIN;
        case 12: return ENOMEM;
        case 22: return EINVAL;
        case 38: return ENOSYS;
        case 75: return EOVERFLOW;
        case 95: return ENOTSUP;
        case 110: return ETIMEDOUT;
        default: return EIO;
    }
}
#endif
