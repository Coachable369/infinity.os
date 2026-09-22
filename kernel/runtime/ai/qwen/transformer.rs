//! Qwen3-8B CPU forward pass, cooperatively advanced in bounded row batches.
use super::{
    gguf::{Error, Model, Tensor},
    quant,
};
mod hermes;
pub const CONTEXT: usize = 4096;
pub const WIDTH: usize = 4096;
pub const HIDDEN: usize = 12288;
pub const KV_WIDTH: usize = 1024;
pub const LAYERS: usize = 36;
pub const VOCAB: usize = 151936;
pub const KV_FLOATS: usize = LAYERS * CONTEXT * KV_WIDTH * 2;
pub const WORK_FLOATS: usize = WIDTH * 4 + KV_WIDTH * 2 + HIDDEN * 3 + CONTEXT + VOCAB;

#[derive(Clone, Copy)]
struct Layer<'a> {
    norm: Tensor<'a>,
    q: Tensor<'a>,
    k: Tensor<'a>,
    v: Tensor<'a>,
    qnorm: Tensor<'a>,
    knorm: Tensor<'a>,
    attention: Tensor<'a>,
    ffn_norm: Tensor<'a>,
    gate: Tensor<'a>,
    up: Tensor<'a>,
    down: Tensor<'a>,
}
pub struct Weights<'a> {
    embedding: Tensor<'a>,
    output: Tensor<'a>,
    norm: Tensor<'a>,
    layers: [Layer<'a>; LAYERS],
    ministral: bool,
    hermes: bool,
    heads: usize,
    width: usize,
    hidden: usize,
    vocabulary: usize,
    layer_count: usize,
}
impl<'a> Weights<'a> {
    // ------------------------=
    // FUNC: load
    // DESC: Validates the two supported model geometries and resolves every tensor once.
    // ------------------=
    pub fn load(model: Model<'a>) -> Result<Self, Error> {
        let (kind, mut architecture) = model.metadata(b"general.architecture")?;
        let architecture = architecture.string()?;
        let ministral = architecture == b"mistral3";
        if kind != 8 || (!ministral && architecture != b"qwen3") {
            return Err(Error::Unsupported);
        }
        let (width, hidden, vocabulary, layer_count) = if ministral {
            (3072, 9216, 131072, 26)
        } else {
            (WIDTH, HIDDEN, VOCAB, LAYERS)
        };
        for (key, expected) in [
            (b"qwen3.block_count".as_slice(), layer_count as u32),
            (b"qwen3.embedding_length", width as u32),
            (b"qwen3.feed_forward_length", hidden as u32),
            (b"qwen3.attention.head_count", 32),
            (b"qwen3.attention.head_count_kv", 8),
            (b"qwen3.attention.key_length", 128),
            (b"qwen3.attention.value_length", 128),
        ] {
            let (kind, mut value) = model.metadata(model_key(key, ministral, &mut [0; 80]))?;
            if kind != 4 || value.u32()? != expected {
                return Err(Error::Unsupported);
            }
        }
        for (key, expected) in [
            (b"qwen3.rope.freq_base".as_slice(), 1_000_000f32),
            (
                b"qwen3.attention.layer_norm_rms_epsilon",
                if ministral { 1e-5 } else { 1e-6 },
            ),
        ] {
            let (kind, mut value) = model.metadata(model_key(key, ministral, &mut [0; 80]))?;
            if kind != 6 || f32::from_bits(value.u32()?) != expected {
                return Err(Error::Unsupported);
            }
        }
        let embedding = checked(model.tensor(b"token_embd.weight")?, width, vocabulary)?;
        if ministral {
            for (key, expected) in [
                (b"mistral3.rope.scaling.factor".as_slice(), 16.0f32),
                (b"mistral3.rope.scaling.yarn_beta_fast", 32.0),
                (b"mistral3.rope.scaling.yarn_beta_slow", 1.0),
                (b"mistral3.attention.temperature_scale", 0.1),
            ] {
                let (kind, mut value) = model.metadata(key)?;
                if kind != 6 || f32::from_bits(value.u32()?) != expected {
                    return Err(Error::Unsupported);
                }
            }
            let (_, mut original) =
                model.metadata(b"mistral3.rope.scaling.original_context_length")?;
            if original.u32()? != 16384 {
                return Err(Error::Unsupported);
            }
        }
        let output = checked(
            if ministral {
                embedding
            } else {
                model.tensor(b"output.weight")?
            },
            width,
            vocabulary,
        )?;
        let norm = checked(model.tensor(b"output_norm.weight")?, width, 1)?;
        let first = load_layer(model, 0, ministral, width, hidden)?;
        let mut layers = [first; LAYERS];
        for (i, layer) in layers.iter_mut().enumerate().take(layer_count).skip(1) {
            *layer = load_layer(model, i, ministral, width, hidden)?;
        }
        Ok(Self {
            embedding,
            output,
            norm,
            layers,
            ministral,
            hermes: false,
            heads: 32,
            width,
            hidden,
            vocabulary,
            layer_count,
        })
    }
}
// ------------------------=
// FUNC: checked
// DESC: Enforces a tensor's matrix geometry before any row operation.
// ------------------=
fn checked(t: Tensor<'_>, width: usize, rows: usize) -> Result<Tensor<'_>, Error> {
    if t.dimensions != [width, rows, 1, 1] || t.rank > 2 {
        Err(Error::Format)
    } else {
        Ok(t)
    }
}
// ------------------------=
// FUNC: load_layer
// DESC: Resolves the eleven tensors making up one standard Qwen3 decoder block.
// ------------------=
fn load_layer(
    model: Model<'_>,
    layer: usize,
    ministral: bool,
    width: usize,
    hidden: usize,
) -> Result<Layer<'_>, Error> {
    let find = |suffix: &[u8], width, rows| {
        let mut name = [0u8; 64];
        name[..4].copy_from_slice(b"blk.");
        let mut n = 4;
        if layer >= 10 {
            name[n] = b'0' + (layer / 10) as u8;
            n += 1;
        }
        name[n] = b'0' + (layer % 10) as u8;
        n += 1;
        name[n] = b'.';
        n += 1;
        name[n..n + suffix.len()].copy_from_slice(suffix);
        checked(model.tensor(&name[..n + suffix.len()])?, width, rows)
    };
    Ok(Layer {
        norm: find(b"attn_norm.weight", width, 1)?,
        q: find(b"attn_q.weight", width, WIDTH)?,
        k: find(b"attn_k.weight", width, KV_WIDTH)?,
        v: find(b"attn_v.weight", width, KV_WIDTH)?,
        qnorm: if ministral {
            find(b"attn_norm.weight", width, 1)?
        } else {
            find(b"attn_q_norm.weight", 128, 1)?
        },
        knorm: if ministral {
            find(b"attn_norm.weight", width, 1)?
        } else {
            find(b"attn_k_norm.weight", 128, 1)?
        },
        attention: find(b"attn_output.weight", WIDTH, width)?,
        ffn_norm: find(b"ffn_norm.weight", width, 1)?,
        gate: find(b"ffn_gate.weight", width, hidden)?,
        up: find(b"ffn_up.weight", width, hidden)?,
        down: find(b"ffn_down.weight", hidden, width)?,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Progress {
    Idle,
    Working,
    Token(u32),
    Prefilled,
    Cancelled,
    ContextFull,
}
pub struct Engine<'a, 'b> {
    weights: Weights<'a>,
    kv: &'b mut [f32],
    x: &'b mut [f32],
    normalized: &'b mut [f32],
    q: &'b mut [f32],
    attention: &'b mut [f32],
    k: &'b mut [f32],
    v: &'b mut [f32],
    gate: &'b mut [f32],
    up: &'b mut [f32],
    row: &'b mut [f32],
    scores: &'b mut [f32],
    logits: &'b mut [f32],
    position: usize,
    layer: usize,
    phase: u8,
    cursor: usize,
    cancelled: bool,
    active: bool,
    predict: bool,
    rotary: [(f32, f32); 64],
}
impl Drop for Engine<'_, '_> {
    // ------------------------=
    // FUNC: drop
    // DESC: Preserves weight lifetimes until all AP readers have retired.
    // ------------------=
    fn drop(&mut self) {
        #[cfg(all(target_arch = "aarch64", target_os = "none"))]
        super::workers::drain();
    }
}
impl<'a, 'b> Engine<'a, 'b> {
    // ------------------------=
    // FUNC: new
    // DESC: Binds caller-owned persistent memory; cache pages are initialized only when written.
    // ------------------=
    pub fn new(
        weights: Weights<'a>,
        kv: &'b mut [f32],
        work: &'b mut [f32],
    ) -> Result<Self, Error> {
        if kv.len() != KV_FLOATS || work.len() != WORK_FLOATS {
            return Err(Error::Format);
        }
        let (x, tail) = work.split_at_mut(weights.width);
        let (normalized, tail) = tail.split_at_mut(weights.width);
        let (q, tail) = tail.split_at_mut(weights.heads * 128);
        let (attention, tail) = tail.split_at_mut(weights.heads * 128);
        let (k, tail) = tail.split_at_mut(KV_WIDTH);
        let (v, tail) = tail.split_at_mut(KV_WIDTH);
        let (gate, tail) = tail.split_at_mut(weights.hidden);
        let (up, tail) = tail.split_at_mut(weights.hidden);
        let (row, tail) = tail.split_at_mut(HIDDEN);
        let (scores, tail) = tail.split_at_mut(CONTEXT);
        let (logits, _) = tail.split_at_mut(weights.vocabulary);
        Ok(Self {
            weights,
            kv,
            x,
            normalized,
            q,
            attention,
            k,
            v,
            gate,
            up,
            row,
            scores,
            logits,
            position: 0,
            layer: 0,
            phase: 0,
            cursor: 0,
            cancelled: false,
            active: false,
            predict: true,
            rotary: [(0.0, 1.0); 64],
        })
    }
    // ------------------------=
    // FUNC: reset
    // DESC: Starts a new sequence without clearing or reallocating a gigabyte-scale cache.
    // ------------------=
    pub fn reset(&mut self) {
        super::workers::discard();
        self.position = 0;
        self.active = false;
        self.cancelled = false;
    }
    // ------------------------=
    // FUNC: cancel
    // DESC: Cancels the next bounded work slice without performing further tensor work.
    // ------------------=
    pub fn cancel(&mut self) {
        super::workers::discard();
        self.cancelled = true;
    }
    // ------------------------=
    // FUNC: resume_prefix
    // DESC: Discards an unfinished token while retaining only fully computed KV positions.
    // ------------------=
    pub fn resume_prefix(&mut self) -> usize {
        super::workers::discard();
        self.active = false;
        self.cancelled = false;
        self.position
    }
    // ------------------------=
    // FUNC: begin
    // DESC: Embeds one input token and schedules its forward pass.
    // ------------------=
    pub fn begin(&mut self, token: u32) -> Result<Progress, Error> {
        self.begin_with_prediction(token, true)
    }
    // ------------------------=
    // FUNC: begin_with_prediction
    // DESC: Skips vocabulary projection for intermediate prompt tokens while preserving identical KV state.
    // ------------------=
    pub fn begin_with_prediction(&mut self, token: u32, predict: bool) -> Result<Progress, Error> {
        if self.active || token as usize >= self.weights.vocabulary {
            return Err(Error::Format);
        }
        if self.position == CONTEXT {
            return Ok(Progress::ContextFull);
        }
        quant::row(self.weights.embedding, token as usize, self.x)?;
        // Every layer and Q/K head uses the same angles at this position.
        // Compute 64 pairs once, rather than 92,160 soft-float trig calls.
        for (i, pair) in self.rotary.iter_mut().enumerate() {
            let frequency = rotary_frequency(i, self.weights.ministral);
            let angle = if self.weights.hermes {
                self.position as f32 * hermes::rotary_frequency(i)
            } else if self.weights.ministral {
                self.position as f32 * frequency
            } else {
                self.position as f32 / libm::powf(1_000_000.0, i as f32 / 64.0)
            };
            *pair = (libm::sinf(angle), libm::cosf(angle));
        }
        self.layer = 0;
        self.phase = 0;
        self.cursor = 0;
        self.active = true;
        self.predict = predict;
        Ok(Progress::Working)
    }
    // ------------------------=
    // FUNC: step
    // DESC: Executes at most eight matrix rows or one attention head and yields to the service pump.
    // ------------------=
    pub fn step(&mut self) -> Result<Progress, Error> {
        if self.cancelled {
            self.active = false;
            return Ok(Progress::Cancelled);
        }
        if !self.active {
            return Ok(Progress::Idle);
        }
        let layer = self.weights.layers[self.layer.min(self.weights.layer_count - 1)];
        match self.phase {
            0 => {
                rms_epsilon(
                    self.x,
                    self.normalized,
                    layer.norm,
                    if self.weights.ministral || self.weights.hermes {
                        1e-5
                    } else {
                        1e-6
                    },
                )?;
                self.phase = 1;
            }
            1 => {
                if mat_rows(layer.q, self.normalized, self.q, self.row, &mut self.cursor)? {
                    self.phase = 2;
                }
            }
            2 => {
                if mat_rows(layer.k, self.normalized, self.k, self.row, &mut self.cursor)? {
                    self.phase = 3;
                }
            }
            3 => {
                if mat_rows(layer.v, self.normalized, self.v, self.row, &mut self.cursor)? {
                    self.phase = 4;
                }
            }
            4 => {
                if self.weights.ministral || self.weights.hermes {
                    adjacent_rope(self.q, &self.rotary);
                    adjacent_rope(self.k, &self.rotary);
                } else {
                    norm_rope(self.q, layer.qnorm, &self.rotary)?;
                    norm_rope(self.k, layer.knorm, &self.rotary)?;
                }
                let offset = (self.layer * CONTEXT + self.position) * KV_WIDTH * 2;
                self.kv[offset..offset + KV_WIDTH].copy_from_slice(self.k);
                self.kv[offset + KV_WIDTH..offset + KV_WIDTH * 2].copy_from_slice(self.v);
                self.phase = 5;
            }
            5 => {
                attend(
                    self.q,
                    self.kv,
                    self.scores,
                    self.attention,
                    self.layer,
                    self.position,
                    self.cursor,
                );
                self.cursor += 1;
                if self.cursor == self.weights.heads {
                    self.cursor = 0;
                    self.phase = 6;
                }
            }
            6 => {
                if mat_rows(
                    layer.attention,
                    self.attention,
                    self.normalized,
                    self.row,
                    &mut self.cursor,
                )? {
                    self.phase = 7;
                }
            }
            7 => {
                for i in 0..self.weights.width {
                    self.x[i] += self.normalized[i];
                }
                rms_epsilon(
                    self.x,
                    self.normalized,
                    layer.ffn_norm,
                    if self.weights.ministral || self.weights.hermes {
                        1e-5
                    } else {
                        1e-6
                    },
                )?;
                self.phase = 8;
            }
            8 => {
                if mat_rows(
                    layer.gate,
                    self.normalized,
                    self.gate,
                    self.row,
                    &mut self.cursor,
                )? {
                    self.phase = 9;
                }
            }
            9 => {
                if mat_rows(
                    layer.up,
                    self.normalized,
                    self.up,
                    self.row,
                    &mut self.cursor,
                )? {
                    self.phase = 10;
                }
            }
            10 => {
                for i in 0..self.weights.hidden {
                    self.gate[i] = self.up[i] * self.gate[i] / (1.0 + libm::expf(-self.gate[i]));
                }
                self.phase = 11;
            }
            11 => {
                if mat_rows(
                    layer.down,
                    self.gate,
                    self.normalized,
                    self.row,
                    &mut self.cursor,
                )? {
                    self.phase = 12;
                }
            }
            12 => {
                for i in 0..self.weights.width {
                    self.x[i] += self.normalized[i];
                }
                self.layer += 1;
                if self.layer == self.weights.layer_count {
                    if !self.predict {
                        self.position += 1;
                        self.active = false;
                        return Ok(Progress::Prefilled);
                    }
                    rms_epsilon(
                        self.x,
                        self.normalized,
                        self.weights.norm,
                        if self.weights.ministral || self.weights.hermes {
                            1e-5
                        } else {
                            1e-6
                        },
                    )?;
                    self.phase = 13;
                } else {
                    self.phase = 0;
                }
            }
            13 => {
                if mat_rows(
                    self.weights.output,
                    self.normalized,
                    self.logits,
                    self.row,
                    &mut self.cursor,
                )? {
                    // Temperature-zero sampling is deterministic and has no RNG dependency.
                    let mut best = 0;
                    for i in 0..self.weights.vocabulary {
                        if !self.logits[i].is_finite() {
                            self.active = false;
                            return Err(Error::Format);
                        }
                        if self.logits[i] > self.logits[best] {
                            best = i;
                        }
                    }
                    self.position += 1;
                    self.active = false;
                    return Ok(Progress::Token(best as u32));
                }
            }
            _ => return Err(Error::Format),
        }
        Ok(Progress::Working)
    }
}
// ------------------------=
// FUNC: mat_rows
// DESC: Computes an eight-row slice without allocating or retaining dequantized matrices.
// ------------------=
fn mat_rows(
    t: Tensor<'_>,
    input: &[f32],
    output: &mut [f32],
    scratch: &mut [f32],
    cursor: &mut usize,
) -> Result<bool, Error> {
    let width = t.dimensions[0];
    if input.len() != width || output.len() != t.dimensions[1] || scratch.len() < width {
        return Err(Error::Format);
    }
    let end = (*cursor + 8).min(output.len());
    #[cfg(all(target_arch = "aarch64", target_os = "none"))]
    // SAFETY: the sole service-owned engine retains its weights until Drop drains APs.
    if let Some(done) = unsafe { super::workers::rows(t.kind, t.data, input, output, cursor) } {
        return Ok(done);
    }
    for at in *cursor..end {
        #[cfg(all(target_arch = "aarch64", target_os = "none"))]
        if t.kind == 12 || t.kind == 14 {
            let block_size = if t.kind == 12 { 144 } else { 210 };
            unsafe {
                extern "C" {
                    fn infinity_qwen_dot(
                        kind: u32,
                        data: *const u8,
                        input: *const f32,
                        width: usize,
                        output: *mut f32,
                    );
                }
                // Protect this row's FP registers from firmware IRQ handlers;
                // the surrounding kernel retains its pointer-only soft-float ABI.
                let saved: u64;
                let level: u64;
                let mut control: u64;
                core::arch::asm!("mrs {0}, daif","msr daifset, #2","mrs {1}, CurrentEL",out(reg)saved,out(reg)level);
                if level == 4 {
                    core::arch::asm!("mrs {0}, cpacr_el1",out(reg)control);
                    control |= 3 << 20;
                    core::arch::asm!("msr cpacr_el1, {0}","isb",in(reg)control);
                } else if level == 8 {
                    core::arch::asm!("mrs {0}, cptr_el2",out(reg)control);
                    control &= !(1 << 10);
                    core::arch::asm!("msr cptr_el2, {0}","isb",in(reg)control);
                }
                infinity_qwen_dot(
                    t.kind,
                    t.data.as_ptr().add(at * (width / 256) * block_size),
                    input.as_ptr(),
                    width,
                    &mut output[at],
                );
                core::arch::asm!("msr daif, {0}",in(reg)saved);
            }
            continue;
        }
        quant::row(t, at, &mut scratch[..width])?;
        let mut sums = [0f32; 4];
        for i in (0..width).step_by(4) {
            for j in 0..4 {
                sums[j] += scratch[i + j] * input[i + j];
            }
        }
        output[at] = (sums[0] + sums[1]) + (sums[2] + sums[3]);
    }
    *cursor = end;
    if end == output.len() {
        *cursor = 0;
        Ok(true)
    } else {
        Ok(false)
    }
}
// ------------------------=
// FUNC: rms
// DESC: Applies epsilon-stabilized RMS normalization and learned scale.
// ------------------=
fn rms(input: &[f32], output: &mut [f32], weights: Tensor<'_>) -> Result<(), Error> {
    rms_epsilon(input, output, weights, 1e-6)
}
// ------------------------=
// FUNC: rms_epsilon
// DESC: Applies the model-specific RMS stability constant.
// ------------------=
fn rms_epsilon(
    input: &[f32],
    output: &mut [f32],
    weights: Tensor<'_>,
    epsilon: f32,
) -> Result<(), Error> {
    quant::row(weights, 0, output)?;
    let scale =
        1.0 / libm::sqrtf(input.iter().map(|v| v * v).sum::<f32>() / input.len() as f32 + epsilon);
    for (value, x) in output.iter_mut().zip(input) {
        *value *= *x * scale;
    }
    Ok(())
}
// ------------------------=
// FUNC: norm_rope
// DESC: Applies Qwen3 per-head Q/K RMSNorm and split-half rotary position embeddings.
// ------------------=
fn norm_rope(values: &mut [f32], norm: Tensor<'_>, rotary: &[(f32, f32); 64]) -> Result<(), Error> {
    let mut normalized = [0f32; 128];
    for head in values.chunks_exact_mut(128) {
        rms(head, &mut normalized, norm)?;
        for i in 0..64 {
            let (s, c) = rotary[i];
            head[i] = normalized[i] * c - normalized[i + 64] * s;
            head[i + 64] = normalized[i] * s + normalized[i + 64] * c;
        }
    }
    Ok(())
}
// ------------------------=
// FUNC: attend
// DESC: Computes one causal grouped-query attention head against initialized KV positions only.
// ------------------=
fn attend(
    query: &[f32],
    kv: &[f32],
    scores: &mut [f32],
    output: &mut [f32],
    layer: usize,
    position: usize,
    head: usize,
) {
    let q = &query[head * 128..head * 128 + 128];
    let kv_head = head / (query.len() / KV_WIDTH);
    let base = layer * CONTEXT * KV_WIDTH * 2 + kv_head * 128;
    let mut max = f32::NEG_INFINITY;
    for time in 0..=position {
        let start = base + time * KV_WIDTH * 2;
        scores[time] = q
            .iter()
            .zip(&kv[start..start + 128])
            .map(|(a, b)| a * b)
            .sum::<f32>()
            * 0.08838834764831845;
        max = max.max(scores[time]);
    }
    let mut sum = 0f32;
    for score in &mut scores[..=position] {
        *score = libm::expf(*score - max);
        sum += *score;
    }
    let out = &mut output[head * 128..head * 128 + 128];
    out.fill(0.0);
    for time in 0..=position {
        let start = base + time * KV_WIDTH * 2 + KV_WIDTH;
        let weight = scores[time] / sum;
        for i in 0..128 {
            out[i] += weight * kv[start + i];
        }
    }
}

// ------------------------=
// FUNC: model_key
// DESC: Resolves metadata within the selected supported architecture namespace.
// ------------------=
fn model_key<'a>(key: &'a [u8], ministral: bool, buffer: &'a mut [u8; 80]) -> &'a [u8] {
    if !ministral {
        return key;
    }
    buffer[..8].copy_from_slice(b"mistral3");
    buffer[8..8 + key.len() - 5].copy_from_slice(&key[5..]);
    &buffer[..key.len() + 3]
}
// ------------------------=
// FUNC: rotary_frequency
// DESC: Computes pinned Qwen or Ministral YaRN frequencies; 4K positions need no temperature scaling.
// ------------------=
pub fn rotary_frequency(index: usize, ministral: bool) -> f32 {
    let inverse = 1.0 / libm::powf(1_000_000.0, index as f32 / 64.0);
    if !ministral {
        return inverse;
    }
    let correction = |rotations: f32| {
        128.0 * libm::logf(16384.0 / (rotations * 2.0 * core::f32::consts::PI))
            / (2.0 * libm::logf(1_000_000.0))
    };
    let low = libm::floorf(correction(32.0)).max(0.0);
    let high = libm::ceilf(correction(1.0)).min(127.0);
    let ramp = ((index as f32 - low) / (high - low)).clamp(0.0, 1.0);
    inverse * ((1.0 - ramp) + ramp / 16.0)
}
// ------------------------=
// FUNC: adjacent_rope
// DESC: Rotates adjacent pairs matching the pinned GGUF's permuted Ministral Q/K tensors.
// ------------------=
fn adjacent_rope(values: &mut [f32], rotary: &[(f32, f32); 64]) {
    for head in values.chunks_exact_mut(128) {
        for (i, &(s, c)) in rotary.iter().enumerate() {
            let (a, b) = (head[2 * i], head[2 * i + 1]);
            head[2 * i] = a * c - b * s;
            head[2 * i + 1] = a * s + b * c;
        }
    }
}
