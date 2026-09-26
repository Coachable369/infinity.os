#include <assert.h>
#include <stddef.h>
#include <string.h>
void *native_memcpy(void *, const void *, size_t);
void *native_memmove(void *, const void *, size_t);
void *native_memset(void *, int, size_t);

// ------------------------=
// FUNC: main
// DESC: Compares exact bytes and surrounding sentinels against libc over all alignment and overlap combinations.
// ------------------=
int main(void) {
    unsigned char original[512], expected[512], actual[512];
    for (size_t i = 0; i < sizeof(original); ++i) original[i] = (unsigned char)(i * 37);
    for (size_t from = 16; from < 48; ++from) {
        for (size_t to = 16; to < 48; ++to) {
            for (size_t count = 0; count <= 256; ++count) {
                memcpy(expected, original, sizeof(original));
                memcpy(actual, original, sizeof(original));
                memmove(expected + to, expected + from, count);
                assert(native_memmove(actual + to, actual + from, count) == actual + to);
                assert(memcmp(expected, actual, sizeof(actual)) == 0);
                memcpy(expected + to, original + from, count);
                assert(native_memcpy(actual + to, original + from, count) == actual + to);
                assert(memcmp(expected, actual, sizeof(actual)) == 0);
                memset(expected + to, 0xa5, count);
                assert(native_memset(actual + to, 0xa5, count) == actual + to);
                assert(memcmp(expected, actual, sizeof(actual)) == 0);
            }
        }
    }
    return 0;
}
