#include <stdint.h>
#include <stddef.h>
#include <stdlib.h>
#include <string.h>
#include <assert.h>
#define EFIAPI
#include "../boot/common/payload_cache.h"

// ------------------------=
// FUNC: main
// DESC: Exercises cross-shard copies, short tails and atomic rejection of invalid or overflowing requests.
// ------------------=
int main(void) {
    uint8_t *first = malloc(PAYLOAD_PART_BYTES);
    assert(first);
    uint8_t tail[] = {3, 4, 5}, output[8];
    first[PAYLOAD_PART_BYTES - 2] = 1; first[PAYLOAD_PART_BYTES - 1] = 2;
    payload_cache[0][0] = (PayloadPart){first, PAYLOAD_PART_BYTES};
    payload_cache[0][1] = (PayloadPart){tail, sizeof(tail)};
    assert(!payload_cached_read(0, PAYLOAD_PART_BYTES - 2, 5, output));
    for (size_t i = 0; i < 5; ++i) assert(output[i] == i + 1);
    memset(output, 0xa5, sizeof(output));
    assert(payload_cached_read(0, PAYLOAD_PART_BYTES - 2, 6, output));
    assert(payload_cached_read(0, UINT64_MAX - 1, 8, output));
    assert(payload_cached_read(2, 0, 1, output));
    assert(payload_cached_read(0, 0, 1024 * 1024 + 1, output));
    assert(payload_cached_read(0, 0, 1, NULL));
    for (size_t i = 0; i < sizeof(output); ++i) assert(output[i] == 0xa5);
    free(first);
    return 0;
}
