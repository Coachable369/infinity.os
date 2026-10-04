#ifndef INFINITY_NATIVE_MATMUL_PARALLEL_H
#define INFINITY_NATIVE_MATMUL_PARALLEL_H
#include "parallel.h"
#include <limits.h>

struct native_matmul_queue {
    const struct ggml_compute_params *params;
    struct ggml_tensor *dst;
    enum ggml_type type;
    int64_t vector_rows;
    int64_t rows, columns;
    int64_t row_chunks, column_chunks;
    int64_t row_step, column_step;
    int64_t next;
};

// ------------------------=
// FUNC: native_matmul_tiles
// DESC: Computes exclusively claimed existing GGML chunks without allocation, mutable model state, recursive dispatch or native fault unwinding.
// ------------------=
static void native_matmul_tiles(void *context) {
    struct native_matmul_queue *queue = (struct native_matmul_queue *)context;
    const int64_t total = queue->row_chunks * queue->column_chunks;
    for (;;) {
        const int64_t tile = __atomic_fetch_add(&queue->next, 1, __ATOMIC_RELAXED);
        if (tile >= total) return;
        const int64_t row_start = queue->row_step * (tile % queue->row_chunks);
        const int64_t col_start = queue->column_step * (tile / queue->row_chunks);
        const int64_t row_end = MIN(row_start + queue->row_step, queue->rows);
        const int64_t col_end = MIN(col_start + queue->column_step, queue->columns);
        int64_t vector_rows = queue->vector_rows;
        if ((queue->rows % 2 != 0) || (queue->dst->src[1]->ne[1] % 2 != 0) ||
            ((row_end - row_start) % 2 != 0) || ((col_end - col_start) % 2 != 0))
            vector_rows = 1;
        ggml_compute_forward_mul_mat_one_chunk(queue->params, queue->dst, queue->type,
            vector_rows, row_start, row_end, col_start, col_end);
    }
}

// ------------------------=
// FUNC: native_parallel_mul_mat
// DESC: Borrows a bounded optional native helper cohort for validated floating matrix tiles after activation conversion, joining all helpers before graph execution resumes.
// ------------------=
static bool native_parallel_mul_mat(const struct ggml_compute_params *params,
    struct ggml_tensor *dst, enum ggml_type type, int64_t vector_rows,
    int64_t rows, int64_t columns, int64_t row_chunks, int64_t column_chunks,
    int64_t row_step, int64_t column_step) {
    // No generic graph threading: integer/quantized kernels and unsupported
    // layouts retain upstream execution on the admitted speech worker.
    if (params->nth != 1 || params->ith != 0 ||
        (type != GGML_TYPE_F16 && type != GGML_TYPE_F32) ||
        rows <= 0 || columns <= 0 || row_chunks <= 0 || column_chunks <= 0 ||
        row_step <= 0 || column_step <= 0 || row_chunks > INT64_MAX / column_chunks ||
        rows > INT64_MAX / columns || dst->src[0]->ne[0] <= 0 ||
        rows * columns > INT64_MAX / dst->src[0]->ne[0] ||
        rows * columns * dst->src[0]->ne[0] < 262144 || row_chunks * column_chunks < 4)
        return false;
    struct native_matmul_queue queue = {params, dst, type, vector_rows, rows, columns,
        row_chunks, column_chunks, row_step, column_step, 0};
    // native_parallel_math is blocking: no helper outlives these stack fields,
    // converted activation storage, source tensors or destination buffers.
    native_parallel_math(native_matmul_tiles, &queue);
    return true;
}
#endif
