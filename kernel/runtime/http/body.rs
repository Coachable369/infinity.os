use crate::response::Body;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Invalid,
    Truncated,
    Capacity,
}

#[derive(Default)]
pub struct Cursor { offset: usize }
impl Cursor {
    // ------------------------=
    // FUNC: complete
    // DESC: Checks only unconsumed chunk framing during streaming, compacting the body once at completion.
    // ------------------=
    pub fn complete(&self, bytes: &mut [u8], framing: Body, eof: bool) -> Result<Option<usize>, Error> {
        if framing == Body::Chunked {
            let pending = bytes.get(self.offset..).ok_or(Error::Invalid)?;
            if chunk_end(pending)?.is_none() {
                return if eof { Err(Error::Truncated) } else { Ok(None) };
            }
        }
        complete(bytes, framing, eof)
    }
    // ------------------------=
    // FUNC: next
    // DESC: Exposes each authenticated payload byte once, withholding chunk data until its framing is validated.
    // ------------------=
    pub fn next<'a>(&mut self, bytes: &'a [u8], framing: Body) -> Result<Option<&'a [u8]>, Error> {
        let start = self.offset;
        let end = match framing {
            Body::Empty => return Ok(None),
            Body::Length(length) => bytes.len().min(usize::try_from(length).map_err(|_| Error::Capacity)?),
            Body::UntilClose => bytes.len(),
            Body::Chunked => {
                let (prefix, length) = match httparse::parse_chunk_size(&bytes[start..]).map_err(|_| Error::Invalid)? {
                    httparse::Status::Partial => return Ok(None),
                    httparse::Status::Complete(value) => value,
                };
                if length == 0 { return Ok(None); }
                let payload = start.checked_add(prefix).ok_or(Error::Capacity)?;
                let end = payload.checked_add(usize::try_from(length).map_err(|_| Error::Capacity)?).ok_or(Error::Capacity)?;
                let next = end.checked_add(2).ok_or(Error::Capacity)?;
                if next > bytes.len() { return Ok(None); }
                if &bytes[end..next] != b"\r\n" { return Err(Error::Invalid); }
                self.offset = next;
                return Ok(Some(&bytes[payload..end]));
            },
        };
        if start >= end { return Ok(None); }
        self.offset = end;
        Ok(Some(&bytes[start..end]))
    }
}

#[cfg(test)]
mod streaming_tests {
    use super::*;
    // ------------------------=
    // FUNC: incremental_completion_preserves_payload_and_trailer_validation
    // DESC: Exercises many fragmented chunks, malformed trailers, and EOF through the production streaming completion path.
    // ------------------=
    #[test]
    fn incremental_completion_preserves_payload_and_trailer_validation() {
        let mut wire = b"1\r\nx\r\n".repeat(4096);
        wire.extend_from_slice(b"0\r\nX-Trace: yes\r\n\r\n");
        let mut cursor = Cursor::default();
        let mut received = std::vec::Vec::new();
        for n in 0..=wire.len() {
            while let Some(chunk) = cursor.next(&wire[..n], Body::Chunked).unwrap() {
                received.extend_from_slice(chunk);
            }
            let result = cursor.complete(&mut wire[..n], Body::Chunked, false).unwrap();
            if n < wire.len() { assert_eq!(result, None); }
            else { assert_eq!(result, Some(4096)); }
        }
        assert_eq!(received, std::vec![b'x'; 4096]);
        assert_eq!(&wire[..4096], received.as_slice());
        for mut bad in [b"1\r\nx\r\n0\r\nContent-Length: 1\r\n\r\n".to_vec(),
                        b"1\r\nx\r\n0\r\nTransfer-Encoding: chunked\r\n\r\n".to_vec()] {
            let mut cursor = Cursor::default();
            assert_eq!(cursor.next(&bad, Body::Chunked).unwrap(), Some(&b"x"[..]));
            assert_eq!(cursor.complete(&mut bad, Body::Chunked, false), Err(Error::Invalid));
        }
        let mut truncated = *b"1\r\n";
        assert_eq!(Cursor::default().complete(&mut truncated, Body::Chunked, true), Err(Error::Truncated));
    }
    // ------------------------=
    // FUNC: fragmented_payload_is_delivered_once_before_completion
    // DESC: Verifies early fixed-length and validated chunk payloads, exact concatenation, and malformed framing rejection.
    // ------------------=
    #[test]
    fn fragmented_payload_is_delivered_once_before_completion() {
        let mut cursor = Cursor::default();
        assert_eq!(cursor.next(b"abc", Body::Length(6)), Ok(Some(&b"abc"[..])));
        assert_eq!(cursor.next(b"abc", Body::Length(6)), Ok(None));
        assert_eq!(cursor.next(b"abcdefEXTRA", Body::Length(6)), Ok(Some(&b"def"[..])));
        let wire = b"3\r\nabc\r\n3\r\ndef\r\n0\r\n\r\n";
        let mut cursor = Cursor::default();
        let mut output = std::vec::Vec::new();
        for n in 0..=wire.len() {
            while let Some(chunk) = cursor.next(&wire[..n], Body::Chunked).unwrap() { output.extend_from_slice(chunk); }
            if n == 8 { assert_eq!(output, b"abc"); }
        }
        assert_eq!(output, b"abcdef");
        assert_eq!(Cursor::default().next(b"3\r\nabcXX", Body::Chunked), Err(Error::Invalid));
    }
}

// ------------------------=
// FUNC: chunk_end
// DESC: Validates chunk framing without modifying incomplete input, bounding chunk lengths and trailers.
// ------------------=
fn chunk_end(bytes: &[u8]) -> Result<Option<usize>, Error> {
    let mut at = 0usize;
    loop {
        let (prefix, length) =
            match httparse::parse_chunk_size(&bytes[at..]).map_err(|_| Error::Invalid)? {
                httparse::Status::Partial => return Ok(None),
                httparse::Status::Complete(value) => value,
            };
        at = at.checked_add(prefix).ok_or(Error::Capacity)?;
        let length = usize::try_from(length).map_err(|_| Error::Capacity)?;
        if length == 0 {
            let mut headers = [httparse::EMPTY_HEADER; 32];
            return match httparse::parse_headers(&bytes[at..], &mut headers)
                .map_err(|_| Error::Invalid)?
            {
                httparse::Status::Partial => Ok(None),
                httparse::Status::Complete((size, trailers)) => {
                    if trailers.iter().any(|h| {
                        h.name.eq_ignore_ascii_case("content-length")
                            || h.name.eq_ignore_ascii_case("transfer-encoding")
                    }) {
                        return Err(Error::Invalid);
                    }
                    Ok(Some(at + size))
                }
            };
        }
        let end = at.checked_add(length).ok_or(Error::Capacity)?;
        let next = end.checked_add(2).ok_or(Error::Capacity)?;
        if next > bytes.len() {
            return Ok(None);
        }
        if &bytes[end..next] != b"\r\n" {
            return Err(Error::Invalid);
        }
        at = next;
    }
}

// ------------------------=
// FUNC: complete
// DESC: Detects complete response bodies and compacts validated chunks in place only once all framing is present.
// ------------------=
pub fn complete(bytes: &mut [u8], framing: Body, eof: bool) -> Result<Option<usize>, Error> {
    let result = match framing {
        Body::Empty => Some(0),
        Body::Length(length) => {
            let length = usize::try_from(length).map_err(|_| Error::Capacity)?;
            if bytes.len() >= length {
                Some(length)
            } else {
                None
            }
        }
        Body::UntilClose => {
            if eof {
                Some(bytes.len())
            } else {
                None
            }
        }
        Body::Chunked => {
            if chunk_end(bytes)?.is_none() {
                None
            } else {
                let (mut at, mut out) = (0, 0);
                loop {
                    let httparse::Status::Complete((prefix, length)) =
                        httparse::parse_chunk_size(&bytes[at..]).map_err(|_| Error::Invalid)?
                    else {
                        return Err(Error::Invalid);
                    };
                    at += prefix;
                    let length = length as usize;
                    if length == 0 {
                        break;
                    }
                    bytes.copy_within(at..at + length, out);
                    out += length;
                    at += length + 2;
                }
                Some(out)
            }
        }
    };
    if eof && result.is_none() {
        Err(Error::Truncated)
    } else {
        Ok(result)
    }
}
