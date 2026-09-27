//! Bounded URL/search interpretation. Servo still performs authoritative URL
//! parsing and the native network service still enforces every request's authority.
use crate::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Destination { Url, Search }

// ------------------------=
// FUNC: resolve
// DESC: Preserves explicit HTTP URLs, upgrades bare domains to HTTPS, and percent-encodes search input without allocation.
// ------------------=
pub fn resolve(input: &str, search_prefix: &str, output: &mut [u8]) -> Result<(Destination, usize), Error> {
    let input = input.trim();
    if input.is_empty() || input.chars().any(char::is_control) { return Err(Error::Invalid); }
    let explicit = input.get(..7).map(|s| s.eq_ignore_ascii_case("http://")).unwrap_or(false)
        || input.get(..8).map(|s| s.eq_ignore_ascii_case("https://")).unwrap_or(false);
    if explicit {
        if input.len() > output.len() { return Err(Error::Full); }
        output[..input.len()].copy_from_slice(input.as_bytes());
        return Ok((Destination::Url, input.len()));
    }
    // Never transform a privileged scheme into a navigable web URL.
    if input.contains("://") || input.starts_with("javascript:") || input.starts_with("data:")
        || input.starts_with("file:") || input.starts_with("about:") { return Err(Error::Invalid); }
    let authority = input.split(['/', '?', '#']).next().unwrap_or("");
    let host = authority.split(':').next().unwrap_or("");
    let bare_domain = !input.chars().any(char::is_whitespace) && !authority.contains('@')
        && (host == "localhost" || (host.contains('.') && !host.starts_with('.') && !host.ends_with('.')
            && host.split('.').all(|part| !part.is_empty() && !part.starts_with('-') && !part.ends_with('-')
                && part.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-'))));
    let (kind, prefix) = if bare_domain { (Destination::Url, "https://") } else {
        if !search_prefix.starts_with("https://") || search_prefix.chars().any(char::is_control) {
            return Err(Error::Invalid);
        }
        (Destination::Search, search_prefix)
    };
    let length = prefix.len() + input.bytes().map(|b| if kind == Destination::Url || unreserved(b) { 1 } else { 3 }).sum::<usize>();
    if length > output.len() { return Err(Error::Full); }
    output[..prefix.len()].copy_from_slice(prefix.as_bytes());
    let mut at = prefix.len();
    for byte in input.bytes() {
        if kind == Destination::Url || unreserved(byte) { output[at] = byte; at += 1; }
        else {
            output[at] = b'%';
            output[at + 1] = b"0123456789ABCDEF"[(byte >> 4) as usize];
            output[at + 2] = b"0123456789ABCDEF"[(byte & 15) as usize];
            at += 3;
        }
    }
    Ok((kind, at))
}

// ------------------------=
// FUNC: unreserved
// DESC: Identifies URI query bytes that do not need percent encoding.
// ------------------=
fn unreserved(byte: u8) -> bool { byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') }

#[cfg(test)]
mod tests {
    use super::*;
    // ------------------------=
    // FUNC: routes_and_encodes_without_injection
    // DESC: Validates URL/search routing and exact request bytes including Unicode and query separators.
    // ------------------=
    #[test]
    fn routes_and_encodes_without_injection() {
        let mut bytes = [0; 2048];
        for (input, kind, expected) in [
            (" example.com/docs ", Destination::Url, "https://example.com/docs"),
            ("http://localhost:8000/a", Destination::Url, "http://localhost:8000/a"),
            ("localhost:8000", Destination::Url, "https://localhost:8000"),
            ("cats & café", Destination::Search, "https://example.com/?q=cats%20%26%20caf%C3%A9"),
            ("two words.com", Destination::Search, "https://example.com/?q=two%20words.com"),
        ] {
            let (actual, length) = resolve(input, "https://example.com/?q=", &mut bytes).unwrap();
            assert_eq!(actual, kind);
            assert_eq!(&bytes[..length], expected.as_bytes());
        }
        assert_eq!(resolve("file:///etc", "https://example.com/?q=", &mut bytes), Err(Error::Invalid));
        assert_eq!(resolve("hello\nworld", "https://example.com/?q=", &mut bytes), Err(Error::Invalid));
        let mut short = [17; 4];
        assert_eq!(resolve("cats", "https://example.com/?q=", &mut short), Err(Error::Full));
        assert_eq!(short, [17; 4]);
    }
}
