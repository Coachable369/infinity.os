//! Bounded spoken controls shared by the identity profile and transcript matcher.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SpokenPhrase([u8; 32], u8);
impl SpokenPhrase {
    // ------------------------=
    // FUNC: parse
    // DESC: Accepts one to four complete spoken words without punctuation or ambiguous whitespace.
    // ------------------=
    pub fn parse(value: &[u8]) -> Option<Self> {
        if value.is_empty() || value.len() > 32 || value.split(|b| *b == b' ').count() > 4
            || !value.split(|b| *b == b' ').all(|w| !w.is_empty() && w.iter().all(u8::is_ascii_alphabetic)) { return None; }
        let mut bytes = [0;32]; bytes[..value.len()].copy_from_slice(value);
        Some(Self(bytes, value.len() as u8))
    }
    // ------------------------=
    // FUNC: bytes
    // DESC: Borrows the validated phrase without padding.
    // ------------------=
    pub fn bytes(&self) -> &[u8] { &self.0[..self.1 as usize] }
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WakeWord { Infinity, Computer, Custom(SpokenPhrase) }
impl WakeWord {
    // ------------------------=
    // FUNC: parse
    // DESC: Keeps the existing recognition aliases for built-in words and matches custom phrases literally.
    // ------------------=
    pub fn parse(value: &[u8]) -> Option<Self> {
        if value.eq_ignore_ascii_case(b"Infinity") { Some(Self::Infinity) }
        else if value.eq_ignore_ascii_case(b"Computer") { Some(Self::Computer) }
        else { SpokenPhrase::parse(value).map(Self::Custom) }
    }
    // ------------------------=
    // FUNC: phrase
    // DESC: Returns the display and recognition spelling of this wake word.
    // ------------------=
    pub fn phrase(&self) -> &[u8] {
        match self { Self::Infinity => b"Infinity", Self::Computer => b"Computer", Self::Custom(p) => p.bytes() }
    }
    // ------------------------=
    // FUNC: next
    // DESC: Cycles the built-in options in Settings.
    // ------------------=
    pub const fn next(self) -> Self { match self { Self::Infinity => Self::Computer, _ => Self::Infinity } }
}
