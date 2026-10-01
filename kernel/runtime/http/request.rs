#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidAuthority,
    InvalidTarget,
    InvalidHeader,
    Capacity,
}

// ------------------------=
// FUNC: get
// DESC: Builds a bounded HTTP GET without permitting header injection or implying TLS protection.
// ------------------=
pub fn get(host: &str, path: &str, output: &mut [u8]) -> Result<usize, Error> {
    get_with_headers(host, path, &[], output)
}

pub const HEADER_LIMIT: usize = 8192;

// ------------------------=
// FUNC: validate_headers
// DESC: Validates bounded end-to-end fields while reserving transport framing and authority to the native client.
// ------------------=
pub fn validate_headers(bytes: &[u8]) -> Result<(), Error> {
    if bytes.len() > HEADER_LIMIT { return Err(Error::Capacity); }
    let mut at=0;
    while at<bytes.len() {
        let end=bytes[at..].windows(2).position(|pair| pair==b"\r\n").ok_or(Error::InvalidHeader)?+at;
        let colon=bytes[at..end].iter().position(|b|*b==b':').ok_or(Error::InvalidHeader)?+at;
        let name=core::str::from_utf8(&bytes[at..colon]).map_err(|_|Error::InvalidHeader)?;
        if name.is_empty() || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b))
            || ["host","connection","transfer-encoding","content-length","accept-encoding","proxy-authorization","proxy-connection","upgrade","te","trailer"].iter().any(|reserved|name.eq_ignore_ascii_case(reserved))
            || !bytes[colon+1..end].iter().all(|b|*b==b'\t' || (*b>=32 && *b!=127)) {
            return Err(Error::InvalidHeader);
        }
        at=end+2;
    }
    Ok(())
}

// ------------------------=
// FUNC: get_with_headers
// DESC: Preserves engine-selected end-to-end request fields without allowing framing or authority injection.
// ------------------=
pub fn get_with_headers(host: &str, path: &str, headers: &[u8], output: &mut [u8]) -> Result<usize, Error> {
    validate_headers(headers)?;
    if host.is_empty()
        || host.len() > 253
        || !host
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-' || b == b':')
    {
        return Err(Error::InvalidAuthority);
    }
    if !path.starts_with('/')
        || !path
            .bytes()
            .all(|b| (0x21..=0x7e).contains(&b) && b != b'#')
    {
        return Err(Error::InvalidTarget);
    }
    let parts = [
        b"GET ".as_slice(),
        path.as_bytes(),
        b" HTTP/1.1\r\nHost: ",
        host.as_bytes(),
        if headers.is_empty() { b"\r\nConnection: close\r\n".as_slice() } else { b"\r\nConnection: keep-alive\r\n" },
        b"Accept-Encoding: identity\r\n",
        if headers.is_empty() { b"User-Agent: InfinityOS/0.1\r\n" } else { b"" },
        headers,
        b"\r\n",
    ];
    let length = parts
        .iter()
        .try_fold(0usize, |sum, p| sum.checked_add(p.len()))
        .ok_or(Error::Capacity)?;
    if length > output.len() {
        return Err(Error::Capacity);
    }
    let mut offset = 0;
    for part in parts {
        output[offset..offset + part.len()].copy_from_slice(part);
        offset += part.len();
    }
    Ok(offset)
}

#[cfg(test)]
mod tests {
    use super::*;
    // ------------------------=
    // FUNC: browser_fields_preserve_wire_values
    // DESC: Parses actual request bytes to verify engine cookies, language and authority survive serialization.
    // ------------------=
    #[test]
    fn browser_fields_preserve_wire_values() {
        let mut bytes=[0;12288];
        let size=get_with_headers("example.test","/search?q=hello",b"Cookie: sid=123; pref=blue\r\nAccept-Language: en-US\r\nUser-Agent: Servo\r\n",&mut bytes).unwrap();
        let mut fields=[httparse::EMPTY_HEADER;16];
        let mut request=httparse::Request::new(&mut fields);
        assert_eq!(request.parse(&bytes[..size]).unwrap(),httparse::Status::Complete(size));
        assert_eq!(request.method,Some("GET"));assert_eq!(request.path,Some("/search?q=hello"));
        for (name,value) in [("Host",b"example.test".as_slice()),("Cookie",b"sid=123; pref=blue"),("Accept-Encoding",b"identity"),("User-Agent",b"Servo"),("Connection",b"keep-alive")] {
            assert_eq!(request.headers.iter().filter(|h|h.name.eq_ignore_ascii_case(name)).map(|h|h.value).collect::<std::vec::Vec<_>>(),std::vec![value]);
        }
    }
    // ------------------------=
    // FUNC: browser_fields_reject_injection_and_overflow
    // DESC: Rejects alternate authorities, framing overrides, invalid controls and unbounded request metadata.
    // ------------------=
    #[test]
    fn browser_fields_reject_injection_and_overflow() {
        for raw in [b"Host: attacker.test\r\n".as_slice(),b"Content-Length: 4\r\n",b"Cookie: a\r\n\r\nGET / HTTP/1.1\r\n",b"Cookie: a\nb\r\n",b"Cookie: a\0b\r\n",b"Cookie: a",b" Bad: value\r\n",b"Accept-Encoding: gzip\r\n"] {
            assert_eq!(validate_headers(raw),Err(Error::InvalidHeader));
        }
        assert_eq!(validate_headers(&[b'x';HEADER_LIMIT+1]),Err(Error::Capacity));
        assert_eq!(get_with_headers("example.test","/",b"Cookie: a=b\r\n",&mut [0;32]),Err(Error::Capacity));
    }
}
