#ifndef INFINITY_PYTHON_NETWORK_ERRNO_H
#define INFINITY_PYTHON_NETWORK_ERRNO_H
#include <errno.h>
#include <stdint.h>

// ------------------------=
// FUNC: infinity_network_errno
// DESC: Translates stable native broker failures to the target C runtime's public errno values; certificate rejection is handled as a separate exception type.
// ------------------=
static int infinity_network_errno(int64_t value) {
    switch (value) {
        case -5: return EIO;
        case -9: return EBADF;
        case -11: return EAGAIN;
        case -13: return EACCES;
        case -14: return EFAULT;
        case -16: return EBUSY;
        case -22: return EINVAL;
        case -27: return EFBIG;
        case -38: return ENOSYS;
        case -75: return EOVERFLOW;
        case -100: return ENETDOWN;
        case -110: return ETIMEDOUT;
        case -113: return EHOSTUNREACH;
        case -125: return ECANCELED;
        default: return EIO;
    }
}
#endif
