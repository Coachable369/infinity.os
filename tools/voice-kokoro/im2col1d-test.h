// ------------------------=
// FUNC: native_verify_im2col1d
// DESC: Compares optimized native convolution expansion with upstream output across input types, padding, dilation, strides, batches, channels, and worker partitions.
// ------------------=
extern "C" uint64_t native_verify_im2col1d(void) {
    alignas(16) float singles[1024];
    alignas(16) ggml_fp16_t halves[1024], expected[4096], actual[4096];
    for (size_t i = 0; i < 1024; ++i) {
        singles[i] = (static_cast<float>(i % 101) - 50.0f) / 17.0f;
        halves[i] = GGML_CPU_FP32_TO_FP16(singles[i]);
    }
    uint64_t cases = 0;
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
