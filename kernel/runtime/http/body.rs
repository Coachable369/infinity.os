use crate::response::Body;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Invalid,
    Truncated,
    Capacity,
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
