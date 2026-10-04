#ifndef INFINITY_NATIVE_MATRIX_TILE_TEST_H
#define INFINITY_NATIVE_MATRIX_TILE_TEST_H

// ------------------------=
// FUNC: native_verify_matrix_dispatch
// DESC: Checks production chunk indexing against independent per-dot results across partial tiles, odd dimensions, broadcast planes and converted/noncontiguous activations.
// ------------------=
static size_t native_verify_matrix_dispatch(void) {
    _Alignas(64) unsigned char weights[8192];
    _Alignas(64) unsigned char activations[16384];
    _Alignas(64) unsigned char converted[16384];
    _Alignas(64) float actual[2048];
    float expected[2048];
    const int shapes[][2] = {{3, 1}, {17, 3}, {18, 19}, {19, 19}};
    size_t cases = 0;
    for (int type = 0; type <= 1; ++type) for (int packed = 0; packed <= type; ++packed) {
        for (size_t shape = 0; shape < sizeof(shapes) / sizeof(shapes[0]); ++shape) {
            const int rows = shapes[shape][0], columns = shapes[shape][1], n = 32;
            const size_t width = type == 1 ? 2 : 4;
            const size_t xs = n * width + 16, ys = n * width + (packed ? 0 : 32);
            struct ggml_tensor x = {0}, y = {0}, dst = {0};
            struct ggml_compute_params params = {0};
            params.nth = 1;
            params.wdata = converted;
            x.type = type; y.type = packed ? GGML_TYPE_F32 : type; dst.type = GGML_TYPE_F32;
            x.data = weights; y.data = activations; dst.data = actual;
            x.ne[0] = y.ne[0] = n; x.ne[1] = dst.ne[0] = rows;
            y.ne[1] = dst.ne[1] = columns; x.ne[2] = 2; y.ne[2] = dst.ne[2] = 4;
            x.ne[3] = y.ne[3] = dst.ne[3] = 1;
            x.nb[0] = width; x.nb[1] = xs; x.nb[2] = xs * rows; x.nb[3] = x.nb[2] * 2;
            y.nb[0] = packed ? 4 : width; y.nb[1] = packed ? n * 4 + 32 : ys;
            y.nb[2] = y.nb[1] * columns; y.nb[3] = y.nb[2] * 4;
            dst.nb[0] = 4; dst.nb[1] = (rows + 3) * 4;
            dst.nb[2] = dst.nb[1] * columns; dst.nb[3] = dst.nb[2] * 4;
            dst.src[0] = &x; dst.src[1] = &y;
            unsigned char *values = packed ? converted : activations;
            for (int row = 0; row < rows * 2; ++row) for (int k = 0; k < n; ++k) {
                const float value = (float)(((row * 17 + k * 13) % 257) - 128) / 31.0f;
                if (type == 1) ((ggml_fp16_t *)(weights + row * xs))[k] = GGML_CPU_FP32_TO_FP16(value);
                else ((float *)(weights + row * xs))[k] = value;
            }
            for (int col = 0; col < columns * 4; ++col) for (int k = 0; k < n; ++k) {
                const float value = (float)(((col * 29 + k * 7) % 193) - 96) / 19.0f;
                if (type == 1) ((ggml_fp16_t *)(values + col * ys))[k] = GGML_CPU_FP32_TO_FP16(value);
                else ((float *)(values + col * ys))[k] = value;
            }
            for (int partial = 0; partial <= 1; ++partial) {
                const int start_row = partial, end_row = rows - partial;
                const int start_col = partial, end_col = columns * 4 - partial;
                for (size_t i = 0; i < 2048; ++i) actual[i] = expected[i] = -12345.0f;
                ggml_compute_forward_mul_mat_one_chunk(&params, &dst, type, 1,
                    start_row, end_row, start_col, end_col);
                for (int col = start_col; col < end_col; ++col) for (int row = start_row; row < end_row; ++row)
                    type_traits_cpu[type].vec_dot(n, &expected[col * (rows + 3) + row], 0,
                        weights + (col / (columns * 2)) * x.nb[2] + row * xs, 0, values + col * ys, 0, 1);
                if (memcmp(actual, expected, sizeof(actual))) return 0;
                ++cases;
            }
        }
    }
    return cases;
}

// ------------------------=
// FUNC: native_verify_matrix_tiles
// DESC: Compares checked two-column production tiles bitwise with upstream dots across types, lengths, row counts, padding and alignment, including rejected shapes.
// ------------------=
size_t native_verify_matrix_tiles(void) {
    _Alignas(64) unsigned char weights[16 * 2176];
    _Alignas(64) unsigned char activations[2 * 2176];
    _Alignas(64) float actual[40];
    float expected[40];
    size_t cases = 0;
    const int lengths[] = {32, 64, 128, 256, 512};
    const int row_counts[] = {2, 4, 6, 16};
    for (int type = 0; type <= 1; ++type) {
        const size_t width = type == 1 ? 2 : 4;
        for (size_t ni = 0; ni < sizeof(lengths) / sizeof(lengths[0]); ++ni) {
            const int n = lengths[ni];
            for (size_t ri = 0; ri < sizeof(row_counts) / sizeof(row_counts[0]); ++ri) {
                const int rows = row_counts[ri];
                for (size_t padding = 0; padding <= 32; padding += 16) {
                    for (size_t offset = 0; offset <= 16; offset += 8) {
                        const size_t stride = n * width + padding;
                        unsigned char *x = weights + offset;
                        unsigned char *y = activations + offset;
                        for (int row = 0; row < rows; ++row) for (int k = 0; k < n; ++k) {
                            const float value = (float)(((row * 17 + k * 13) % 257) - 128) / 31.0f;
                            if (type == 1) ((ggml_fp16_t *)(x + row * stride))[k] = GGML_CPU_FP32_TO_FP16(value);
                            else ((float *)(x + row * stride))[k] = value;
                        }
                        for (int col = 0; col < 2; ++col) for (int k = 0; k < n; ++k) {
                            const float value = (float)(((col * 29 + k * 7) % 193) - 96) / 19.0f;
                            if (type == 1) ((ggml_fp16_t *)(y + col * stride))[k] = GGML_CPU_FP32_TO_FP16(value);
                            else ((float *)(y + col * stride))[k] = value;
                        }
                        for (size_t i = 0; i < 40; ++i) actual[i] = expected[i] = -12345.0f;
                        const int accepted = native_mat2(type, n, actual + 1, 20 * sizeof(float), x, stride, y, stride, rows);
                        if (offset == 8 && type == 0) {
                            if (accepted || memcmp(actual, expected, sizeof(actual))) return 0;
                        } else {
                            if (!accepted) return 0;
                            for (int col = 0; col < 2; ++col) for (int row = 0; row < rows; ++row)
                                type_traits_cpu[type].vec_dot(n, &expected[1 + col * 20 + row], 0,
                                    x + row * stride, 0, y + col * stride, 0, 1);
                            if (memcmp(actual, expected, sizeof(actual))) return 0;
                        }
                        ++cases;
                    }
                }
            }
        }
    }
    for (size_t i = 0; i < 40; ++i) actual[i] = expected[i] = -12345.0f;
    if (native_mat2(2, 32, actual, 80, weights, 128, activations, 128, 2) ||
        native_mat2(0, 31, actual, 80, weights, 128, activations, 128, 2) ||
        native_mat2(0, 32, actual, 80, weights, 128, activations, 128, 3) ||
        native_mat2(0, 32, actual, 80, weights, 64, activations, 128, 2) ||
        native_mat2(0, 32, actual, 80, weights, 128, activations, 64, 2) ||
        native_mat2(0, 32, actual, 4, weights, 128, activations, 128, 2) ||
        native_mat2(0, 32, actual, 80, weights + 1, 128, activations, 128, 2) ||
        native_mat2(0, 32, actual, 80, weights, 128, activations + 1, 128, 2) ||
        memcmp(actual, expected, sizeof(actual))) return 0;
    if (native_verify_matrix_dispatch() != 24) return 0;
    return cases + 8 + 24;
}
#endif
