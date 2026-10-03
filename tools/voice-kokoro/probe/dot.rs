#[repr(C, align(16))]
struct Halves([u16; 80]);
#[repr(C, align(16))]
struct Singles([f32; 80]);
#[repr(C, align(16))]
struct Tile([u16; 640]);
#[repr(C, align(16))]
struct FloatTile([f32; 640]);
unsafe extern "C" {
    #[link_name = "infinity_kokoro_native_verify_im2col1d"]
    // ------------------------=
    // FUNC: verify_im2col1d
    // DESC: Returns the number of byte-identical native convolution expansion cases verified against the upstream implementation.
    // ------------------=
    fn verify_im2col1d() -> u64;
    #[link_name = "infinity_kokoro_native_dot4"]
    fn dot4(kind: i32, n: i32, out: *mut f32, x: *const u16, stride: usize, y: *const u16) -> i32;
    #[link_name = "infinity_kokoro_ggml_vec_dot_f16"]
    fn dot16(n: i32, s: *mut f32, bs: usize, x: *const u16, bx: usize, y: *const u16, by: usize, nrc: i32);
    #[link_name = "infinity_kokoro_ggml_vec_dot_f32"]
    fn dot32(n: i32, s: *mut f32, bs: usize, x: *const f32, bx: usize, y: *const f32, by: usize, nrc: i32);
}

// ------------------------=
// FUNC: verify_tiles
// DESC: Compares FP16/FP32 four-row tiles against production single-row dots and checks rejected inputs leave output untouched.
// ------------------=
pub unsafe fn verify_tiles() -> u64 {
    let mut cases = 0;
    for n in [16usize, 32, 48, 64, 128] {
        for padding in [0usize, 4, 8] {
            let stride = n + padding;
            let mut weights = Tile([0; 640]); let mut input = Tile([0; 640]);
            for row in 0..4 { for i in 0..n {
                weights.0[row * stride + i] = 0x3000 + (i as u16 * 13) + if (i + row) % 3 == 0 { 0x8000 } else { 0 };
                input.0[i] = 0x3400 + i as u16 * 7;
            }}
            let mut output = [123.0f32; 6];
            assert_eq!(dot4(1, n as i32, output.as_mut_ptr().add(1), weights.0.as_ptr(), stride * 2, input.0.as_ptr()), 1);
            assert_eq!(output[0], 123.0); assert_eq!(output[5], 123.0);
            for row in 0..4 {
                let mut expected = 0.0f32;
                dot16(n as i32, &mut expected, 0, weights.0.as_ptr().add(row * stride), 0, input.0.as_ptr(), 0, 1);
                assert_eq!(output[row + 1].to_bits(), expected.to_bits()); cases += 1;
            }
            let before = output;
            assert_eq!(dot4(1, n as i32 - 1, output.as_mut_ptr(), weights.0.as_ptr(), stride * 2, input.0.as_ptr()), 0);
            assert_eq!(dot4(1, n as i32, output.as_mut_ptr(), weights.0.as_ptr().add(1), stride * 2, input.0.as_ptr()), 0);
            assert_eq!(dot4(2, n as i32, output.as_mut_ptr(), weights.0.as_ptr(), stride * 2, input.0.as_ptr()), 0);
            assert_eq!(output, before); cases += 3;
            let mut float_weights = FloatTile([0.0; 640]);
            let mut float_input = FloatTile([0.0; 640]);
            for row in 0..4 { for i in 0..n {
                float_weights.0[row * stride + i] = f32::from_bits(0x3f000000 + i as u32 * 1024 + if (i + row) % 3 == 0 { 0x80000000 } else { 0 });
                float_input.0[i] = f32::from_bits(0x3f800000 + i as u32 * 4096);
            }}
            assert_eq!(dot4(0, n as i32, output.as_mut_ptr().add(1), float_weights.0.as_ptr().cast(), stride * 4, float_input.0.as_ptr().cast()), 1);
            assert_eq!(output[0], 123.0); assert_eq!(output[5], 123.0);
            for row in 0..4 {
                let mut expected = 0.0f32;
                dot32(n as i32, &mut expected, 0, float_weights.0.as_ptr().add(row * stride), 0, float_input.0.as_ptr(), 0, 1);
                assert_eq!(output[row + 1].to_bits(), expected.to_bits()); cases += 1;
            }
        }
    }
    cases
}

// ------------------------=
// FUNC: verify
// DESC: Checks aligned and fallback native dot products produce identical bits across row lengths, tails, and offsets.
// ------------------=
pub unsafe fn verify() -> u64 {
    assert!(verify_im2col1d() >= 1000);
    let mut cases = 0;
    for n in 0..=65usize {
        for offset_x in 0..8usize {
            for offset_y in 0..8usize {
                let mut ax = Halves([0; 80]); let mut ay = Halves([0; 80]);
                let mut bx = Halves([0; 80]); let mut by = Halves([0; 80]);
                let mut cx = Singles([0.0; 80]); let mut cy = Singles([0.0; 80]);
                let mut dx = Singles([0.0; 80]); let mut dy = Singles([0.0; 80]);
                for i in 0..n {
                    ax.0[i] = 0x3000 + (i as u16 * 17); ay.0[i] = 0x3400 + (i as u16 * 23);
                    bx.0[i + offset_x] = ax.0[i]; by.0[i + offset_y] = ay.0[i];
                    cx.0[i] = f32::from_bits(0x3f000000 + i as u32 * 1024);
                    cy.0[i] = f32::from_bits(0x3f800000 + i as u32 * 4096);
                    dx.0[i + offset_x] = cx.0[i]; dy.0[i + offset_y] = cy.0[i];
                }
                let mut expected = 0.0; let mut actual = 0.0;
                dot16(n as i32, &mut expected, 0, ax.0.as_ptr(), 0, ay.0.as_ptr(), 0, 1);
                dot16(n as i32, &mut actual, 0, bx.0.as_ptr().add(offset_x), 0, by.0.as_ptr().add(offset_y), 0, 1);
                assert_eq!(expected.to_bits(), actual.to_bits());
                dot32(n as i32, &mut expected, 0, cx.0.as_ptr(), 0, cy.0.as_ptr(), 0, 1);
                dot32(n as i32, &mut actual, 0, dx.0.as_ptr().add(offset_x), 0, dy.0.as_ptr().add(offset_y), 0, 1);
                assert_eq!(expected.to_bits(), actual.to_bits());
                cases += 2;
            }
        }
    }
    cases
}
