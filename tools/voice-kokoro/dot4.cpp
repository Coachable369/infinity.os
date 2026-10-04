#if defined(__aarch64__)
#include <arm_neon.h>
#else
#if defined(NATIVE_FAST_F16C)
#include <immintrin.h>
#else
#include <xmmintrin.h>
extern "C" float ggml_table_f32_f16[65536];
#endif
#endif
#include <stddef.h>
#include <stdint.h>

// ------------------------=
// FUNC: load
// DESC: Converts one aligned half or single precision lane group without changing the baseline dot-product precision.
// ------------------=
#if defined(__aarch64__)
template<bool Half> static inline float32x4_t load(const unsigned char *p) {
    if constexpr (Half) return vcvt_f32_f16(vld1_f16((const __fp16 *)__builtin_assume_aligned(p, 8)));
    else return vld1q_f32((const float *)__builtin_assume_aligned(p, 16));
}
#else
// ------------------------=
// FUNC: load
// DESC: Converts half lanes through baseline tables or the separately guarded F16C variant.
// ------------------=
template<bool Half> static inline __m128 load(const unsigned char *p) {
    if constexpr (Half) {
#if defined(NATIVE_FAST_F16C)
        return _mm_cvtph_ps(_mm_loadl_epi64((const __m128i *)p));
#else
        const uint16_t *h = (const uint16_t *)p;
        return _mm_set_ps(ggml_table_f32_f16[h[3]], ggml_table_f32_f16[h[2]],
                          ggml_table_f32_f16[h[1]], ggml_table_f32_f16[h[0]]);
#endif
    } else return _mm_load_ps((const float *)p);
}
#endif

// ------------------------=
// FUNC: dot4
// DESC: Reuses each activation vector across four weight rows while preserving upstream accumulator and reduction order.
// ------------------=
template<bool Half> static void dot4(int n, float *out, const unsigned char *x, size_t stride, const unsigned char *y) {
    constexpr int width = Half ? 2 : 4;
#if defined(__aarch64__)
    float32x4_t sums[4][4];
    for (int row = 0; row < 4; ++row) for (int lane = 0; lane < 4; ++lane) sums[row][lane] = vdupq_n_f32(0);
    for (int i = 0; i < n; i += 16) {
#if defined(NATIVE_FAST_FHM)
        if constexpr (Half) {
            // Widening half multiplication still accumulates in FP32, with
            // the same four accumulators and final reduction as the fallback.
            for (int pair = 0; pair < 2; ++pair) {
                const auto activation = vld1q_f16((const __fp16 *)__builtin_assume_aligned(y + (i + pair * 8) * 2,16));
                for (int row = 0; row < 4; ++row) {
                    const auto weight = vld1q_f16((const __fp16 *)__builtin_assume_aligned(x + row * stride + (i + pair * 8) * 2,16));
                    sums[row][pair*2] = vfmlalq_low_f16(sums[row][pair*2],weight,activation);
                    sums[row][pair*2+1] = vfmlalq_high_f16(sums[row][pair*2+1],weight,activation);
                }
            }
            continue;
        }
#endif
        #pragma clang loop unroll(full)
        for (int lane = 0; lane < 4; ++lane) {
            const auto activation = load<Half>(y + (i + lane * 4) * width);
            #pragma clang loop unroll(full)
            for (int row = 0; row < 4; ++row)
                sums[row][lane] = vfmaq_f32(sums[row][lane], load<Half>(x + row * stride + (i + lane * 4) * width), activation);
        }
    }
    for (int row = 0; row < 4; ++row)
        out[row] = vaddvq_f32(vaddq_f32(vaddq_f32(sums[row][0], sums[row][2]), vaddq_f32(sums[row][1], sums[row][3])));
#else
    __m128 sums[4][8];
    // Four live accumulators fit SSE2's register file. Keeping all 32 live
    // caused a load/store spill for every multiply-add in the previous loop.
    #pragma clang loop unroll(disable)
    for (int lane = 0; lane < 8; ++lane) {
        __m128 acc[4] = {_mm_setzero_ps(), _mm_setzero_ps(), _mm_setzero_ps(), _mm_setzero_ps()};
        #pragma clang loop unroll(disable)
        for (int i = 0; i < n; i += 32) {
            const auto activation = load<Half>(y + (i + lane * 4) * width);
            #pragma clang loop unroll(full)
            for (int row = 0; row < 4; ++row)
                acc[row] = _mm_add_ps(acc[row],
                    _mm_mul_ps(load<Half>(x + row * stride + (i + lane * 4) * width), activation));
        }
        for (int row = 0; row < 4; ++row) sums[row][lane] = acc[row];
    }
    for (int row = 0; row < 4; ++row) {
        for (int offset = 4; offset; offset >>= 1)
            for (int lane = 0; lane < offset; ++lane)
                sums[row][lane] = _mm_add_ps(sums[row][lane], sums[row][lane + offset]);
        __m128 value = sums[row][0];
        __m128 paired = _mm_add_ps(value, _mm_shuffle_ps(value, value, _MM_SHUFFLE(2,3,0,1)));
        out[row] = _mm_cvtss_f32(_mm_add_ss(paired, _mm_movehl_ps(paired, paired)));
    }
#endif
}

// ------------------------=
// FUNC: dot2x2
// DESC: Reuses weights and activations across two rows and columns with exactly the upstream per-dot accumulator and reduction order.
// ------------------=
template<bool Half> static void dot2x2(int n, float *out, size_t output_stride,
    const unsigned char *x, size_t x_stride, const unsigned char *y, size_t y_stride) {
    constexpr int width = Half ? 2 : 4;
#if defined(__aarch64__)
    float32x4_t sums[2][2][4];
    for (int row = 0; row < 2; ++row) for (int col = 0; col < 2; ++col)
        for (int lane = 0; lane < 4; ++lane) sums[row][col][lane] = vdupq_n_f32(0);
    for (int i = 0; i < n; i += 16) {
#if defined(NATIVE_FAST_FHM)
        if constexpr (Half) {
            #pragma clang loop unroll(full)
            for (int pair = 0; pair < 2; ++pair) {
                const auto a0 = vld1q_f16((const __fp16 *)__builtin_assume_aligned(x + (i + pair * 8) * 2, 16));
                const auto a1 = vld1q_f16((const __fp16 *)__builtin_assume_aligned(x + x_stride + (i + pair * 8) * 2, 16));
                const auto b0 = vld1q_f16((const __fp16 *)__builtin_assume_aligned(y + (i + pair * 8) * 2, 16));
                const auto b1 = vld1q_f16((const __fp16 *)__builtin_assume_aligned(y + y_stride + (i + pair * 8) * 2, 16));
                sums[0][0][pair*2] = vfmlalq_low_f16(sums[0][0][pair*2], a0, b0);
                sums[0][0][pair*2+1] = vfmlalq_high_f16(sums[0][0][pair*2+1], a0, b0);
                sums[1][0][pair*2] = vfmlalq_low_f16(sums[1][0][pair*2], a1, b0);
                sums[1][0][pair*2+1] = vfmlalq_high_f16(sums[1][0][pair*2+1], a1, b0);
                sums[0][1][pair*2] = vfmlalq_low_f16(sums[0][1][pair*2], a0, b1);
                sums[0][1][pair*2+1] = vfmlalq_high_f16(sums[0][1][pair*2+1], a0, b1);
                sums[1][1][pair*2] = vfmlalq_low_f16(sums[1][1][pair*2], a1, b1);
                sums[1][1][pair*2+1] = vfmlalq_high_f16(sums[1][1][pair*2+1], a1, b1);
            }
            continue;
        }
#endif
        #pragma clang loop unroll(full)
        for (int lane = 0; lane < 4; ++lane) {
            const auto a0 = load<Half>(x + (i + lane * 4) * width);
            const auto a1 = load<Half>(x + x_stride + (i + lane * 4) * width);
            const auto b0 = load<Half>(y + (i + lane * 4) * width);
            const auto b1 = load<Half>(y + y_stride + (i + lane * 4) * width);
            sums[0][0][lane] = vfmaq_f32(sums[0][0][lane], a0, b0);
            sums[1][0][lane] = vfmaq_f32(sums[1][0][lane], a1, b0);
            sums[0][1][lane] = vfmaq_f32(sums[0][1][lane], a0, b1);
            sums[1][1][lane] = vfmaq_f32(sums[1][1][lane], a1, b1);
        }
    }
    for (int col = 0; col < 2; ++col) for (int row = 0; row < 2; ++row) {
        float32x4_t *sum = sums[row][col];
        ((float *)((unsigned char *)out + col * output_stride))[row] =
            vaddvq_f32(vaddq_f32(vaddq_f32(sum[0], sum[2]), vaddq_f32(sum[1], sum[3])));
    }
#else
    __m128 sums[2][2][8];
    #pragma clang loop unroll(disable)
    for (int lane = 0; lane < 8; ++lane) {
        __m128 acc[2][2] = {{_mm_setzero_ps(), _mm_setzero_ps()}, {_mm_setzero_ps(), _mm_setzero_ps()}};
        for (int i = 0; i < n; i += 32) {
            const auto a0 = load<Half>(x + (i + lane * 4) * width);
            const auto a1 = load<Half>(x + x_stride + (i + lane * 4) * width);
            const auto b0 = load<Half>(y + (i + lane * 4) * width);
            const auto b1 = load<Half>(y + y_stride + (i + lane * 4) * width);
            acc[0][0] = _mm_add_ps(acc[0][0], _mm_mul_ps(a0, b0));
            acc[1][0] = _mm_add_ps(acc[1][0], _mm_mul_ps(a1, b0));
            acc[0][1] = _mm_add_ps(acc[0][1], _mm_mul_ps(a0, b1));
            acc[1][1] = _mm_add_ps(acc[1][1], _mm_mul_ps(a1, b1));
        }
        for (int row = 0; row < 2; ++row) for (int col = 0; col < 2; ++col)
            sums[row][col][lane] = acc[row][col];
    }
    for (int col = 0; col < 2; ++col) for (int row = 0; row < 2; ++row) {
        __m128 *sum = sums[row][col];
        for (int offset = 4; offset; offset >>= 1)
            for (int lane = 0; lane < offset; ++lane) sum[lane] = _mm_add_ps(sum[lane], sum[lane + offset]);
        const __m128 paired = _mm_add_ps(sum[0], _mm_shuffle_ps(sum[0], sum[0], _MM_SHUFFLE(2,3,0,1)));
        ((float *)((unsigned char *)out + col * output_stride))[row] =
            _mm_cvtss_f32(_mm_add_ss(paired, _mm_movehl_ps(paired, paired)));
    }
#endif
}

#if defined(__x86_64__) && !defined(NATIVE_FAST_F16C)
extern "C" int native_dot4_f16c(int, int, float *, const void *, size_t, const void *);
extern "C" int native_mat2_f16c(int, int, float *, size_t, const void *, size_t, const void *, size_t, int);
// ------------------------=
// FUNC: native_f16c_available
// DESC: Requires both hardware capability and OS-enabled XMM/YMM state before dispatching optional instructions.
// ------------------=
extern "C" __attribute__((noinline)) int native_f16c_available(void) {
    unsigned a, b, c, d;
    __asm__ volatile("cpuid" : "=a"(a), "=b"(b), "=c"(c), "=d"(d) : "a"(1), "c"(0));
    if ((c & 0x3c180201u) != 0x3c180201u) return 0;
    __asm__ volatile("xgetbv" : "=a"(a), "=d"(d) : "c"(0));
    return (a & 7) == 7;
}
#endif

#if defined(__aarch64__) && !defined(NATIVE_FAST_FHM)
extern "C" int native_dot4_fhm(int, int, float *, const void *, size_t, const void *);
extern "C" int native_mat2_fhm(int, int, float *, size_t, const void *, size_t, const void *, size_t, int);
// ------------------------=
// FUNC: native_fhm_available
// DESC: Checks guest CPU widening-half capability before entering the optional native instruction variant.
// ------------------=
static int native_fhm_available(void) {
    uint64_t features;
    __asm__ volatile("mrs %0, id_aa64isar0_el1" : "=r"(features));
    return ((features >> 48) & 15) == 1;
}
#endif

// ------------------------=
// FUNC: native_dot4_isa
// DESC: Accepts checked four-row floating tiles using immutable capability evidence from the executing CPU.
// ------------------=
extern "C" int
#if defined(NATIVE_FAST_F16C)
native_dot4_f16c
#elif defined(NATIVE_FAST_FHM)
native_dot4_fhm
#else
native_dot4_isa
#endif
(int type, int n, float *out, const void *x, size_t stride, const void *y
#if !defined(NATIVE_FAST_FHM) && !defined(NATIVE_FAST_F16C)
 , int native_isa
#endif
) {
    if (n <= 0 || n % 16 || (type != 0 && type != 1)) return 0;
#if defined(__x86_64__)
    if (n % 32) return 0;
#endif
    const uintptr_t mask =
#if defined(NATIVE_FAST_FHM)
        15;
#else
        type == 1 ? 7 : 15;
#endif
    if (((uintptr_t)x | (uintptr_t)y | stride) & mask) return 0;
#if defined(__aarch64__) && !defined(NATIVE_FAST_FHM)
    if (type == 1 && native_isa && !(((uintptr_t)x | (uintptr_t)y | stride) & 15))
        return native_dot4_fhm(type,n,out,x,stride,y);
#endif
#if defined(__x86_64__) && !defined(NATIVE_FAST_F16C)
    if (native_isa) return native_dot4_f16c(type, n, out, x, stride, y);
#endif
    if (type == 1) dot4<true>(n, out, (const unsigned char *)x, stride, (const unsigned char *)y);
    else dot4<false>(n, out, (const unsigned char *)x, stride, (const unsigned char *)y);
    return 1;
}

// ------------------------=
// FUNC: native_mat2_isa
// DESC: Computes checked paired columns using immutable capability evidence from the executing CPU.
// ------------------=
extern "C" int
#if defined(NATIVE_FAST_F16C)
native_mat2_f16c
#elif defined(NATIVE_FAST_FHM)
native_mat2_fhm
#else
native_mat2_isa
#endif
(int type, int n, float *out, size_t output_stride, const void *x, size_t x_stride,
 const void *y, size_t y_stride, int rows
#if !defined(NATIVE_FAST_FHM) && !defined(NATIVE_FAST_F16C)
 , int native_isa
#endif
) {
    if (!out || !x || !y || n <= 0 || n % 16 || (type != 0 && type != 1) ||
        rows < 2 || rows > 16 || rows % 2 || output_stride < size_t(rows) * sizeof(float) ||
        ((uintptr_t)out | output_stride) & 3) return 0;
#if defined(__x86_64__)
    if (n % 32) return 0;
#endif
    const size_t width = type == 1 ? 2 : 4;
    if (x_stride < size_t(n) * width || y_stride < size_t(n) * width) return 0;
    const uintptr_t mask =
#if defined(NATIVE_FAST_FHM)
        15;
#else
        type == 1 ? 7 : 15;
#endif
    if (((uintptr_t)x | (uintptr_t)y | x_stride | y_stride) & mask) return 0;
#if defined(__aarch64__) && !defined(NATIVE_FAST_FHM)
    if (type == 1 && native_isa && !(((uintptr_t)x | (uintptr_t)y | x_stride | y_stride) & 15))
        return native_mat2_fhm(type, n, out, output_stride, x, x_stride, y, y_stride, rows);
#endif
#if defined(__x86_64__) && !defined(NATIVE_FAST_F16C)
    if (native_isa) return native_mat2_f16c(type, n, out, output_stride, x, x_stride, y, y_stride, rows);
#endif
    for (int row = 0; row < rows; row += 2) {
        const auto *weights = (const unsigned char *)x + row * x_stride;
        if (type == 1) dot2x2<true>(n, out + row, output_stride, weights, x_stride, (const unsigned char *)y, y_stride);
        else dot2x2<false>(n, out + row, output_stride, weights, x_stride, (const unsigned char *)y, y_stride);
    }
    return 1;
}

#if !defined(NATIVE_FAST_FHM) && !defined(NATIVE_FAST_F16C)
// ------------------------=
// FUNC: native_math_isa
// DESC: Queries only the currently executing CPU; callers may reuse the immutable result within one non-migrating math chunk.
// ------------------=
extern "C" int native_math_isa(void) {
#if defined(__aarch64__)
    return native_fhm_available();
#else
    return native_f16c_available();
#endif
}

// ------------------------=
// FUNC: native_dot4
// DESC: Preserves the checked public tile API, resolving current-CPU capabilities before entering optional native instructions.
// ------------------=
extern "C" int native_dot4(int type, int n, float *out, const void *x, size_t stride, const void *y) {
    return native_dot4_isa(type, n, out, x, stride, y, native_math_isa());
}

// ------------------------=
// FUNC: native_mat2
// DESC: Preserves the checked public paired-column API without a shared mutable or thread-local capability cache.
// ------------------=
extern "C" int native_mat2(int type, int n, float *out, size_t output_stride,
    const void *x, size_t x_stride, const void *y, size_t y_stride, int rows) {
    return native_mat2_isa(type, n, out, output_stride, x, x_stride, y, y_stride, rows, native_math_isa());
}
#endif
