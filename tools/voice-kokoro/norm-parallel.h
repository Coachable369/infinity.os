#pragma once
#include "parallel.h"

struct native_norm_queue {
    ggml_compute_params parameters;
    ggml_tensor *destination;
    int next;
};

// ------------------------=
// FUNC: native_norm_stripes
// DESC: Runs unchanged upstream normalization on disjoint row stripes without altering each row's reduction order.
// ------------------=
static void native_norm_stripes(void *context) {
    auto *queue = static_cast<native_norm_queue *>(context);
    for (;;) {
        const int stripe = __atomic_fetch_add(&queue->next, 1, __ATOMIC_RELAXED);
        if (stripe >= 16) return;
        auto parameters = queue->parameters;
        parameters.ith = stripe;
        parameters.nth = 16;
        ggml_compute_forward_norm_f32(&parameters, queue->destination);
    }
}

// ------------------------=
// FUNC: native_norm_parallel
// DESC: Shares large contiguous normalization tensors with idle helpers while retaining single-worker fallback and alias safety.
// ------------------=
static bool native_norm_parallel(const ggml_compute_params *params, ggml_tensor *dst) {
    const ggml_tensor *src = dst->src[0];
    float epsilon;
    memcpy(&epsilon, dst->op_params, sizeof(epsilon));
    if (params->nth != 1 || params->ith != 0 || src->type != GGML_TYPE_F32 ||
        dst->type != GGML_TYPE_F32 || !ggml_are_same_shape(src, dst) ||
        !ggml_is_contiguous(src) || !ggml_is_contiguous(dst) ||
        src->ne[1] < 16 || ggml_nelements(src) < 32768 || !(epsilon >= 0.0f)) return false;
    const uintptr_t input = reinterpret_cast<uintptr_t>(src->data);
    const uintptr_t output = reinterpret_cast<uintptr_t>(dst->data);
    const size_t bytes = ggml_nbytes(src);
    if (input != output && (input < output ? output - input < bytes : input - output < bytes)) return false;
    native_norm_queue queue{*params, dst, 0};
    native_parallel_math(native_norm_stripes, &queue);
    return true;
}
