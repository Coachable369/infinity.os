//! Bounded URL/search interpretation. Servo still performs authoritative URL
//! parsing and the native network service still enforces every request's authority.
use crate::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Destination { Url, Search }

// ------------------------=
// FUNC: web_association
// DESC: Resolves explicit public web schemes for OS open requests without turning local paths into searches.
// ------------------=
pub fn web_association(input:&[u8],output:&mut[u8])->Result<Option<usize>,Error> {
    let scheme=if input.get(..8).is_some_and(|s|s.eq_ignore_ascii_case(b"https://")) {8}
        else if input.get(..7).is_some_and(|s|s.eq_ignore_ascii_case(b"http://")) {7}
        else {return Ok(None);};
    let text=core::str::from_utf8(input).map_err(|_|Error::Invalid)?;
    if text.chars().any(char::is_whitespace) || input.len()==scheme
        || matches!(input[scheme],b'/'|b'?'|b'#') {return Err(Error::Invalid);}
    let (_,length)=resolve(text,"https://example.com/",output)?;
    output[..scheme].make_ascii_lowercase();
    Ok(Some(length))
}

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
    if !bare_domain {return search(input,search_prefix,output).map(|n|(Destination::Search,n));}
    let prefix="https://";
    let length = prefix.len() + input.len();
    if length > output.len() { return Err(Error::Full); }
    output[..prefix.len()].copy_from_slice(prefix.as_bytes());
    output[prefix.len()..length].copy_from_slice(input.as_bytes());
    Ok((Destination::Url, length))
}

// ------------------------=
// FUNC: search
// DESC: Encodes literal selected text as a query even when it resembles a URL; rejects overflow before writing.
// ------------------=
pub fn search(input:&str,prefix:&str,output:&mut[u8])->Result<usize,Error> {
    if input.trim().is_empty() || !prefix.starts_with("https://") || prefix.chars().any(char::is_control) {return Err(Error::Invalid);}
    let length=prefix.len()+input.bytes().map(|b|if unreserved(b){1}else{3}).sum::<usize>();
    if length>output.len() {return Err(Error::Full);}
    output[..prefix.len()].copy_from_slice(prefix.as_bytes());let mut at=prefix.len();
    for b in input.bytes() {
        if unreserved(b) {output[at]=b;at+=1;}else{
            output[at]=b'%';output[at+1]=b"0123456789ABCDEF"[(b>>4) as usize];
            output[at+2]=b"0123456789ABCDEF"[(b&15) as usize];at+=3;
        }
    }Ok(at)
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
    // FUNC: selected_text_is_always_a_literal_query
    // DESC: Tests URL-shaped selections, multiline text and atomic rejection when the query exceeds capacity.
    // ------------------=
    #[test]
    fn selected_text_is_always_a_literal_query() {
        let mut bytes=[0;2048];let prefix="https://www.google.com/search?q=";
        let n=search("https://example.com/?x=1&y=2\n",prefix,&mut bytes).unwrap();
        assert_eq!(&bytes[..n],b"https://www.google.com/search?q=https%3A%2F%2Fexample.com%2F%3Fx%3D1%26y%3D2%0A");
        let mut small=[7;8];assert_eq!(search("text",prefix,&mut small),Err(Error::Full));assert_eq!(small,[7;8]);
    }
    // ------------------------=
    // FUNC: explicit_web_association_preserves_local_objects
    // DESC: Checks default web routing, bounded writes and refusal to turn file or privileged schemes into web requests.
    // ------------------=
    #[test]
    fn explicit_web_association_preserves_local_objects() {
        let mut output=[0u8;128];
        assert_eq!(web_association(b"HTTPS://example.com/",&mut output),Ok(Some(20)));
        assert_eq!(&output[..20],b"https://example.com/");
        for input in [b"/home/default/note".as_slice(),b"file:///secret",b"javascript:alert(1)",b"example.com"] {
            assert_eq!(web_association(input,&mut output),Ok(None));
        }
        for input in [b"https://".as_slice(),b"https:///",b"https://bad host/",b"http://\n"] {
            assert_eq!(web_association(input,&mut output),Err(Error::Invalid));
        }
        assert_eq!(web_association(b"https://example.com/",&mut [0u8;4]),Err(Error::Full));
    }
    // ------------------------=
    // FUNC: routes_and_encodes_without_injection
    // DESC: Validates URL/search routing and exact request bytes including Unicode and query separators.
    // ------------------=
    #[test]
    fn routes_and_encodes_without_injection() {
        let mut bytes = [0; 2048];
        for (input, kind, expected) in [
            ("https://example.com/path?q=1#top", Destination::Url, "https://example.com/path?q=1#top"),
            ("www.wikipedia.org", Destination::Url, "https://www.wikipedia.org"),
            ("192.168.1.1/settings", Destination::Url, "https://192.168.1.1/settings"),
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
