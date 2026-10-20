/* The probe consumes the same target configuration and actual Newlib clock
 * identifiers as native CPython/compiler C. Do not substitute host clock IDs. */
#include "../../sdk/compiler/target.h"
#include <stdint.h>
#include <time.h>

const uint64_t INFINITY_PROBE_CLOCK_IDS[2] = { CLOCK_REALTIME, CLOCK_MONOTONIC };
