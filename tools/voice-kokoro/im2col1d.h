#pragma once

// ------------------------=
// FUNC: native_im2col1d_rows
// DESC: Expands one-dimensional convolution rows with padding bounds computed once per output position, preserving the upstream half conversion.
// ------------------=
template<bool FloatInput>
static void native_im2col1d_rows(const ggml_compute_params *params, ggml_tensor *dst) {
    const ggml_tensor *source = dst->src[1];
    const int64_t width = source->ne[0], channels = source->ne[1];
    const int64_t kernel = dst->src[0]->ne[0], outputs = dst->ne[1];
    const int32_t *options = reinterpret_cast<const int32_t *>(dst->op_params);
    const int64_t stride = options[0], padding = options[2], dilation = options[4];
    auto *destination = static_cast<ggml_fp16_t *>(dst->data);
    for (int64_t batch = 0; batch < source->ne[2]; ++batch) {
        for (int64_t x = 0; x < outputs; ++x) {
            const int64_t origin = x * stride - padding;
            const int64_t begin = std::min(kernel, std::max(int64_t(0), (-origin + dilation - 1) / dilation));
            const int64_t end = std::max(begin, std::min(kernel, (width - origin + dilation - 1) / dilation));
            ggml_fp16_t *row = destination + (batch * outputs + x) * channels * kernel;
            for (int64_t channel = params->ith; channel < channels; channel += params->nth) {
                ggml_fp16_t *out = row + channel * kernel;
                const char *input = static_cast<const char *>(source->data) + batch * source->nb[2] + channel * source->nb[1];
                for (int64_t k = 0; k < begin; ++k) out[k] = 0;
                if constexpr (FloatInput) {
                    const float *values = reinterpret_cast<const float *>(input);
                    for (int64_t k = begin; k < end; ++k)
                        out[k] = GGML_CPU_FP32_TO_FP16(values[origin + k * dilation]);
                } else {
                    const ggml_fp16_t *values = reinterpret_cast<const ggml_fp16_t *>(input);
                    for (int64_t k = begin; k < end; ++k) out[k] = values[origin + k * dilation];
                }
                for (int64_t k = end; k < kernel; ++k) out[k] = 0;
            }
        }
    }
}

// ------------------------=
// FUNC: native_im2col1d_f16
// DESC: Selects the shape-checked one-dimensional expansion while leaving all other convolution layouts on the upstream path.
// ------------------=
static bool native_im2col1d_f16(const ggml_compute_params *params, ggml_tensor *dst) {
    const ggml_tensor *source = dst->src[1];
    const int32_t *options = reinterpret_cast<const int32_t *>(dst->op_params);
    if (options[6] != 0 || options[3] != 0 || options[0] <= 0 || options[4] <= 0 || params->nth <= 0 ||
        dst->type != GGML_TYPE_F16 || source->ne[0] <= 0 || dst->src[0]->ne[0] <= 0 ||
        source->nb[0] != ggml_type_size(source->type)) return false;
    if (source->type == GGML_TYPE_F32) native_im2col1d_rows<true>(params, dst);
    else if (source->type == GGML_TYPE_F16) native_im2col1d_rows<false>(params, dst);
    else return false;
    return true;
}
