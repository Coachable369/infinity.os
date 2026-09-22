//! Pinned Hermes geometry; Qwen loading and model constants remain independent.
use super::*;

impl<'a> Weights<'a> {
    // ------------------------=
    // FUNC: load_hermes
    // DESC: Validates the exact Hermes Llama 3.2 3B geometry before using shared native kernels.
    // ------------------=
    pub fn load_hermes(model: Model<'a>) -> Result<Self, Error> {
        let (kind, mut arch) = model.metadata(b"general.architecture")?;
        if kind != 8 || arch.string()? != b"llama" {
            return Err(Error::Unsupported);
        }
        for (key, expected) in [
            (b"llama.block_count".as_slice(), 28),
            (b"llama.embedding_length", 3072),
            (b"llama.feed_forward_length", 8192),
            (b"llama.attention.head_count", 24),
            (b"llama.attention.head_count_kv", 8),
        ] {
            let (kind, mut value) = model.metadata(key)?;
            if kind != 4 || value.u32()? != expected {
                return Err(Error::Unsupported);
            }
        }
        for (key, expected) in [
            (b"llama.rope.freq_base".as_slice(), 500000f32),
            (b"llama.attention.layer_norm_rms_epsilon", 1e-5),
        ] {
            let (kind, mut value) = model.metadata(key)?;
            if kind != 6 || f32::from_bits(value.u32()?) != expected {
                return Err(Error::Unsupported);
            }
        }
        let embedding = checked(model.tensor(b"token_embd.weight")?, 3072, 128256)?;
        let norm = checked(model.tensor(b"output_norm.weight")?, 3072, 1)?;
        let first = layer(model, 0)?;
        let mut layers = [first; LAYERS];
        for (index, target) in layers.iter_mut().enumerate().take(28).skip(1) {
            *target = layer(model, index)?;
        }
        Ok(Self {
            embedding,
            output: embedding,
            norm,
            layers,
            ministral: false,
            hermes: true,
            heads: 24,
            width: 3072,
            hidden: 8192,
            vocabulary: 128256,
            layer_count: 28,
        })
    }
}

// ------------------------=
// FUNC: layer
// DESC: Resolves Hermes tensors without assuming Qwen Q/K normalization tensors exist.
// ------------------=
fn layer(model: Model<'_>, index: usize) -> Result<Layer<'_>, Error> {
    let find = |suffix: &[u8], width, rows| {
        let mut name = [0u8; 64];
        name[..4].copy_from_slice(b"blk.");
        let mut n = 4;
        if index >= 10 {
            name[n] = b'0' + (index / 10) as u8;
            n += 1;
        }
        name[n] = b'0' + (index % 10) as u8;
        n += 1;
        name[n] = b'.';
        n += 1;
        name[n..n + suffix.len()].copy_from_slice(suffix);
        checked(model.tensor(&name[..n + suffix.len()])?, width, rows)
    };
    let norm = find(b"attn_norm.weight", 3072, 1)?;
    Ok(Layer {
        norm,
        q: find(b"attn_q.weight", 3072, 3072)?,
        k: find(b"attn_k.weight", 3072, 1024)?,
        v: find(b"attn_v.weight", 3072, 1024)?,
        attention: find(b"attn_output.weight", 3072, 3072)?,
        ffn_norm: find(b"ffn_norm.weight", 3072, 1)?,
        gate: find(b"ffn_gate.weight", 3072, 8192)?,
        up: find(b"ffn_up.weight", 3072, 8192)?,
        down: find(b"ffn_down.weight", 8192, 3072)?,
    })
}

// ------------------------=
// FUNC: rotary_frequency
// DESC: Applies Llama 3.2 frequency scaling from the pinned model configuration.
// ------------------=
pub(super) fn rotary_frequency(index: usize) -> f32 {
    let inverse = 1.0 / libm::powf(500000.0, index as f32 / 64.0);
    let wavelength = 2.0 * core::f32::consts::PI / inverse;
    if wavelength < 2048.0 {
        inverse
    } else if wavelength > 8192.0 {
        inverse / 32.0
    } else {
        let smooth = (8192.0 / wavelength - 1.0) / 3.0;
        (1.0 - smooth) * inverse / 32.0 + smooth * inverse
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // ------------------------=
    // FUNC: distinct_head_geometries_keep_their_kv_groups
    // DESC: Exercises 24-head Hermes and unchanged 32-head Qwen attention against distinct KV values.
    // ------------------=
    #[test]
    fn distinct_head_geometries_keep_their_kv_groups() {
        for heads in [24, 32] {
            let query = vec![0.0; heads * 128];
            let mut output = vec![0.0; heads * 128];
            let mut kv = vec![0.0; KV_WIDTH * 2];
            for head in 0..8 {
                kv[KV_WIDTH + head * 128..KV_WIDTH + (head + 1) * 128].fill((head + 1) as f32);
            }
            for head in 0..heads {
                attend(&query, &kv, &mut [0.0], &mut output, 0, 0, head);
                let expected = (head / (heads / 8) + 1) as f32;
                assert!(output[head * 128..(head + 1) * 128].iter().all(|v| *v == expected));
            }
        }
    }
    // ------------------------=
    // FUNC: scaled_rope_preserves_high_and_reduces_low_frequencies
    // DESC: Checks pinned Llama scaling endpoints against an independent f64 calculation.
    // ------------------=
    #[test]
    fn scaled_rope_preserves_high_and_reduces_low_frequencies() {
        assert_eq!(rotary_frequency(0), 1.0);
        let expected = 500000f64.powf(-63.0 / 64.0) / 32.0;
        assert!((rotary_frequency(63) as f64 - expected).abs() < 1e-12);
    }
}
