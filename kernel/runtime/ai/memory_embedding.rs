//! Native mean pooling of the pinned, pretrained Potion 2M English WordPiece table.
use super::{memory_search::EmbeddingProvider, persistent_memory::Error};

pub const MODEL: &[u8] = b"potion-2M-389b9f64-en-v1";
pub const DIM: usize = 64;
const WORDS: &[u8] = include_bytes!("../../../assets/ai-memory/tokens.bin");
const OFFSETS: &[u8] = include_bytes!("../../../assets/ai-memory/offsets.bin");
const WEIGHTS: &[u8] = include_bytes!("../../../assets/ai-memory/weights.bin");
pub struct Potion;

// ------------------------=
// FUNC: offset
// DESC: Reads a byte offset in the sorted vocabulary generated from the original tokenizer.
// ------------------=
fn offset(index: usize) -> usize {
    let at = index * 4;
    u32::from_le_bytes(OFFSETS[at..at + 4].try_into().unwrap()) as usize
}

// ------------------------=
// FUNC: lookup
// DESC: Finds an exact WordPiece and returns its learned embedding row.
// ------------------=
fn lookup(piece: &[u8]) -> Option<usize> {
    let mut lo = 0;
    let mut hi = OFFSETS.len() / 4 - 1;
    while lo < hi {
        let mid = (lo + hi) / 2;
        match WORDS[offset(mid)..offset(mid + 1)].cmp(piece) {
            core::cmp::Ordering::Equal => return Some(mid),
            core::cmp::Ordering::Less => lo = mid + 1,
            core::cmp::Ordering::Greater => hi = mid,
        }
    }
    None
}

// ------------------------=
// FUNC: accumulate
// DESC: Segments an English token greedily and excludes the entire token on an unknown continuation.
// ------------------=
fn accumulate(word: &[u8], sum: &mut [f32]) -> usize {
    if word.len() > 100 { return 0; }
    let mut pieces = [0usize; 100];
    let mut count = 0;
    let mut start = 0;
    while start < word.len() {
        let mut end = word.len();
        let mut found = None;
        while end > start {
            let mut key = [0u8; 102];
            let prefix = if start == 0 { 0 } else { key[..2].copy_from_slice(b"##"); 2 };
            let size = prefix + end - start;
            key[prefix..size].copy_from_slice(&word[start..end]);
            if let Some(row) = lookup(&key[..size]) { found = Some(row); break; }
            end -= 1;
        }
        let Some(row) = found else { return 0; };
        pieces[count] = row;
        count += 1;
        start = end;
    }
    for row in &pieces[..count] {
        let at = row * DIM * 4;
        for (value, bytes) in sum.iter_mut().zip(WEIGHTS[at..at + DIM * 4].chunks_exact(4)) {
            *value += f32::from_le_bytes(bytes.try_into().unwrap());
        }
    }
    count
}

impl EmbeddingProvider for Potion {
    // ------------------------=
    // FUNC: model
    // DESC: Identifies the immutable embedding space, independent of conversational model selection.
    // ------------------=
    fn model(&self) -> Result<(&[u8], usize), Error> { Ok((MODEL, DIM)) }

    // ------------------------=
    // FUNC: embed
    // DESC: Pools pretrained token vectors and normalizes the sentence, falling back to lexical search for unsupported scripts.
    // ------------------=
    fn embed(&mut self, text: &[u8], out: &mut [f32]) -> Result<usize, Error> {
        if out.len() < DIM { return Err(Error::DimensionMismatch); }
        if !text.is_ascii() { return Err(Error::Unavailable); }
        let out = &mut out[..DIM];
        out.fill(0.0);
        let mut pos = 0;
        while pos < text.len() {
            let start = pos;
            let byte = text[pos];
            pos += 1;
            if byte.is_ascii_control() || byte.is_ascii_whitespace() { continue; }
            if byte.is_ascii_alphanumeric() {
                while pos < text.len() && text[pos].is_ascii_alphanumeric() { pos += 1; }
            }
            let size = pos - start;
            if size <= 100 {
                let mut word = [0u8; 100];
                word[..size].copy_from_slice(&text[start..pos]);
                word[..size].make_ascii_lowercase();
                accumulate(&word[..size], out);
            }
        }
        let norm = libm::sqrtf(out.iter().map(|x| x*x).sum());
        if !norm.is_finite() || norm == 0.0 { return Err(Error::Unavailable); }
        for x in out { *x /= norm; }
        Ok(DIM)
    }
}
