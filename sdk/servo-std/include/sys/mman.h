#ifndef INFINITY_SERVO_MMAN_H
#define INFINITY_SERVO_MMAN_H
/* Share the native mapping ABI, not the compiler's serial runtime.
 * Actual map/protect/unmap providers remain mandatory at engine linkage. */
#include "../../../compiler/include/sys/mman.h"
#endif
