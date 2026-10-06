//! Strict bounded HTTP/1 response headers; body decoding is a separate layer.
pub const HEADER_LIMIT: usize = 8192;
pub const HEADER_COUNT: usize = 128;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    TooLarge,
    Invalid,
    AmbiguousFraming,
    UnsupportedEncoding,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Body {
    Empty,
    Length(u64),
    Chunked,
    UntilClose,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Head {
    pub status: u16,
    pub bytes: usize,
    pub body: Body,
}

/// A borrowed, allocation-free view of the preserved wire head. Repeated fields
/// remain separate (notably Set-Cookie); callers apply field-specific policy.
pub struct Headers<'a> {
    fields: [httparse::Header<'a>; HEADER_COUNT],
    length: usize,
}

impl<'a> Headers<'a> {
    // ------------------------=
    // FUNC: parse
    // DESC: Validates framing before exposing a complete response's header fields.
    // ------------------=
    pub fn parse(bytes: &'a [u8]) -> Result<Self, Error> {
        let head = parse(bytes, false)?.ok_or(Error::Invalid)?;
        let mut fields = [httparse::EMPTY_HEADER; HEADER_COUNT];
        let mut response = httparse::Response::new(&mut fields);
        response.parse(&bytes[..head.bytes]).map_err(|_| Error::Invalid)?;
        let length = response.headers.len();
        Ok(Self { fields, length })
    }

    // ------------------------=
    // FUNC: values
    // DESC: Enumerates case-insensitive field values without merging repeated fields or copying bytes.
    // ------------------=
    pub fn values<'b>(&'b self, name: &'b str) -> impl Iterator<Item = &'a [u8]> + 'b {
        self.fields[..self.length].iter()
            .filter(move |field| field.name.eq_ignore_ascii_case(name))
            .map(|field| field.value)
    }

    // ------------------------=
    // FUNC: iter
    // DESC: Preserves every validated response field for native browser security and content processing.
    // ------------------=
    pub fn iter(&self) -> impl Iterator<Item = (&'a str, &'a [u8])> + '_ {
        self.fields[..self.length].iter().map(|field| (field.name, field.value))
    }
}

// ------------------------=
// FUNC: parse
// DESC: Parses complete or fragmented response headers and rejects conflicting framing before body consumption.
// ------------------=
pub fn parse(bytes: &[u8], head_request: bool) -> Result<Option<Head>, Error> {
    let mut headers = [httparse::EMPTY_HEADER; HEADER_COUNT];
    let mut response = httparse::Response::new(&mut headers);
    let size = match response
        .parse(&bytes[..bytes.len().min(HEADER_LIMIT)])
        .map_err(|_| Error::Invalid)?
    {
        httparse::Status::Partial if bytes.len() >= HEADER_LIMIT => return Err(Error::TooLarge),
        httparse::Status::Partial => return Ok(None),
        httparse::Status::Complete(size) => size,
    };
    let status = response.code.ok_or(Error::Invalid)?;
    if !(100..=599).contains(&status) {
        return Err(Error::Invalid);
    }
    let mut length = None;
    let mut chunked = false;
    for header in response.headers {
        if header.name.eq_ignore_ascii_case("content-length") {
            if length.is_some() {
                return Err(Error::AmbiguousFraming);
            }
            let text = core::str::from_utf8(header.value)
                .map_err(|_| Error::Invalid)?
                .trim();
            if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
                return Err(Error::Invalid);
            }
            length = Some(text.parse::<u64>().map_err(|_| Error::TooLarge)?);
        } else if header.name.eq_ignore_ascii_case("transfer-encoding") {
            if chunked {
                return Err(Error::AmbiguousFraming);
            }
            if !header.value.eq_ignore_ascii_case(b"chunked") {
                return Err(Error::UnsupportedEncoding);
            }
            chunked = true;
        }
    }
    if chunked && length.is_some() {
        return Err(Error::AmbiguousFraming);
    }
    let body = if head_request || status < 200 || status == 204 || status == 304 {
        Body::Empty
    } else if chunked {
        Body::Chunked
    } else if let Some(length) = length {
        Body::Length(length)
    } else {
        Body::UntilClose
    };
    Ok(Some(Head {
        status,
        bytes: size,
        body,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    // ------------------------=
    // FUNC: iteration_preserves_repeated_fields_and_binary_values
    // DESC: Verifies lossless protocol metadata and rejects ambiguous framing before exposing fields.
    // ------------------=
    #[test]
    fn iteration_preserves_repeated_fields_and_binary_values() {
        let bytes = b"HTTP/1.1 200 OK\r\nSet-Cookie: a=1\r\nSet-Cookie: b=2\r\nX-Value: \x80\r\nContent-Length: 0\r\n\r\n";
        let headers = Headers::parse(bytes).unwrap();
        let fields: std::vec::Vec<_> = headers.iter().collect();
        assert_eq!(fields.len(), 4);
        assert_eq!(fields[0], ("Set-Cookie", b"a=1".as_slice()));
        assert_eq!(fields[1], ("Set-Cookie", b"b=2".as_slice()));
        assert_eq!(fields[2], ("X-Value", b"\x80".as_slice()));
        assert!(Headers::parse(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nTransfer-Encoding: chunked\r\n\r\n").is_err());
    }
    // ------------------------=
    // FUNC: many_security_and_cookie_fields_remain_bounded
    // DESC: Accepts realistic field counts without weakening framing checks or allowing unbounded metadata.
    // ------------------=
    #[test]
    fn many_security_and_cookie_fields_remain_bounded() {
        let mut bytes=std::string::String::from("HTTP/1.1 200 OK\r\n");
        for _ in 0..80 {bytes.push_str("Set-Cookie: a=b; Secure\r\n");}
        bytes.push_str("Content-Length: 0\r\n\r\n");
        assert_eq!(Headers::parse(bytes.as_bytes()).unwrap().values("set-cookie").count(),80);
        assert!(Headers::parse(bytes.replace("Content-Length: 0", "Content-Length: 0\r\nContent-Length: 1").as_bytes()).is_err());
        let too_many=std::format!("HTTP/1.1 200 OK\r\n{}\r\n","X: y\r\n".repeat(HEADER_COUNT+1));
        assert!(Headers::parse(too_many.as_bytes()).is_err());
    }
}
