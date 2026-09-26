#include <stddef.h>
#include <stdint.h>

typedef uint64_t memory_word __attribute__((__may_alias__));

// ------------------------=
// FUNC: memset
// DESC: Clears private model buffers with aligned volatile words, preserving erasure and exact bounds.
// ------------------=
void *memset(void *pointer, int value, size_t count) {
    volatile unsigned char *out = pointer;
    unsigned char byte = (unsigned char)value;
    while (count && ((uintptr_t)out & 7)) { *out++ = byte; --count; }
    uint64_t word = (uint64_t)byte * UINT64_C(0x0101010101010101);
    while (count >= 8) { *(volatile memory_word *)out = word; out += 8; count -= 8; }
    while (count--) *out++ = byte;
    return pointer;
}

// ------------------------=
// FUNC: memcpy
// DESC: Copies aligned words only when both buffers permit it, with byte-exact unaligned fallback.
// ------------------=
void *memcpy(void *destination, const void *source, size_t count) {
    volatile unsigned char *out = destination;
    const volatile unsigned char *in = source;
    if (((uintptr_t)out & 7) == ((uintptr_t)in & 7)) {
        while (count && ((uintptr_t)out & 7)) { *out++ = *in++; --count; }
        while (count >= 8) {
            *(volatile memory_word *)out = *(const volatile memory_word *)in;
            out += 8; in += 8; count -= 8;
        }
    }
    while (count--) *out++ = *in++;
    return destination;
}

// ------------------------=
// FUNC: memmove
// DESC: Copies overlapping buffers in the safe direction without touching bytes outside the requested span.
// ------------------=
void *memmove(void *destination, const void *source, size_t count) {
    if ((uintptr_t)destination <= (uintptr_t)source) return memcpy(destination, source, count);
    volatile unsigned char *out = (volatile unsigned char *)destination + count;
    const volatile unsigned char *in = (const volatile unsigned char *)source + count;
    if (((uintptr_t)out & 7) == ((uintptr_t)in & 7)) {
        while (count && ((uintptr_t)out & 7)) { *--out = *--in; --count; }
        while (count >= 8) {
            out -= 8; in -= 8; count -= 8;
            *(volatile memory_word *)out = *(const volatile memory_word *)in;
        }
    }
    while (count--) *--out = *--in;
    return destination;
}
