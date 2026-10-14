//! Explicit current-document compiler intent shared by voice and attached chat.
// ------------------------=
// FUNC: parse
// DESC: Returns whether an explicit compile request first requires saving; never accepts embedded instructions.
// ------------------=
pub fn parse(input: &[u8]) -> Option<bool> {
    let text = core::str::from_utf8(input).ok()?.trim();
    let text = text.strip_suffix('.').unwrap_or(text).trim_end();
    let text = if text.get(..7).is_some_and(|s| s.eq_ignore_ascii_case("please ")) {
        text[7..].trim_start()
    } else { text };
    for (prefix, save) in [("save and compile", true), ("save then compile", true), ("compile", false)] {
        let Some(head) = text.get(..prefix.len()) else { continue; };
        if !head.eq_ignore_ascii_case(prefix) { continue; }
        let tail = &text[prefix.len()..];
        if !tail.is_empty() && !tail.starts_with(' ') { continue; }
        if ["", "this", "code", "my code", "this code", "the code", "program", "this program",
            "my program", "the program", "this file", "my file", "the file", "this document",
            "the document", "my document"].iter().any(|s| tail.trim().eq_ignore_ascii_case(s)) {
            return Some(save);
        }
    }
    None
}
