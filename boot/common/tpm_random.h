#ifndef INFINITY_TPM_RANDOM_H
#define INFINITY_TPM_RANDOM_H
#include <stdint.h>

typedef struct InfinityTcg2 InfinityTcg2;
typedef uint64_t (EFIAPI *InfinityTpmSubmit)(InfinityTcg2 *, uint32_t, uint8_t *, uint32_t, uint8_t *);
struct InfinityTcg2 {
    void *get_capability;
    void *get_event_log;
    void *hash_log_extend_event;
    InfinityTpmSubmit submit_command;
};

// ------------------------=
// FUNC: infinity_tpm_random
// DESC: Obtains 32 actual random bytes using bounded TPM2_GetRandom commands; validates framing and partial replies, clears incomplete output, and never substitutes synthetic entropy.
// ------------------=
static int infinity_tpm_random(InfinityTcg2 *tpm, uint8_t entropy[32]) {
    for (unsigned i = 0; i < 32; ++i) entropy[i] = 0;
    if (!tpm || !tpm->submit_command) return 0;
    unsigned filled = 0;
    for (unsigned attempt = 0; attempt < 32 && filled < 32; ++attempt) {
        uint8_t request[12] = {0x80, 0x01, 0, 0, 0, 12, 0, 0, 0x01, 0x7b, 0, 0};
        uint8_t response[44] = {0};
        request[11] = (uint8_t)(32 - filled);
        if (tpm->submit_command(tpm, sizeof(request), request, sizeof(response), response) != 0) break;
        uint32_t size = ((uint32_t)response[2] << 24) | ((uint32_t)response[3] << 16) |
            ((uint32_t)response[4] << 8) | response[5];
        unsigned count = ((unsigned)response[10] << 8) | response[11];
        if (response[0] != 0x80 || response[1] != 1 || response[6] || response[7] ||
            response[8] || response[9] || !count || count > 32 - filled || size != 12 + count) break;
        for (unsigned i = 0; i < count; ++i) entropy[filled+i] = response[12+i];
        filled += count;
    }
    uint8_t nonzero = 0;
    for (unsigned i = 0; i < 32; ++i) nonzero |= entropy[i];
    if (filled == 32 && nonzero) return 1;
    for (unsigned i = 0; i < 32; ++i) entropy[i] = 0;
    return 0;
}
#endif
