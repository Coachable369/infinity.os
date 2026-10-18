//! Caller-owned CPU reference kernels for the independent image backend.
use crate::Error;

// ------------------------=
// FUNC: bf16
// DESC: Decodes little-endian BF16 weights without alignment assumptions.
// ------------------=
pub fn bf16(bytes: [u8; 2]) -> f32 {
    f32::from_bits((u16::from_le_bytes(bytes) as u32) << 16)
}

// ------------------------=
// FUNC: linear_bf16
// DESC: Multiplies one activation vector by a streamed row-major weight tile, without a full float weight copy.
// ------------------=
pub fn linear_bf16(input: &[f32], weights: &[u8], bias: Option<&[f32]>, out: &mut [f32]) -> Result<(), Error> {
    let size = input.len().checked_mul(out.len()).and_then(|n| n.checked_mul(2)).ok_or(Error::Bounds)?;
    if input.is_empty() || out.is_empty() || weights.len() != size || bias.is_some_and(|b| b.len() != out.len()) {
        return Err(Error::Shape);
    }
    if input.iter().any(|x| !x.is_finite()) || weights.chunks_exact(2).any(|b| !bf16([b[0], b[1]]).is_finite())
        || bias.is_some_and(|b| b.iter().any(|x| !x.is_finite())) { return Err(Error::NonFinite); }
    for (row, value) in out.iter_mut().enumerate() {
        let mut sum = bias.map_or(0.0, |b| b[row]);
        for (col, x) in input.iter().enumerate() {
            let i = (row * input.len() + col) * 2;
            sum += x * bf16([weights[i], weights[i + 1]]);
        }
        if !sum.is_finite() { return Err(Error::NonFinite); }
        *value = sum;
    }
    Ok(())
}

// ------------------------=
// FUNC: pack_latents
// DESC: Packs channel-first VAE latents into 2x2 image tokens in Qwen image channel/patch order.
// ------------------=
pub fn pack_latents(input: &[f32], channels: usize, height: usize, width: usize, out: &mut [f32]) -> Result<(), Error> {
    let n = channels.checked_mul(height).and_then(|n| n.checked_mul(width)).ok_or(Error::Bounds)?;
    if n == 0 || height % 2 != 0 || width % 2 != 0 || input.len() != n || out.len() != n { return Err(Error::Shape); }
    for y in 0..height/2 { for x in 0..width/2 { for c in 0..channels { for dy in 0..2 { for dx in 0..2 {
        out[((y * (width/2) + x) * channels + c) * 4 + dy * 2 + dx] = input[(c * height + y * 2 + dy) * width + x * 2 + dx];
    }}}}}
    Ok(())
}

// ------------------------=
// FUNC: unpack_latents
// DESC: Restores packed image tokens to the decoder's channel-first latent grid.
// ------------------=
pub fn unpack_latents(input: &[f32], channels: usize, height: usize, width: usize, out: &mut [f32]) -> Result<(), Error> {
    let n = channels.checked_mul(height).and_then(|n| n.checked_mul(width)).ok_or(Error::Bounds)?;
    if n == 0 || height % 2 != 0 || width % 2 != 0 || input.len() != n || out.len() != n { return Err(Error::Shape); }
    for y in 0..height/2 { for x in 0..width/2 { for c in 0..channels { for dy in 0..2 { for dx in 0..2 {
        out[(c * height + y * 2 + dy) * width + x * 2 + dx] = input[((y * (width/2) + x) * channels + c) * 4 + dy * 2 + dx];
    }}}}}
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    // ------------------------=
    // FUNC: linear_values_and_rejection
    // DESC: Tests real BF16 arithmetic, bias, nonfinite weights and malformed tile dimensions.
    // ------------------=
    fn linear_values_and_rejection() {
        let weights = [0x80,0x3f,0,0x40,0x80,0xbf,0,0x3f];
        let mut out = [0.;2];
        assert_eq!(linear_bf16(&[3.,4.],&weights,Some(&[1.,2.]),&mut out),Ok(()));
        assert_eq!(out,[12.,1.]);
        assert_eq!(linear_bf16(&[3.],&weights,None,&mut out),Err(Error::Shape));
        assert_eq!(linear_bf16(&[3.,4.],&[0x80,0x7f,0,0,0,0,0,0],None,&mut out),Err(Error::NonFinite));
    }
    #[test]
    // ------------------------=
    // FUNC: packing_order_and_roundtrip
    // DESC: Verifies independently enumerated multi-channel token order and exact inversion.
    // ------------------=
    fn packing_order_and_roundtrip() {
        let source = core::array::from_fn::<_,16,_>(|i|i as f32);
        let mut packed=[0.;16];let mut restored=[0.;16];
        pack_latents(&source,2,2,4,&mut packed).unwrap();
        assert_eq!(packed,[0.,1.,4.,5.,8.,9.,12.,13.,2.,3.,6.,7.,10.,11.,14.,15.]);
        unpack_latents(&packed,2,2,4,&mut restored).unwrap();assert_eq!(restored,source);
        assert_eq!(pack_latents(&source,2,1,8,&mut packed),Err(Error::Shape));
    }
}
