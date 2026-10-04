// ------------------------=
// FUNC: native_verify_im2col_half_bits
// DESC: Requires identical half bits at every finite half value and rounding midpoint, plus signed exceptional and randomized float encodings.
// ------------------=
static uint64_t native_verify_im2col_half_bits() {
#if defined(__aarch64__)
    if (!native_im2col_half_conversion_available()) return 0;
    uint64_t cases = 0;
    for (uint32_t half = 0; half <= UINT16_MAX; ++half) {
        const float value = GGML_CPU_FP16_TO_FP32(static_cast<ggml_fp16_t>(half));
        if (native_im2col_fp32_to_fp16<true>(value) != GGML_CPU_FP32_TO_FP16(value)) return 0;
        ++cases;
    }
    for (uint32_t half = 0; half < 0x7bff; ++half) {
        const float lower = GGML_CPU_FP16_TO_FP32(static_cast<ggml_fp16_t>(half));
        const float upper = GGML_CPU_FP16_TO_FP32(static_cast<ggml_fp16_t>(half + 1));
        const uint32_t midpoint = fp32_to_bits((lower + upper) * 0.5f);
        for (const int offset : {-1, 0, 1}) for (const uint32_t sign : {0u, UINT32_C(0x80000000)}) {
            const float value = fp32_from_bits((midpoint + offset) | sign);
            if (native_im2col_fp32_to_fp16<true>(value) != GGML_CPU_FP32_TO_FP16(value)) return 0;
            ++cases;
        }
    }
    for (const uint32_t bits : {0u, 1u, 0x007fffffu, 0x00800000u, 0x33000000u,
                                0x387fc000u, 0x38800000u, 0x477fefffu, 0x477ff000u,
                                0x477ff001u, 0x7f7fffffu, 0x7f800000u, 0x7f800001u,
                                0x7fc00000u, 0x7fffffffu}) {
        for (const uint32_t sign : {0u, UINT32_C(0x80000000)}) {
            const float value = fp32_from_bits(bits | sign);
            if (native_im2col_fp32_to_fp16<true>(value) != GGML_CPU_FP32_TO_FP16(value)) return 0;
            ++cases;
        }
    }
    uint32_t random = 0x9e3779b9u;
    for (size_t i = 0; i < 65536; ++i) {
        random = random * 1664525u + 1013904223u;
        const float value = fp32_from_bits(random);
        if (native_im2col_fp32_to_fp16<true>(value) != GGML_CPU_FP32_TO_FP16(value)) return 0;
        ++cases;
    }
    return cases;
#else
    return native_im2col_half_conversion_available() ? 0 : 1;
#endif
}

// ------------------------=
// FUNC: native_verify_im2col_control_fallback
// DESC: Compares real convolution output under nondefault ARM rounding and denormal controls and restores the caller's FPCR.
// ------------------=
static uint64_t native_verify_im2col_control_fallback() {
#if defined(__aarch64__)
    uint64_t original;
    __asm__ volatile("mrs %0, fpcr" : "=r"(original));
    alignas(16) float singles[16];
    alignas(16) ggml_fp16_t expected[16], actual[16];
    for (size_t i = 0; i < 16; ++i) singles[i] = fp32_from_bits(0x38000000u + static_cast<uint32_t>(i) * 0x00101fffu);
    ggml_tensor weight{}, source{}, destination{};
    weight.ne[0] = 1;
    source.type = GGML_TYPE_F32;
    source.ne[0] = 16; source.ne[1] = source.ne[2] = source.ne[3] = 1;
    source.nb[0] = 4; source.nb[1] = source.nb[2] = source.nb[3] = 64;
    source.data = singles;
    destination.type = GGML_TYPE_F16;
    destination.src[0] = &weight; destination.src[1] = &source;
    destination.ne[0] = 1; destination.ne[1] = 16; destination.ne[2] = destination.ne[3] = 1;
    destination.nb[0] = destination.nb[1] = 2; destination.nb[2] = destination.nb[3] = 32;
    int32_t *options = reinterpret_cast<int32_t *>(destination.op_params);
    options[0] = options[1] = options[4] = options[5] = 1;
    ggml_compute_params params{};
    params.nth = 1;
    uint64_t cases = 0;
    bool valid = true;
    for (const uint64_t control : {UINT64_C(1) << 22, UINT64_C(2) << 22, UINT64_C(3) << 22,
                                   UINT64_C(1) << 24, UINT64_C(1) << 25, UINT64_C(1) << 26}) {
        __asm__ volatile("msr fpcr, %0; isb" :: "r"(control) : "memory");
        valid &= !native_im2col_half_conversion_available();
        destination.data = expected;
        ggml_compute_forward_im2col_f16(&params, &destination);
        destination.data = actual;
        valid &= native_im2col1d_f16(&params, &destination);
        valid &= memcmp(expected, actual, sizeof(expected)) == 0;
        ++cases;
    }
    __asm__ volatile("msr fpcr, %0; isb" :: "r"(original) : "memory");
    return valid ? cases : 0;
#else
    return 1;
#endif
}

// ------------------------=
// FUNC: native_verify_im2col1d
// DESC: Compares optimized native convolution expansion with upstream output across input types, padding, dilation, strides, batches, channels, and worker partitions.
// ------------------=
extern "C" uint64_t native_verify_im2col1d(void) {
    const uint64_t conversion_cases = native_verify_im2col_half_bits();
    const uint64_t fallback_cases = native_verify_im2col_control_fallback();
    if (!conversion_cases || !fallback_cases) return 0;
    alignas(16) float singles[1024];
    alignas(16) ggml_fp16_t halves[1024], expected[4096], actual[4096];
    for (size_t i = 0; i < 1024; ++i) {
        singles[i] = (static_cast<float>(i % 101) - 50.0f) / 17.0f;
        halves[i] = GGML_CPU_FP32_TO_FP16(singles[i]);
    }
    uint64_t cases = conversion_cases + fallback_cases;
    for (const int64_t width : {1, 2, 5, 16, 31})
    for (const int64_t kernel : {1, 3, 7})
    for (const int32_t stride : {1, 2, 3})
    for (const int32_t padding : {0, 1, 4})
    for (const int32_t dilation : {1, 2, 3})
    for (const int64_t channels : {1, 3})
    for (const int64_t batches : {1, 2})
    for (const auto type : {GGML_TYPE_F32, GGML_TYPE_F16})
    for (const int workers : {1, 2, 3}) {
        const int64_t extent = width + padding * 2 - dilation * (kernel - 1) - 1;
        if (extent < 0) continue;
        const int64_t outputs = extent / stride + 1;
        ggml_tensor weight{}, source{}, destination{};
        weight.ne[0] = kernel;
        source.type = type;
        source.ne[0] = width; source.ne[1] = channels; source.ne[2] = batches; source.ne[3] = 1;
        source.nb[0] = type == GGML_TYPE_F32 ? sizeof(float) : sizeof(ggml_fp16_t);
        source.nb[1] = (width + 3) * source.nb[0];
        source.nb[2] = (channels + 1) * source.nb[1];
        source.nb[3] = batches * source.nb[2];
        source.data = type == GGML_TYPE_F32 ? static_cast<void *>(singles + 1) : static_cast<void *>(halves + 1);
        destination.type = GGML_TYPE_F16;
        destination.src[0] = &weight; destination.src[1] = &source;
        destination.ne[0] = channels * kernel; destination.ne[1] = outputs; destination.ne[2] = batches; destination.ne[3] = 1;
        destination.nb[0] = sizeof(ggml_fp16_t);
        for (int i = 1; i < 4; ++i) destination.nb[i] = destination.nb[i - 1] * destination.ne[i - 1];
        int32_t *options = reinterpret_cast<int32_t *>(destination.op_params);
        options[0] = stride; options[1] = 1; options[2] = padding; options[3] = 0;
        options[4] = dilation; options[5] = 1; options[6] = 0;
        for (size_t i = 0; i < 4096; ++i) expected[i] = actual[i] = 0x55aa;
        for (int worker = 0; worker < workers; ++worker) {
            ggml_compute_params params{};
            params.ith = worker; params.nth = workers;
            destination.data = expected + 1;
            ggml_compute_forward_im2col_f16(&params, &destination);
            destination.data = actual + 1;
            if (!native_im2col1d_f16(&params, &destination)) return 0;
        }
        if (memcmp(expected, actual, sizeof(expected)) != 0) return 0;
        ++cases;
        if (workers == 1) {
            // Disjoint ranges deliberately execute out of order and split
            // inside batch boundaries. Every output and guard half must match.
            for (size_t i = 0; i < 4096; ++i) actual[i] = 0x55aa;
            ggml_compute_params params{};
            params.nth = 1;
            const int64_t rows = batches * outputs;
            const int64_t first = rows / 3, second = rows * 2 / 3;
            const int64_t ranges[][2] = {{second, rows}, {0, first}, {first, second}};
            for (const auto &range : ranges) {
                if (type == GGML_TYPE_F32)
                    native_im2col1d_rows<true>(&params, &destination, range[0], range[1]);
                else native_im2col1d_rows<false>(&params, &destination, range[0], range[1]);
            }
            if (memcmp(expected, actual, sizeof(expected)) != 0) return 0;
            ++cases;
        }
    }
    ggml_tensor weight{}, source{}, destination{};
    weight.ne[0] = 3;
    source.type = GGML_TYPE_F32; source.ne[0] = 5; source.nb[0] = sizeof(float);
    destination.type = GGML_TYPE_F16; destination.src[0] = &weight; destination.src[1] = &source;
    destination.data = actual;
    int32_t *options = reinterpret_cast<int32_t *>(destination.op_params);
    options[0] = 1; options[4] = 1;
    ggml_compute_params params{};
    params.nth = 1;
    for (const int unsupported : {3, 6}) {
        options[unsupported] = 1;
        if (native_im2col1d_f16(&params, &destination) || memcmp(expected, actual, sizeof(expected)) != 0) return 0;
        options[unsupported] = 0;
        ++cases;
    }
    return cases;
}
