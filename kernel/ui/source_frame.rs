//! One bounded source frame shared by streaming preview and final admission.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Frame<'a> {
    pub source: &'a [u8],
    pub raw_source: &'a [u8],
    pub completed_end: Option<usize>,
    pub malformed: bool,
    pub continuable: bool,
    pub framed: bool,
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
// FUNC: starts_python
// DESC: Recognizes Python statement starts only for an explicitly Python source transaction; it never grants execution authority.
// ------------------=
fn starts_python(mut source: &[u8]) -> bool {
    loop {
        source=source.trim_ascii_start();
        if !source.starts_with(b"#") {break;}
        let end=match source.iter().position(|byte|*byte==b'\n') {Some(end)=>end,None=>return false};
        source=&source[end+1..];
    }
    if [b"def ".as_slice(),b"async def ",b"class ",b"import ",b"from ",b"print(",b"print (",
        b"if ",b"for ",b"while ",b"with ",b"try:",b"match ",b"assert ",b"raise "]
        .iter().any(|prefix|source.starts_with(prefix)) {return true;}
    if !source.first().is_some_and(|byte|byte.is_ascii_alphabetic() || *byte==b'_') {return false;}
    let end=source.iter().position(|byte|!byte.is_ascii_alphanumeric() && *byte!=b'_').unwrap_or(source.len());
    let tail=source[end..].trim_ascii_start();
    tail.starts_with(b"=") && !tail.starts_with(b"==")
}

// ------------------------=
// FUNC: control_line
// DESC: Identifies only standalone protocol headers, never a token embedded in C or quoted text.
// ------------------=
fn control_line(line: &[u8]) -> bool {
    [b"REPLACE_COMPILE".as_slice(), b"REPLACE", b"INSERT", b"ACTION", b"CHAT", b"CONTINUE_SOURCE"]
        .contains(&line)
}

// ------------------------=
// FUNC: partial_control
// DESC: Withholds an unfinished final control line before any of its bytes reach the editor.
// ------------------=
fn partial_control(line: &[u8]) -> bool {
    !line.is_empty() && [b"END_ACTION".as_slice(), b"REPLACE_COMPILE", b"REPLACE", b"INSERT", b"ACTION", b"CHAT", b"CONTINUE_SOURCE", b"```"]
        .iter().any(|marker| marker.starts_with(line))
}

#[derive(Default)]
struct LexicalState { block: bool, quote: u8, escaped: bool, triple: bool }
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
                else if byte == self.quote && (!self.triple || line.get(index..index+3)==Some(&[self.quote;3])) {
                    if self.triple {index+=2;} self.quote=0;self.triple=false;
                }
            } else if byte == b'/' && next == Some(b'/') { break; }
            else if byte == b'#' {break;}
            else if byte == b'/' && next == Some(b'*') { self.block = true; index += 1; }
            else if matches!(byte, b'"' | b'\'') {
                self.quote=byte;self.triple=line.get(index..index+3)==Some(&[byte;3]);
                if self.triple {index+=2;}
            }
            index += 1;
        }
    }
}

// ------------------------=
// FUNC: inspect
// DESC: Selects the first source frame identically for preview and completion, accepting CRLF and indented markers without leaking repeated model actions.
// ------------------=
pub fn inspect(output: &[u8]) -> Option<Frame<'_>> {
    inspect_language(output,false,false)
}

// ------------------------=
// FUNC: inspect_python
// DESC: Uses identical framing and literal boundaries while allowing a raw Python body selected by trusted request language.
// ------------------=
pub fn inspect_python(output: &[u8]) -> Option<Frame<'_>> {
    inspect_language(output,true,false)
}

// ------------------------=
// FUNC: inspect_generated
// DESC: Allows one bounded prose introduction only for an already authorized source transaction and its matching explicit language fence.
// ------------------=
pub fn inspect_generated(output: &[u8], python: bool) -> Option<Frame<'_>> {
    inspect_language(output,python,true)
}

// ------------------------=
// FUNC: preamble_fence
// DESC: Finds one explicit matching source fence within eight short introduction lines, rejecting protocol or competing source frames.
// ------------------=
fn preamble_fence(output: &[u8], python: bool) -> Option<usize> {
    let mut cursor = 0;
    let mut lines = 0;
    while cursor <= 512 && lines <= 8 {
        let end = cursor + output.get(cursor..)?.iter().position(|byte| *byte == b'\n')?;
        let line = output[cursor..end].trim_ascii();
        if if python { matches!(line,b"```python"|b"```py") } else { matches!(line,b"```c"|b"```C") } {
            return Some(cursor);
        }
        if lines == 8 || end + 1 > 512 || control_line(line) || line == b"END_ACTION"
            || line.windows(3).any(|bytes| bytes == b"```")
            || starts_source(line) || python && starts_python(line)
            || line.iter().any(|byte| !matches!(*byte,b'\t'|b'\r'|32..=126)) {
            return None;
        }
        cursor = end + 1;
        lines += 1;
    }
    None
}

// ------------------------=
// FUNC: preamble_tail
// DESC: Accepts only whitespace or one optional closing protocol marker after a prefaced fence, withholding a split marker and rejecting additional frames.
// ------------------=
fn preamble_tail(tail: &[u8]) -> (bool,bool) {
    let tail = tail.trim_ascii_start();
    let complete = tail.trim_ascii_end();
    if complete.is_empty() || complete == b"END_ACTION" { return (true,false); }
    if tail == complete && b"END_ACTION".starts_with(tail) { return (false,false); }
    (false,true)
}

// ------------------------=
// FUNC: inspect_language
// DESC: Shares first-frame selection and completion bounds for framed source and language-scoped raw source.
// ------------------=
fn inspect_language(output: &[u8], python: bool, allow_preamble: bool) -> Option<Frame<'_>> {
    let trimmed = output.trim_ascii_start();
    let leading = output.len() - trimmed.len();
    let first_end = trimmed.iter().position(|b| *b == b'\n');
    let first = trimmed[..first_end.unwrap_or(trimmed.len())].trim_ascii();
    let (mut start, prefaced) = if matches!(first, b"REPLACE" | b"REPLACE_COMPILE") {
        (leading + first_end? + 1,false)
    } else if starts_source(trimmed) || python && starts_python(trimmed) || first.starts_with(b"```") {
        (leading,false)
    } else if allow_preamble {
        (preamble_fence(output,python)?,true)
    } else { return None; };
    let framed = prefaced || matches!(first, b"REPLACE" | b"REPLACE_COMPILE") || first.starts_with(b"```");
    start += output[start..].len() - output[start..].trim_ascii_start().len();
    let mut fenced = false;
    if output[start..].starts_with(b"`") {
        let end = output[start..].iter().position(|b| *b == b'\n')?;
        if !matches!(output[start..start + end].trim_ascii(), b"```c" | b"```C" | b"```python" | b"```py" | b"```") { return None; }
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
                return Some(Frame { source: output[start..cursor].trim_ascii_end(), raw_source: &output[start..cursor], completed_end: None, malformed: true, continuable: false, framed });
            }
            if line == b"END_ACTION" || (fenced && line == b"```") {
                let (complete,malformed) = if prefaced { preamble_tail(&output[end..]) } else { (true,false) };
                return Some(Frame { source: output[start..cursor].trim_ascii_end(), raw_source: &output[start..cursor],
                    completed_end: complete.then_some(end - (output[cursor..end].len() - output[cursor..end].trim_ascii_end().len())),
                    malformed, continuable: false, framed });
            }
            if control_line(line) {
                return Some(Frame { source: output[start..cursor].trim_ascii_end(), raw_source: &output[start..cursor], completed_end: None, malformed: true, continuable: false, framed });
            }
            if relative_end.is_none() && partial_control(line) {
                return Some(Frame { source: output[start..cursor].trim_ascii_end(), raw_source: &output[start..cursor], completed_end: None, malformed: true, continuable: true, framed });
            }
        }
        lexical.advance(&output[cursor..end]);
        cursor = end;
    }
    Some(Frame { source: output[start..].trim_ascii_end(), raw_source: &output[start..], completed_end: None, malformed: fenced, continuable: true, framed })
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
