#pragma once
#include "parallel.h"

template<typename T> struct native_transpose_queue {
    ggml_tensor *destination;
    const T *kernel;
    const T *input;
    int64_t next;
};

// ------------------------=
// FUNC: native_transpose_rows
// DESC: Assigns whole output channels to helpers, preserving upstream dot products and accumulation order within each channel.
// ------------------=
template<typename T>
static void native_transpose_rows(void *context) {
    auto *queue = static_cast<native_transpose_queue<T> *>(context);
    ggml_tensor *dst = queue->destination;
    const int64_t channels = dst->src[0]->ne[2];
    const int64_t kernel = dst->src[0]->ne[0];
    const int64_t length = dst->src[1]->ne[0];
    const int32_t stride = reinterpret_cast<const int32_t *>(dst->op_params)[0];
    for (;;) {
        const int64_t row = __atomic_fetch_add(&queue->next, 1, __ATOMIC_RELAXED);
        if (row >= dst->ne[1]) return;
        float *output = reinterpret_cast<float *>(static_cast<char *>(dst->data) + row * dst->nb[1]);
        const T *weights = queue->kernel + row * channels * kernel;
        for (int64_t x = 0; x < length; ++x) {
            for (int64_t k = 0; k < kernel; ++k) {
                float value = 0;
                if constexpr (std::is_same_v<T, float>) {
                    ggml_vec_dot_f32(channels, &value, 0, queue->input + x * channels, 0,
                        weights + k * channels, 0, 1);
                } else {
                    // Upstream's half-dot signature predates const input types;
                    // it only reads these prepared buffers.
                    ggml_vec_dot_f16(channels, &value, 0, const_cast<T *>(queue->input + x * channels), 0,
                        const_cast<T *>(weights + k * channels), 0, 1);
                }
                output[x * stride + k] += value;
            }
        }
    }
}

// ------------------------=
// FUNC: native_transpose_parallel
// DESC: Borrows idle helpers only after upstream workspace preparation; joins all helpers before the workspace can be reused.
// ------------------=
template<typename T>
static bool native_transpose_parallel(const ggml_compute_params *params, ggml_tensor *dst,
    const T *kernel, const T *input) {
    if (params->nth != 1 || params->ith != 0 || dst->ne[1] < 4 ||
        dst->src[0]->ne[2] != dst->src[1]->ne[1] ||
        dst->nb[1] < static_cast<size_t>(dst->ne[0]) * sizeof(float)) return false;
    native_transpose_queue<T> queue{dst, kernel, input, 0};
    native_parallel_math(native_transpose_rows<T>, &queue);
    return true;
}
