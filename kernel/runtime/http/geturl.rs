//! Initial native command contract. Unsupported curl features fail explicitly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Options<'a> {
    pub host: &'a str,
    pub target: &'a str,
    pub fail: bool,
    pub silent: bool,
    pub show_error: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command<'a> {
    Help,
    Version,
    Get(Options<'a>),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Unsupported,
    Url,
    Arguments,
}
impl Error {
    // ------------------------=
    // FUNC: exit_code
    // DESC: Reports curl-compatible categories for unsupported protocol, malformed URL and argument errors.
    // ------------------=
    pub fn exit_code(self) -> u8 {
        match self {
            Self::Unsupported => 1,
            Self::Url => 3,
            Self::Arguments => 2,
        }
    }
}
impl Options<'_> {
    // ------------------------=
    // FUNC: response_exit
    // DESC: Maps HTTP failure status to curl's fail exit code without treating ordinary HTTP errors as transport failures.
    // ------------------=
    pub fn response_exit(self, status: u16) -> u8 {
        if self.fail && status >= 400 {
            22
        } else {
            0
        }
    }
}
// ------------------------=
// FUNC: parse
// DESC: Parses the supported argv subset without treating unknown flags, multiple URLs or unsupported ports as successful GETs.
// ------------------=
pub fn parse<'a>(args: impl Iterator<Item = &'a str>) -> Result<Command<'a>, Error> {
    let mut options = Options {
        host: "",
        target: "",
        fail: false,
        silent: false,
        show_error: false,
    };
    let mut url = None;
    let mut flags = true;
    for arg in args {
        if flags && arg == "--" {
            flags = false;
            continue;
        }
        if flags && arg.starts_with('-') {
            match arg {
                "--help" | "-h" => return Ok(Command::Help),
                "--version" | "-V" => return Ok(Command::Version),
                "--fail" => options.fail = true,
                "--silent" => options.silent = true,
                "--show-error" => options.show_error = true,
                "--http1.1" | "--ipv4" => {}
                _ if !arg.starts_with("--") && arg.len() > 1 => {
                    for flag in arg[1..].bytes() {
                        match flag {
                            b'f' => options.fail = true,
                            b's' => options.silent = true,
                            b'S' => options.show_error = true,
                            b'4' => {}
                            _ => return Err(Error::Arguments),
                        }
                    }
                }
                _ => return Err(Error::Arguments),
            }
        } else if url.replace(arg).is_some() {
            return Err(Error::Arguments);
        }
    }
    let url = url.ok_or(Error::Arguments)?;
    let rest = url.strip_prefix("https://").ok_or(Error::Unsupported)?;
    let rest = rest
        .split_once('#')
        .map(|(before, _)| before)
        .unwrap_or(rest);
    let at = rest.find(['/', '?']).unwrap_or(rest.len());
    let authority = &rest[..at];
    let host = authority.strip_suffix(":443").unwrap_or(authority);
    if host.is_empty()
        || host.len() > 253
        || host.contains(':')
        || host.split('.').any(|label| {
            label.is_empty()
                || label.len() > 63
                || label.starts_with('-')
                || label.ends_with('-')
                || !label
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        })
    {
        return Err(Error::Url);
    }
    // A query without a slash requires an owned normalized target; reject until that path is supported.
    let target = if at == rest.len() { "/" } else { &rest[at..] };
    if target.len() > 8192
        || !target.starts_with('/')
        || !target.bytes().all(|b| (0x21..=0x7e).contains(&b))
        || target.contains(['{', '}', '[', ']'])
    {
        return Err(Error::Url);
    }
    options.host = host;
    options.target = target;
    Ok(Command::Get(options))
}

#[cfg(test)]
mod tests {
    use super::*;
    // ------------------------=
    // FUNC: supported_argv_and_fail_behavior
    // DESC: Checks typed URL components, combined flags and HTTP exit semantics without output-text assertions.
    // ------------------=
    #[test]
    fn supported_argv_and_fail_behavior() {
        let Command::Get(o) =
            parse(["-fsS", "https://example.org:443/path?q=yes#local"].into_iter()).unwrap()
        else {
            panic!()
        };
        assert_eq!((o.host, o.target), ("example.org", "/path?q=yes"));
        assert!(o.fail && o.silent && o.show_error);
        assert_eq!(o.response_exit(404), 22);
        assert_eq!(o.response_exit(200), 0);
        let Command::Get(o) = parse(["https://example.org"].into_iter()).unwrap() else {
            panic!()
        };
        assert_eq!(o.target, "/");
        assert_eq!(o.response_exit(500), 0);
    }
    // ------------------------=
    // FUNC: unsupported_and_injected_inputs_fail
    // DESC: Rejects unsupported transport and flags, ambiguous requests, credentials and URL injection.
    // ------------------=
    #[test]
    fn unsupported_and_injected_inputs_fail() {
        for url in [
            "https://a:444/",
            "https://user@a/",
            "https://a/\r\nHeader:bad",
            "https://-a/",
            "https://a/{one,two}",
        ] {
            assert_eq!(parse([url].into_iter()), Err(Error::Url));
        }
        assert_eq!(parse(["http://a/"].into_iter()), Err(Error::Unsupported));
        assert_eq!(
            parse(["-L", "https://a/"].into_iter()),
            Err(Error::Arguments)
        );
        assert_eq!(
            parse(["https://a/", "https://b/"].into_iter()),
            Err(Error::Arguments)
        );
    }
}
