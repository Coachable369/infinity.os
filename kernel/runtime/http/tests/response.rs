use infinity_http::response::{parse, Body, Error, HEADER_LIMIT};

#[test]
// ------------------------=
// FUNC: fragmented_headers_and_body_boundary
// DESC: Verifies every split point remains incomplete until all headers arrive and leaves body bytes untouched.
// ------------------=
fn fragmented_headers_and_body_boundary() {
    let header = b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\n";
    for split in 0..header.len() {
        assert_eq!(parse(&header[..split], false), Ok(None));
    }
    let mut bytes = header.to_vec();
    bytes.extend_from_slice(b"hello");
    let head = parse(&bytes, false).unwrap().unwrap();
    assert_eq!(head.status, 200);
    assert_eq!(head.body, Body::Length(5));
    assert_eq!(&bytes[head.bytes..], b"hello");
    assert_eq!(parse(&bytes, true).unwrap().unwrap().body, Body::Empty);
}

#[test]
// ------------------------=
// FUNC: framing_is_unambiguous_and_bounded
// DESC: Rejects conflicting lengths, encoding ambiguity, overflow and unbounded headers.
// ------------------=
fn framing_is_unambiguous_and_bounded() {
    assert_eq!(
        parse(
            b"HTTP/1.1 200 OK\r\nContent-Length: 1\r\nContent-Length: 2\r\n\r\n",
            false
        ),
        Err(Error::AmbiguousFraming)
    );
    assert_eq!(
        parse(
            b"HTTP/1.1 200 OK\r\nContent-Length: 1\r\nTransfer-Encoding: chunked\r\n\r\n",
            false
        ),
        Err(Error::AmbiguousFraming)
    );
    assert_eq!(
        parse(
            b"HTTP/1.1 200 OK\r\nContent-Length: 18446744073709551616\r\n\r\n",
            false
        ),
        Err(Error::TooLarge)
    );
    assert_eq!(
        parse(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: gzip\r\n\r\n", false),
        Err(Error::UnsupportedEncoding)
    );
    assert_eq!(
        parse(
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n",
            false
        )
        .unwrap()
        .unwrap()
        .body,
        Body::Chunked
    );
    let mut oversized = b"HTTP/1.1 200 OK\r\nX-Large: ".to_vec();
    oversized.resize(HEADER_LIMIT, b'a');
    assert_eq!(parse(&oversized, false), Err(Error::TooLarge));
}
