//! Independent scalar GGUF Q4_K/Q6_K decoding. No host inference library.
use super::gguf::{Error, Tensor};

// ------------------------=
// FUNC: half
// DESC: Converts every IEEE binary16 value, including subnormals, to binary32.
// ------------------=
pub fn half(bits: u16) -> f32 {
    let sign = (bits as u32 & 0x8000) << 16;
    let exponent = (bits >> 10) & 31;
    let fraction = bits as u32 & 1023;
    if exponent == 0 {
        let value = fraction as f32 * (1.0 / 16777216.0);
        if sign != 0 {
            -value
        } else {
            value
        }
    } else if exponent == 31 {
        f32::from_bits(sign | 0x7f800000 | fraction << 13)
    } else {
        f32::from_bits(sign | ((exponent as u32 + 112) << 23) | fraction << 13)
    }
}
// ------------------------=
// FUNC: fp16
// DESC: Reads unaligned little-endian half precision data.
// ------------------=
fn fp16(bytes: &[u8]) -> f32 {
    half(u16::from_le_bytes([bytes[0], bytes[1]]))
}

// ------------------------=
// FUNC: decode_block
// DESC: Decodes one K-quant block to 256 float values.
// ------------------=
pub fn decode_block(kind: u32, bytes: &[u8], out: &mut [f32; 256]) -> Result<(), Error> {
    match kind {
        12 => {
            if bytes.len() != 144 {
                return Err(Error::Format);
            }
            let d = fp16(bytes);
            let min = fp16(&bytes[2..]);
            let scales = &bytes[4..16];
            for group in 0..8 {
                let (scale, minimum) = if group < 4 {
                    (scales[group] & 63, scales[group + 4] & 63)
                } else {
                    (
                        (scales[group + 4] & 15) | ((scales[group - 4] >> 6) << 4),
                        (scales[group + 4] >> 4) | ((scales[group] >> 6) << 4),
                    )
                };
                for lane in 0..32 {
                    let packed = bytes[16 + (group / 2) * 32 + lane];
                    let q = if group & 1 == 0 {
                        packed & 15
                    } else {
                        packed >> 4
                    };
                    out[group * 32 + lane] = d * scale as f32 * q as f32 - min * minimum as f32;
                }
            }
        }
        14 => {
            if bytes.len() != 210 {
                return Err(Error::Format);
            }
            let d = fp16(&bytes[208..]);
            for part in 0..2 {
                for lane in 0..32 {
                    let low = &bytes[part * 64..];
                    let hi = bytes[128 + part * 32 + lane];
                    let quants = [
                        low[lane] & 15,
                        low[lane + 32] & 15,
                        low[lane] >> 4,
                        low[lane + 32] >> 4,
                    ];
                    for group in 0..4 {
                        let q = (quants[group] | (((hi >> (group * 2)) & 3) << 4)) as i32 - 32;
                        let scale = bytes[192 + part * 8 + group * 2 + lane / 16] as i8;
                        out[part * 128 + group * 32 + lane] = d * scale as f32 * q as f32;
                    }
                }
            }
        }
        _ => return Err(Error::Unsupported),
    }
    Ok(())
}
// ------------------------=
// FUNC: row
// DESC: Dequantizes one matrix row into caller-owned memory.
// ------------------=
pub fn row(tensor: Tensor<'_>, index: usize, output: &mut [f32]) -> Result<(), Error> {
    let width = tensor.dimensions[0];
    if output.len() != width || index >= tensor.dimensions[1] || tensor.rank > 2 {
        return Err(Error::Format);
    }
    match tensor.kind {
        0 => {
            for (i, value) in output.iter_mut().enumerate() {
                let at = (index * width + i) * 4;
                *value = f32::from_le_bytes(tensor.data[at..at + 4].try_into().unwrap());
            }
        }
        1 => {
            for (i, value) in output.iter_mut().enumerate() {
                *value = fp16(&tensor.data[(index * width + i) * 2..]);
            }
        }
        kind @ (12 | 14) => {
            let size = if kind == 12 { 144 } else { 210 };
            let start = index * (width / 256) * size;
            for (block, chunk) in output.chunks_exact_mut(256).enumerate() {
                decode_block(
                    kind,
                    &tensor.data[start + block * size..start + (block + 1) * size],
                    chunk.try_into().unwrap(),
                )?;
            }
        }
        _ => return Err(Error::Unsupported),
    }
    Ok(())
}
