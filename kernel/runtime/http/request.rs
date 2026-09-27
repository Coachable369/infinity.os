#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidAuthority,
    InvalidTarget,
    Capacity,
}

// ------------------------=
// FUNC: get
// DESC: Builds a bounded HTTP GET without permitting header injection or implying TLS protection.
// ------------------=
pub fn get(host: &str, path: &str, output: &mut [u8]) -> Result<usize, Error> {
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
        b"\r\nConnection: close\r\nAccept-Encoding: identity\r\nUser-Agent: InfinityOS/0.1\r\n\r\n",
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
