//! GGUF byte-level BPE with explicit merge ranks and caller-owned indexes.
use super::gguf::{Error, Model};

#[derive(Clone, Copy)]
pub struct Entry<'a> {
    pub bytes: &'a [u8],
    pub id: u32,
}
pub const EMPTY: Entry<'static> = Entry { bytes: &[], id: 0 };
pub struct Tokenizer<'a, 'b> {
    vocabulary: &'b [Entry<'a>],
    merges: &'b [Entry<'a>],
    tekken: bool,
    llama: bool,
}
impl<'a, 'b> Tokenizer<'a, 'b> {
    // ------------------------=
    // FUNC: load
    // DESC: Indexes the artifact vocabulary and ordered BPE merge rules once at load time.
    // ------------------=
    pub fn load(
        model: Model<'a>,
        vocabulary: &'b mut [Entry<'a>],
        merges: &'b mut [Entry<'a>],
    ) -> Result<Self, Error> {
        for (key, target) in [
            (b"tokenizer.ggml.tokens".as_slice(), &mut *vocabulary),
            (b"tokenizer.ggml.merges".as_slice(), &mut *merges),
        ] {
            let (kind, mut values) = model.metadata(key)?;
            if kind != 9 || values.u32()? != 8 {
                return Err(Error::Format);
            }
            let count = values.u64()? as usize;
            if count != target.len() {
                return Err(Error::Format);
            }
            for (id, entry) in target.iter_mut().enumerate() {
                *entry = Entry {
                    bytes: values.string()?,
                    id: id as u32,
                };
            }
            target.sort_unstable_by(|a, b| a.bytes.cmp(b.bytes));
        }
        let (_, mut pre) = model.metadata(b"tokenizer.ggml.pre")?;
        let pre = pre.string()?;
        if pre != b"qwen2" && pre != b"tekken" && pre != b"llama-bpe" {
            return Err(Error::Unsupported);
        }
        Ok(Self {
            vocabulary,
            merges,
            tekken: pre == b"tekken",
            llama: pre == b"llama-bpe",
        })
    }
    // ------------------------=
    // FUNC: lookup
    // DESC: Looks up an exact byte string in a sorted immutable index.
    // ------------------=
    fn lookup(index: &[Entry<'_>], bytes: &[u8]) -> Option<u32> {
        index
            .binary_search_by(|e| e.bytes.cmp(bytes))
            .ok()
            .map(|i| index[i].id)
    }
    // ------------------------=
    // FUNC: special
    // DESC: Resolves an explicitly requested control token, separate from user input.
    // ------------------=
    pub fn special(&self, bytes: &[u8]) -> Result<u32, Error> {
        Self::lookup(self.vocabulary, bytes).ok_or(Error::Missing)
    }
    // ------------------------=
    // FUNC: encode
    // DESC: Encodes ordinary UTF-8 text without interpreting user-supplied control tokens.
    // ------------------=
    pub fn encode(&self, text: &str, output: &mut [u32]) -> Result<usize, Error> {
        let mut used = 0;
        let mut remaining = text;
        while !remaining.is_empty() {
            let length = if self.tekken {
                tekken_piece_length(remaining)?
            } else if self.llama && remaining.chars().next().is_some_and(char::is_numeric) {
                remaining
                    .chars()
                    .take_while(|c| c.is_numeric())
                    .take(3)
                    .map(char::len_utf8)
                    .sum()
            } else {
                piece_length(remaining)
            };
            used += self.piece(&remaining.as_bytes()[..length], &mut output[used..])?;
            remaining = &remaining[length..];
        }
        Ok(used)
    }
    // ------------------------=
    // FUNC: piece
    // DESC: Applies ranked adjacent merges within exactly one pretokenized piece.
    // ------------------=
    fn piece(&self, input: &[u8], output: &mut [u32]) -> Result<usize, Error> {
        // Bounded work: reject oversized pieces instead of silently changing segmentation.
        if input.len() > 1024 {
            return Err(Error::Unsupported);
        }
        let mut encoded = [0u8; 2048];
        let mut boundaries = [0usize; 1025];
        let mut end = 0;
        for (i, byte) in input.iter().enumerate() {
            boundaries[i] = end;
            let code = byte_character(*byte);
            end += code.encode_utf8(&mut encoded[end..]).len();
        }
        boundaries[input.len()] = end;
        let mut count = input.len();
        let mut pair = [0u8; 4097];
        while count > 1 {
            let mut best = u32::MAX;
            let mut selected = None;
            for i in 0..count - 1 {
                let left = &encoded[boundaries[i]..boundaries[i + 1]];
                let right = &encoded[boundaries[i + 1]..boundaries[i + 2]];
                pair[..left.len()].copy_from_slice(left);
                pair[left.len()] = b' ';
                pair[left.len() + 1..left.len() + 1 + right.len()].copy_from_slice(right);
                if let Some(rank) = Self::lookup(self.merges, &pair[..left.len() + 1 + right.len()])
                {
                    if rank < best {
                        best = rank;
                        selected = Some(i);
                    }
                }
            }
            let Some(i) = selected else {
                break;
            };
            boundaries.copy_within(i + 2..count + 1, i + 1);
            count -= 1;
        }
        if output.len() < count {
            return Err(Error::Overflow);
        }
        for i in 0..count {
            output[i] = self.special(&encoded[boundaries[i]..boundaries[i + 1]])?;
        }
        Ok(count)
    }
    // ------------------------=
    // FUNC: decode
    // DESC: Reverses byte-level encoding; output can contain a partial UTF-8 sequence.
    // ------------------=
    pub fn decode(&self, token: u32, output: &mut [u8]) -> Result<usize, Error> {
        let entry = self
            .vocabulary
            .iter()
            .find(|e| e.id == token)
            .ok_or(Error::Missing)?;
        let text = core::str::from_utf8(entry.bytes).map_err(|_| Error::Format)?;
        let mut used = 0;
        for c in text.chars() {
            let byte = (0..=255u16)
                .find(|b| byte_character(*b as u8) == c)
                .ok_or(Error::Format)? as u8;
            *output.get_mut(used).ok_or(Error::Overflow)? = byte;
            used += 1;
        }
        Ok(used)
    }
}
// ------------------------=
// FUNC: byte_character
// DESC: Maps all 256 bytes to the GPT-2 reversible Unicode alphabet used by Qwen.
// ------------------=
fn byte_character(byte: u8) -> char {
    let kept = |b: u8| (33..=126).contains(&b) || (161..=172).contains(&b) || b >= 174;
    if kept(byte) {
        byte as char
    } else {
        let rank = (0..byte).filter(|b| !kept(*b)).count();
        char::from_u32(256 + rank as u32).unwrap()
    }
}
// ------------------------=
// FUNC: tekken_piece_length
// DESC: Matches Tekken's case-sensitive ASCII branches; unsupported Unicode fails explicitly instead of mis-tokenizing.
// ------------------=
fn tekken_piece_length(text: &str) -> Result<usize, Error> {
    if !text.is_ascii() {
        return Err(Error::Unsupported);
    }
    let b = text.as_bytes();
    let mut start = 0;
    if !b[0].is_ascii_alphanumeric()
        && b[0] != b'\r'
        && b[0] != b'\n'
        && b.get(1).is_some_and(u8::is_ascii_alphabetic)
    {
        start = 1;
    }
    if b[start].is_ascii_alphabetic() {
        let mut end = start;
        while end < b.len() && b[end].is_ascii_uppercase() {
            end += 1;
        }
        while end < b.len() && b[end].is_ascii_lowercase() {
            end += 1;
        }
        return Ok(end);
    }
    // Numeric, punctuation, and whitespace alternatives are shared, except
    // Tekken has no contraction branch and accepts trailing slash after newline.
    if b[0] == b'\'' {
        return Ok(1);
    }
    let mut end = piece_length(text);
    if b[..end].iter().any(|c| *c == b'\r' || *c == b'\n') && !b[0].is_ascii_whitespace() {
        while end < b.len() && matches!(b[end], b'\r' | b'\n' | b'/') {
            end += 1;
        }
    }
    Ok(end)
}
// ------------------------=
// FUNC: piece_length
// DESC: Implements Qwen2 pretokenization for ASCII; rejects no bytes and preserves Unicode boundaries.
// ------------------=
fn piece_length(text: &str) -> usize {
    let mut chars = text.char_indices();
    let (_, first) = chars.next().unwrap();
    if first == '\'' {
        for suffix in ["s", "t", "re", "ve", "m", "ll", "d"] {
            if text
                .get(1..1 + suffix.len())
                .is_some_and(|v| v.eq_ignore_ascii_case(suffix))
            {
                return 1 + suffix.len();
            }
        }
    }
    let letter = |c: char| c.is_alphabetic();
    let number = |c: char| c.is_numeric();
    let mut length = first.len_utf8();
    let second = chars.clone().next().map(|(_, c)| c);
    if letter(first)
        || (first != '\r' && first != '\n' && !number(first) && second.is_some_and(letter))
    {
        for (at, c) in chars {
            if !letter(c) {
                break;
            }
            length = at + c.len_utf8();
        }
        return length;
    }
    if number(first) {
        return length;
    }
    let punctuation = |c: char| !c.is_whitespace() && !letter(c) && !number(c);
    if punctuation(first) || (first == ' ' && second.is_some_and(punctuation)) {
        let mut newline = false;
        for (at, c) in chars {
            if c == '\r' || c == '\n' {
                newline = true;
            } else if newline || !punctuation(c) {
                break;
            }
            length = at + c.len_utf8();
        }
        return length;
    }
    let mut last_newline = if first == '\r' || first == '\n' {
        length
    } else {
        0
    };
    let mut last_start = 0;
    for (at, c) in chars {
        if !c.is_whitespace() {
            break;
        }
        last_start = at;
        length = at + c.len_utf8();
        if c == '\r' || c == '\n' {
            last_newline = length;
        }
    }
    if last_newline != 0 {
        last_newline
    } else if length < text.len() && last_start > 0 {
        last_start
    } else {
        length
    }
}
