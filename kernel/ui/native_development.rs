//! Stable parsing for native developer commands; execution remains capability checked by the desktop runtime.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command<'a> {
    Compile(&'a [u8]),
    Run(&'a [u8]),
    Status,
    Cancel,
}

// ------------------------=
// FUNC: parse
// DESC: Parses exact cc and run verbs with one non-empty ObjectStore path and no shell expansion.
// ------------------=
pub fn parse(input: &[u8]) -> Option<Command<'_>> {
    let first = input
        .iter()
        .position(|byte| !byte.is_ascii_whitespace())
        .unwrap_or(input.len());
    let input = &input[first..];
    let separator = input
        .iter()
        .position(|byte| byte.is_ascii_whitespace())
        .unwrap_or(input.len());
    let verb = &input[..separator];
    let tail = &input[separator..];
    let path_start = tail
        .iter()
        .position(|byte| !byte.is_ascii_whitespace())
        .unwrap_or(tail.len());
    let path = &tail[path_start..];
    if matches!(verb, b"cc-status" | b"cc-cancel") {
        if !path.is_empty() {
            return None;
        }
        return Some(if verb == b"cc-status" {
            Command::Status
        } else {
            Command::Cancel
        });
    }
    if path.is_empty()
        || path
            .iter()
            .any(|byte| byte.is_ascii_whitespace() || *byte == 0)
    {
        return None;
    }
    match verb {
        b"cc" => Some(Command::Compile(path)),
        b"run" => Some(Command::Run(path)),
        _ => None,
    }
}
