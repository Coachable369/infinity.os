#if defined(__aarch64__)
#include <arm_neon.h>
#else
#include <xmmintrin.h>
extern "C" float ggml_table_f32_f16[65536];
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
// DESC: Uses initialized IEEE half conversion tables and baseline SSE without requiring AVX or F16C.
// ------------------=
template<bool Half> static inline __m128 load(const unsigned char *p) {
    if constexpr (Half) {
        const uint16_t *h = (const uint16_t *)p;
        return _mm_set_ps(ggml_table_f32_f16[h[3]], ggml_table_f32_f16[h[2]],
                          ggml_table_f32_f16[h[1]], ggml_table_f32_f16[h[0]]);
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
    for (int row = 0; row < 4; ++row) for (int lane = 0; lane < 8; ++lane) sums[row][lane] = _mm_setzero_ps();
    for (int i = 0; i < n; i += 32) {
        for (int lane = 0; lane < 8; ++lane) {
            const auto activation = load<Half>(y + (i + lane * 4) * width);
            for (int row = 0; row < 4; ++row)
                sums[row][lane] = _mm_add_ps(sums[row][lane],
                    _mm_mul_ps(load<Half>(x + row * stride + (i + lane * 4) * width), activation));
        }
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
// FUNC: native_dot4
// DESC: Accepts only complete four-row FP32/FP16 tiles with verified alignment; all other tensors retain upstream execution.
// ------------------=
extern "C" int native_dot4(int type, int n, float *out, const void *x, size_t stride, const void *y) {
    if (n <= 0 || n % 16 || (type != 0 && type != 1)) return 0;
#if defined(__x86_64__)
    if (n % 32) return 0;
#endif
    const uintptr_t mask = type == 1 ? 7 : 15;
    if (((uintptr_t)x | (uintptr_t)y | stride) & mask) return 0;
    if (type == 1) dot4<true>(n, out, (const unsigned char *)x, stride, (const unsigned char *)y);
    else dot4<false>(n, out, (const unsigned char *)x, stride, (const unsigned char *)y);
    return 1;
}
