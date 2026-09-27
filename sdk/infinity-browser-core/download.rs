//! Bounded download staging, not a filesystem or ObjectStore implementation.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error { TooLarge, LengthMismatch, Closed, InvalidName, InvalidType }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Metadata {
    pub name: [u8; 63],
    pub name_length: usize,
    pub media_type: [u8; 127],
    pub type_length: usize,
}

// ------------------------=
// FUNC: attachment
// DESC: Parses a conservative attachment basename and MIME type without accepting paths or header injection.
// ------------------=
pub fn attachment(disposition: &str, media_type: &str) -> Result<Option<Metadata>, Error> {
    let mut fields = disposition.split(';');
    if !fields.next().unwrap_or("").trim().eq_ignore_ascii_case("attachment") { return Ok(None); }
    let mut filename = None;
    for field in fields {
        let Some((key, value)) = field.trim().split_once('=') else { continue; };
        if key.trim().eq_ignore_ascii_case("filename") {
            if filename.is_some() { return Err(Error::InvalidName); }
            let value = value.trim();
            let value = if value.starts_with('"') {
                value.strip_prefix('"').and_then(|v| v.strip_suffix('"')).ok_or(Error::InvalidName)?
            } else { value };
            if value.contains('"') { return Err(Error::InvalidName); }
            filename = Some(value);
        }
    }
    let name = filename.unwrap_or("download.bin");
    let media_type = media_type.split(';').next().unwrap_or("").trim();
    let media_type = if media_type.is_empty() { "application/octet-stream" } else { media_type };
    validate_metadata(name, media_type)?;
    let mut result = Metadata { name: [0; 63], name_length: name.len(),
        media_type: [0; 127], type_length: media_type.len() };
    result.name[..name.len()].copy_from_slice(name.as_bytes());
    result.media_type[..media_type.len()].copy_from_slice(media_type.as_bytes());
    Ok(Some(result))
}

pub struct Download<const N: usize> {
    bytes: [u8; N],
    length: usize,
    expected: Option<usize>,
    closed: bool,
}

impl<const N: usize> Download<N> {
    // ------------------------=
    // FUNC: new
    // DESC: Rejects declared oversize downloads before accepting any response body.
    // ------------------=
    pub fn new(expected: Option<usize>) -> Result<Self, Error> {
        if expected.is_some_and(|length| length > N) { return Err(Error::TooLarge); }
        Ok(Self { bytes: [0; N], length: 0, expected, closed: false })
    }
    // ------------------------=
    // FUNC: append
    // DESC: Accumulates whole chunks or invalidates the download without silently truncating it.
    // ------------------=
    pub fn append(&mut self, chunk: &[u8]) -> Result<(), Error> {
        if self.closed { return Err(Error::Closed); }
        let length = self.length.checked_add(chunk.len()).ok_or(Error::TooLarge);
        let error = match length {
            Ok(length) if length > N => Some(Error::TooLarge),
            Ok(length) if self.expected.is_some_and(|expected| length > expected) => Some(Error::LengthMismatch),
            Err(error) => Some(error),
            _ => None,
        };
        if let Some(error) = error { self.cancel(); return Err(error); }
        let length = length.unwrap();
        self.bytes[self.length..length].copy_from_slice(chunk);
        self.length = length;
        Ok(())
    }
    // ------------------------=
    // FUNC: finish
    // DESC: Exposes validated complete bytes once, for a separately authorized native object commit.
    // ------------------=
    pub fn finish(&mut self) -> Result<&[u8], Error> {
        if self.closed { return Err(Error::Closed); }
        if self.expected.is_some_and(|expected| expected != self.length) {
            self.cancel();
            return Err(Error::LengthMismatch);
        }
        self.closed = true;
        Ok(&self.bytes[..self.length])
    }
    // ------------------------=
    // FUNC: cancel
    // DESC: Discards staged content and prevents subsequent appends or commits.
    // ------------------=
    pub fn cancel(&mut self) {
        self.bytes[..self.length].fill(0);
        self.length = 0;
        self.closed = true;
    }
}

// ------------------------=
// FUNC: validate_metadata
// DESC: Accepts a bounded basename and ASCII media type, never an arbitrary namespace path.
// ------------------=
pub fn validate_metadata(name: &str, media_type: &str) -> Result<(), Error> {
    if name.is_empty() || name.len() > 63 || name == "." || name == ".."
        || name.chars().any(|c| c.is_control() || matches!(c, '/' | '\\' | ':')) {
        return Err(Error::InvalidName);
    }
    let Some((kind, subtype)) = media_type.split_once('/') else { return Err(Error::InvalidType); };
    if media_type.len() > 127 || kind.is_empty() || subtype.is_empty()
        || !kind.bytes().chain(subtype.bytes()).all(|c| c.is_ascii_alphanumeric() || b"!#$&^_.+-".contains(&c)) {
        return Err(Error::InvalidType);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    // ------------------------=
    // FUNC: attachment_metadata_preserves_types_and_rejects_paths
    // DESC: Exercises response metadata parsing without giving remote names namespace authority.
    // ------------------=
    #[test]
    fn attachment_metadata_preserves_types_and_rejects_paths() {
        let value = attachment("attachment; filename=\"notes.txt\"", "text/plain; charset=utf-8").unwrap().unwrap();
        assert_eq!(&value.name[..value.name_length], b"notes.txt");
        assert_eq!(&value.media_type[..value.type_length], b"text/plain");
        assert_eq!(attachment("inline; filename=notes.txt", "text/plain"), Ok(None));
        for value in ["attachment; filename=../secret", "attachment; filename=a; filename=b",
            "attachment; filename=\"unterminated", "attachment; filename=a\\b"] {
            assert_eq!(attachment(value, "text/plain"), Err(Error::InvalidName));
        }
        let fallback=attachment("attachment", "").unwrap().unwrap();
        assert_eq!(&fallback.name[..fallback.name_length], b"download.bin");
        assert_eq!(&fallback.media_type[..fallback.type_length], b"application/octet-stream");
    }
    // ------------------------=
    // FUNC: bounded_content_is_complete_or_rejected
    // DESC: Verifies byte preservation, single consumption, overflow and partial-body failure.
    // ------------------=
    #[test]
    fn bounded_content_is_complete_or_rejected() {
        let mut download = Download::<4>::new(Some(4)).unwrap();
        download.append(&[1, 2]).unwrap();
        download.append(&[3, 4]).unwrap();
        assert_eq!(download.finish(), Ok(&[1, 2, 3, 4][..]));
        assert_eq!(download.finish(), Err(Error::Closed));
        let mut download = Download::<4>::new(None).unwrap();
        assert_eq!(download.append(&[0; 5]), Err(Error::TooLarge));
        assert_eq!(download.finish(), Err(Error::Closed));
        let mut download = Download::<4>::new(Some(4)).unwrap();
        download.append(&[1]).unwrap();
        assert_eq!(download.finish(), Err(Error::LengthMismatch));
    }
    // ------------------------=
    // FUNC: metadata_cannot_choose_paths_or_inject_headers
    // DESC: Rejects path components and response metadata control characters.
    // ------------------=
    #[test]
    fn metadata_cannot_choose_paths_or_inject_headers() {
        assert_eq!(validate_metadata("note.txt", "text/plain"), Ok(()));
        for name in ["../note", "..", "C:\\note", "a/b", "a\n"] {
            assert_eq!(validate_metadata(name, "text/plain"), Err(Error::InvalidName));
        }
        assert_eq!(validate_metadata("note", "text/plain\r\nx: y"), Err(Error::InvalidType));
    }
}
