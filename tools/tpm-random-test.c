#include <assert.h>
#include <string.h>
#define EFIAPI
#include "../boot/common/tpm_random.h"

typedef struct {
    InfinityTcg2 protocol;
    unsigned calls, supplied, chunk, fault;
} Fixture;

// ------------------------=
// FUNC: submit
// DESC: Exercises exact TPM wire requests and injects bounded valid, partial, malformed and failing responses.
// ------------------=
static uint64_t submit(InfinityTcg2 *protocol, uint32_t input_size, uint8_t *input,
                       uint32_t output_size, uint8_t *output) {
    Fixture *f = (Fixture *)protocol;
    f->calls++;
    const uint8_t header[11] = {0x80,1,0,0,0,12,0,0,1,0x7b,0};
    assert(input_size == 12 && output_size == 44);
    assert(!memcmp(input, header, 11));
    assert(input[11] == 32-f->supplied);
    if (f->fault == 1 || (f->fault == 9 && f->calls == 3)) return 1;
    unsigned count = input[11] < f->chunk ? input[11] : f->chunk;
    output[0] = 0x80; output[1] = 1; output[5] = 12+count; output[11] = count;
    for (unsigned i=0;i<count;++i) output[12+i] = f->fault == 8 ? 0 : f->supplied+i+1;
    f->supplied += count;
    switch(f->fault) {
        case 2: output[0]=0; break;
        case 3: output[9]=1; break;
        case 4: output[11]=33; break;
        case 5: output[5]++; break;
        case 6: output[11]=0; output[5]=12; break;
        case 7: output[2]=255; break;
        default: break;
    }
    return 0;
}

// ------------------------=
// FUNC: main
// DESC: Verifies full/partial TPM randomness, finite retries, malformed-response rejection and zeroed outputs after every failure under sanitizers.
// ------------------=
int main(void) {
    uint8_t result[32];
    for (unsigned chunk=1;chunk<=32;++chunk) {
        Fixture f = {{0,0,0,submit},0,0,chunk,0};
        assert(infinity_tpm_random(&f.protocol,result));
        assert(f.calls == (32+chunk-1)/chunk && f.calls <= 32);
        for (unsigned i=0;i<32;++i) assert(result[i] == i+1);
    }
    for (unsigned fault=1;fault<=9;++fault) {
        Fixture f = {{0,0,0,submit},0,0,4,fault};
        memset(result,0xaa,32);
        assert(!infinity_tpm_random(&f.protocol,result));
        assert(f.calls <= 32);
        for (unsigned i=0;i<32;++i) assert(result[i] == 0);
    }
    assert(!infinity_tpm_random(0,result));
    InfinityTcg2 absent = {0};
    assert(!infinity_tpm_random(&absent,result));
    return 0;
}
