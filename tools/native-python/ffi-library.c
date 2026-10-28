#include <stdint.h>
#include <stddef.h>
#include <stdarg.h>

typedef struct { int32_t integer; double decimal; } Pair;
typedef struct { double values[4]; } Vector;
typedef struct { int64_t values[5]; } Large;
static int64_t counter = 7;

// ------------------------=
// FUNC: native_ffi_integer
// DESC: Exercises signed narrowing and full-width outbound integer arguments without any host dependency.
// ------------------=
int64_t native_ffi_integer(int8_t a, uint16_t b, int64_t c) { return c + a * (int64_t)b; }

// ------------------------=
// FUNC: native_ffi_mixed
// DESC: Exercises separate native integer and floating-point register banks.
// ------------------=
double native_ffi_mixed(int32_t a, double b, float c, uint64_t d) { return a + b * c + d; }

// ------------------------=
// FUNC: native_ffi_pair
// DESC: Returns a mixed-layout structure by value through the actual target ABI.
// ------------------=
Pair native_ffi_pair(Pair value, int32_t delta) {
    value.integer += delta;
    value.decimal *= 2.0;
    return value;
}

// ------------------------=
// FUNC: native_ffi_vector
// DESC: Exercises a four-double homogeneous aggregate and target register-return rules.
// ------------------=
Vector native_ffi_vector(Vector value) {
    for (size_t i = 0; i < 4; ++i) value.values[i] += (double)i;
    return value;
}

// ------------------------=
// FUNC: native_ffi_large
// DESC: Exercises an aggregate that must be passed and returned with indirect native storage.
// ------------------=
Large native_ffi_large(Large value) {
    for (size_t i = 0; i < 5; ++i) value.values[i] *= (int64_t)(i + 1);
    return value;
}

// ------------------------=
// FUNC: native_ffi_variadic
// DESC: Sums promoted double arguments including register overflow through genuine va_list semantics.
// ------------------=
double native_ffi_variadic(unsigned count, ...) {
    va_list arguments;
    va_start(arguments, count);
    double total = 0;
    for (unsigned i = 0; i < count; ++i) total += va_arg(arguments, double);
    va_end(arguments);
    return total;
}

// ------------------------=
// FUNC: native_ffi_many
// DESC: Exercises stack arguments beyond the native integer argument-register limit.
// ------------------=
int64_t native_ffi_many(int64_t a, int64_t b, int64_t c, int64_t d, int64_t e,
                       int64_t f, int64_t g, int64_t h, int64_t i, int64_t j,
                       int64_t k, int64_t l) { return a+b+c+d+e+f+g+h+i+j+k+l; }

// ------------------------=
// FUNC: native_ffi_bytes
// DESC: Mutates only the explicitly supplied process-private byte buffer.
// ------------------=
void native_ffi_bytes(unsigned char *bytes, size_t count) {
    for (size_t i = 0; i < count; ++i) bytes[i] ^= 0x5a;
}

// ------------------------=
// FUNC: native_ffi_counter
// DESC: Returns a genuine relocated writable-data pointer from the loaded native library.
// ------------------=
int64_t *native_ffi_counter(void) { return &counter; }
