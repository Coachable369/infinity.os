#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidAuthority,
    InvalidTarget,
    InvalidHeader,
    InvalidMethod,
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
pub const BODY_LIMIT: usize = 64 * 1024;

// ------------------------=
// FUNC: validate_method
// DESC: Admits standard document and fetch methods without exposing CONNECT, TRACE, or arbitrary wire tokens.
// ------------------=
pub fn validate_method(method:&str)->Result<(),Error> {
    if matches!(method,"GET"|"HEAD"|"POST"|"PUT"|"PATCH"|"DELETE"|"OPTIONS") {Ok(())} else {Err(Error::InvalidMethod)}
}

// ------------------------=
// FUNC: validate_headers
// DESC: Validates bounded end-to-end fields while reserving transport framing and authority to the native client.
// ------------------=
pub fn validate_headers(bytes: &[u8]) -> Result<(), Error> {
    if bytes.len() > HEADER_LIMIT { return Err(Error::Capacity); }
    let mut at=0;
    let mut encoding_seen=false;
    while at<bytes.len() {
        let end=bytes[at..].windows(2).position(|pair| pair==b"\r\n").ok_or(Error::InvalidHeader)?+at;
        let colon=bytes[at..end].iter().position(|b|*b==b':').ok_or(Error::InvalidHeader)?+at;
        let name=core::str::from_utf8(&bytes[at..colon]).map_err(|_|Error::InvalidHeader)?;
        if name.is_empty() || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b))
            || ["host","connection","transfer-encoding","content-length","proxy-authorization","proxy-connection","upgrade","te","trailer"].iter().any(|reserved|name.eq_ignore_ascii_case(reserved))
            || !bytes[colon+1..end].iter().all(|b|*b==b'\t' || (*b>=32 && *b!=127)) {
            return Err(Error::InvalidHeader);
        }
        if name.eq_ignore_ascii_case("accept-encoding") {
            if encoding_seen {return Err(Error::InvalidHeader);}encoding_seen=true;
            let value=core::str::from_utf8(&bytes[colon+1..end]).map_err(|_|Error::InvalidHeader)?.trim();
            if !matches!(value,"gzip"|"identity") {return Err(Error::InvalidHeader);}
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
    head(host,path,"GET",headers,0,output)
}

// ------------------------=
// FUNC: head
// DESC: Serializes a validated method and native-owned body length while preserving origin and cookie metadata.
// ------------------=
pub fn head(host:&str,path:&str,method:&str,headers:&[u8],body_length:usize,output:&mut[u8])->Result<usize,Error> {
    validate_method(method)?;
    if body_length>BODY_LIMIT {return Err(Error::Capacity);}
    if body_length!=0 && matches!(method,"GET"|"HEAD") {return Err(Error::InvalidMethod);}
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
    let mut digits=[0u8;20];let mut at=digits.len();let mut remaining=body_length;
    loop {at-=1;digits[at]=b'0'+(remaining%10) as u8;remaining/=10;if remaining==0 {break;}}
    let has_length=!matches!(method,"GET"|"HEAD");
    let parts = [
        method.as_bytes(),b" ",
        path.as_bytes(),
        b" HTTP/1.1\r\nHost: ",
        host.as_bytes(),
        if headers.is_empty() { b"\r\nConnection: close\r\n".as_slice() } else { b"\r\nConnection: keep-alive\r\n" },
        if headers.split(|b|*b==b'\n').any(|line|line.get(..16).is_some_and(|name|name.eq_ignore_ascii_case(b"accept-encoding:"))) {b"".as_slice()}else{b"Accept-Encoding: identity\r\n"},
        if headers.is_empty() { b"User-Agent: InfinityOS/0.1\r\n" } else { b"" },
        headers,
        if has_length {b"Content-Length: ".as_slice()} else {b""},
        if has_length {&digits[at..]} else {b""},
        if has_length {b"\r\n".as_slice()} else {b""},
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
    // FUNC: negotiated_encoding_has_one_wire_field
    // DESC: Parses serialized requests to verify compression negotiation without duplicate defaults.
    // ------------------=
    #[test]
    fn negotiated_encoding_has_one_wire_field() {
        for (headers, expected) in [(b"Accept-Encoding: gzip\r\n".as_slice(), b"gzip".as_slice()),
            (b"accept-encoding: identity\r\n", b"identity"), (b"", b"identity")] {
            let mut bytes=[0;512];
            let size=get_with_headers("example.test","/",headers,&mut bytes).unwrap();
            let mut fields=[httparse::EMPTY_HEADER;16];
            let mut request=httparse::Request::new(&mut fields);
            assert_eq!(request.parse(&bytes[..size]).unwrap(),httparse::Status::Complete(size));
            let values=request.headers.iter().filter(|h|h.name.eq_ignore_ascii_case("accept-encoding"))
                .map(|h|h.value).collect::<std::vec::Vec<_>>();
            assert_eq!(values,std::vec![expected]);
        }
    }
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
        for raw in [b"Host: attacker.test\r\n".as_slice(),b"Content-Length: 4\r\n",b"Cookie: a\r\n\r\nGET / HTTP/1.1\r\n",b"Cookie: a\nb\r\n",b"Cookie: a\0b\r\n",b"Cookie: a",b" Bad: value\r\n",b"Accept-Encoding: br\r\n",b"Accept-Encoding: gzip\r\nAccept-Encoding: identity\r\n"] {
            assert_eq!(validate_headers(raw),Err(Error::InvalidHeader));
        }
        assert_eq!(validate_headers(&[b'x';HEADER_LIMIT+1]),Err(Error::Capacity));
        assert_eq!(get_with_headers("example.test","/",b"Cookie: a=b\r\n",&mut [0;32]),Err(Error::Capacity));
    }
    // ------------------------=
    // FUNC: long_search_target_survives_native_serialization
    // DESC: Preserves long search and challenge redirect targets without truncating parameters or accepting controls.
    // ------------------=
    #[test]
    fn long_search_target_survives_native_serialization() {
        let target=std::format!("/search?q={}","a".repeat(6000));
        let url=std::format!("https://example.test{target}");
        assert!(crate::geturl::parse(core::iter::once(url.as_str())).is_ok());
        let mut bytes=[0;17408];
        let size=get_with_headers("example.test",&target,b"Cookie: accepted=1\r\n",&mut bytes).unwrap();
        let mut fields=[httparse::EMPTY_HEADER;16];
        let mut request=httparse::Request::new(&mut fields);
        assert!(request.parse(&bytes[..size]).unwrap().is_complete());
        assert_eq!(request.path,Some(target.as_str()));
        let too_long=std::format!("https://example.test/{}","a".repeat(8192));
        assert!(crate::geturl::parse(core::iter::once(too_long.as_str())).is_err());
    }
    // ------------------------=
    // FUNC: standard_methods_own_their_framing
    // DESC: Verifies method and exact generated length while rejecting unsafe methods, body overflow, and framing injection.
    // ------------------=
    #[test]
    fn standard_methods_own_their_framing() {
        for method in ["POST","PUT","PATCH","DELETE","OPTIONS"] {
            let mut bytes=[0;512];
            let n=head("example.test","/submit",method,b"Content-Type: application/octet-stream\r\n",17,&mut bytes).unwrap();
            let mut fields=[httparse::EMPTY_HEADER;16];let mut parsed=httparse::Request::new(&mut fields);
            assert!(parsed.parse(&bytes[..n]).unwrap().is_complete());assert_eq!(parsed.method,Some(method));
            assert_eq!(parsed.headers.iter().filter(|h|h.name.eq_ignore_ascii_case("content-length")).map(|h|h.value).collect::<std::vec::Vec<_>>(),std::vec![b"17".as_slice()]);
        }
        for method in ["TRACE","CONNECT","POST\r\nX: x",""] {assert_eq!(validate_method(method),Err(Error::InvalidMethod));}
        assert_eq!(head("example.test","/","POST",&[],BODY_LIMIT+1,&mut[0;512]),Err(Error::Capacity));
        assert_eq!(head("example.test","/","HEAD",&[],1,&mut[0;512]),Err(Error::InvalidMethod));
        assert_eq!(head("example.test","/","POST",b"Content-Length: 1\r\n",2,&mut[0;512]),Err(Error::InvalidHeader));
    }
}
