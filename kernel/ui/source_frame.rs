//! One bounded source frame shared by streaming preview and final admission.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Frame<'a> {
    pub source: &'a [u8],
    pub completed_end: Option<usize>,
    pub malformed: bool,
}

// ------------------------=
// FUNC: starts_source
// DESC: Recognizes raw C after complete comments without admitting prose as an executable proposal.
// ------------------=
pub fn starts_source(mut source: &[u8]) -> bool {
    loop {
        source = source.trim_ascii_start();
        if source.starts_with(b"//") {
            let Some(end) = source.iter().position(|b| *b == b'\n') else { return false; };
            source = &source[end + 1..];
        } else if source.starts_with(b"/*") {
            let Some(end) = source[2..].windows(2).position(|b| b == b"*/") else { return false; };
            source = &source[end + 4..];
        } else { break; }
    }
    [b"#include".as_slice(), b"int main(", b"int main ("].iter().any(|p| source.starts_with(p))
}

// ------------------------=
// FUNC: control_line
// DESC: Identifies only standalone protocol headers, never a token embedded in C or quoted text.
// ------------------=
fn control_line(line: &[u8]) -> bool {
    [b"REPLACE_COMPILE".as_slice(), b"REPLACE", b"INSERT", b"ACTION", b"CHAT"]
        .contains(&line)
}

// ------------------------=
// FUNC: partial_control
// DESC: Withholds an unfinished final control line before any of its bytes reach the editor.
// ------------------=
fn partial_control(line: &[u8]) -> bool {
    !line.is_empty() && [b"END_ACTION".as_slice(), b"REPLACE_COMPILE", b"REPLACE", b"INSERT", b"ACTION", b"CHAT", b"```"]
        .iter().any(|marker| marker.starts_with(line))
}

#[derive(Default)]
struct LexicalState { block: bool, quote: u8, escaped: bool }
impl LexicalState {
    // ------------------------=
    // FUNC: advance
    // DESC: Tracks comments and literals so a protocol-looking line inside C data is preserved verbatim.
    // ------------------=
    fn advance(&mut self, line: &[u8]) {
        let mut index = 0;
        while index < line.len() {
            let byte = line[index];
            let next = line.get(index + 1).copied();
            if self.block {
                if byte == b'*' && next == Some(b'/') { self.block = false; index += 1; }
            } else if self.quote != 0 {
                if self.escaped { self.escaped = false; }
                else if byte == b'\\' { self.escaped = true; }
                else if byte == self.quote { self.quote = 0; }
            } else if byte == b'/' && next == Some(b'/') { break; }
            else if byte == b'/' && next == Some(b'*') { self.block = true; index += 1; }
            else if matches!(byte, b'"' | b'\'') { self.quote = byte; }
            index += 1;
        }
    }
}

// ------------------------=
// FUNC: inspect
// DESC: Selects the first source frame identically for preview and completion, accepting CRLF and indented markers without leaking repeated model actions.
// ------------------=
pub fn inspect(output: &[u8]) -> Option<Frame<'_>> {
    let trimmed = output.trim_ascii_start();
    let leading = output.len() - trimmed.len();
    let first_end = trimmed.iter().position(|b| *b == b'\n');
    let first = trimmed[..first_end.unwrap_or(trimmed.len())].trim_ascii();
    let mut start = if matches!(first, b"REPLACE" | b"REPLACE_COMPILE") {
        leading + first_end? + 1
    } else if starts_source(trimmed) || first.starts_with(b"```") {
        leading
    } else { return None; };
    start += output[start..].len() - output[start..].trim_ascii_start().len();
    let mut fenced = false;
    if output[start..].starts_with(b"`") {
        let end = output[start..].iter().position(|b| *b == b'\n')?;
        if !matches!(output[start..start + end].trim_ascii(), b"```c" | b"```C" | b"```") { return None; }
        start += end + 1;
        fenced = true;
    }
    let mut lexical = LexicalState::default();
    let mut cursor = start;
    while cursor < output.len() {
        let relative_end = output[cursor..].iter().position(|b| *b == b'\n');
        let end = relative_end.map_or(output.len(), |n| cursor + n + 1);
        let line = output[cursor..end].trim_ascii();
        if !lexical.block && lexical.quote == 0 {
            if fenced && line == b"END_ACTION" {
                return Some(Frame { source: output[start..cursor].trim_ascii_end(), completed_end: None, malformed: true });
            }
            if line == b"END_ACTION" || (fenced && line == b"```") {
                return Some(Frame { source: output[start..cursor].trim_ascii_end(),
                    completed_end: Some(end - (output[cursor..end].len() - output[cursor..end].trim_ascii_end().len())), malformed: false });
            }
            if control_line(line) {
                return Some(Frame { source: output[start..cursor].trim_ascii_end(), completed_end: None, malformed: true });
            }
            if relative_end.is_none() && partial_control(line) {
                return Some(Frame { source: output[start..cursor].trim_ascii_end(), completed_end: None, malformed: true });
            }
        }
        lexical.advance(&output[cursor..end]);
        cursor = end;
    }
    Some(Frame { source: output[start..].trim_ascii_end(), completed_end: None, malformed: fenced })
}

// ------------------------=
// FUNC: completed_length
// DESC: Recognizes the first complete action or source frame, never a later repeated model response.
// ------------------=
pub fn completed_length(output: &[u8]) -> Option<usize> {
    if let Some(frame) = inspect(output) { return frame.completed_end; }
    let trimmed = output.trim_ascii_start();
    let leading = output.len() - trimmed.len();
    let first = trimmed.iter().position(|b| *b == b'\n')?;
    if !control_line(trimmed[..first].trim_ascii()) { return None; }
    let mut cursor = leading + first + 1;
    while cursor < output.len() {
        let end = output[cursor..].iter().position(|b| *b == b'\n').map_or(output.len(), |n| cursor + n + 1);
        if output[cursor..end].trim_ascii() == b"END_ACTION" {
            return Some(end - (output[cursor..end].len() - output[cursor..end].trim_ascii_end().len()));
        }
        cursor = end;
    }
    None
}
