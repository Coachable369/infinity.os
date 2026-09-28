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

#if defined(__x86_64__) && !defined(NATIVE_FAST_F16C)
extern "C" int native_dot4_f16c(int, int, float *, const void *, size_t, const void *);
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
// FUNC: native_dot4
// DESC: Accepts only complete four-row FP32/FP16 tiles with verified alignment; all other tensors retain upstream execution.
// ------------------=
extern "C" int
#if defined(NATIVE_FAST_F16C)
native_dot4_f16c
#elif defined(NATIVE_FAST_FHM)
native_dot4_fhm
#else
native_dot4
#endif
(int type, int n, float *out, const void *x, size_t stride, const void *y) {
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
    static int accelerated = -1;
    if (accelerated < 0) accelerated = native_fhm_available();
    if (type == 1 && accelerated && !(((uintptr_t)x | (uintptr_t)y | stride) & 15))
        return native_dot4_fhm(type,n,out,x,stride,y);
#endif
#if defined(__x86_64__) && !defined(NATIVE_FAST_F16C)
    // One speech worker owns this engine for its lifetime.
    static int accelerated = -1;
    if (accelerated < 0) accelerated = native_f16c_available();
    if (accelerated) return native_dot4_f16c(type, n, out, x, stride, y);
#endif
    if (type == 1) dot4<true>(n, out, (const unsigned char *)x, stride, (const unsigned char *)y);
    else dot4<false>(n, out, (const unsigned char *)x, stride, (const unsigned char *)y);
    return 1;
}
