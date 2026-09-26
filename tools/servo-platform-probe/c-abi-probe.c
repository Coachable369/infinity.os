#include <sys/types.h>
#include <stdatomic.h>
_Static_assert(sizeof(off_t) == sizeof(long), "Native libz offset ABI mismatch");
_Static_assert(_Alignof(off_t) == _Alignof(long), "Native libz offset alignment mismatch");
_Static_assert(sizeof(pthread_rwlock_t) == sizeof(uint32_t), "Native lock handle ABI mismatch");
_Static_assert(ATOMIC_INT_LOCK_FREE == 2, "Native lock atomics must be lock free");
