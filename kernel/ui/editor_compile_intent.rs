//! Explicit current-document compiler intent shared by voice and attached chat.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApplicationStyle { Terminal, Ui }

// ------------------------=
// FUNC: create_application
// DESC: Admits a direct application creation request, never quoted, negated, or explanatory text.
// ------------------=
pub fn create_application(input: &[u8]) -> bool {
    let Ok(text) = core::str::from_utf8(input) else { return false; };
    let text = text.trim().trim_end_matches('.');
    if text.len() > 256 { return false; }
    let mut words = text.split_ascii_whitespace();
    let mut verb = words.next().unwrap_or("");
    if verb.eq_ignore_ascii_case("please") { verb = words.next().unwrap_or(""); }
    if !["create", "build", "generate", "write"].iter().any(|v| verb.eq_ignore_ascii_case(v)) { return false; }
    let mut count = 0;
    let mut application_noun = false;
    let mut ui = false;
    let mut product = false;
    let mut last = "";
    for word in words {
        let word = word.trim_end_matches([',', '.']);
        if word.is_empty() || !word.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
            || ["not", "never", "without", "only", "instead", "explain"].iter().any(|v| word.eq_ignore_ascii_case(v)) {
            return false;
        }
        count += 1; last = word;
        application_noun |= ["application", "app"].iter().any(|v| word.eq_ignore_ascii_case(v));
        ui |= ["ui", "gui", "graphical", "windowed", "interface"].iter().any(|v| word.eq_ignore_ascii_case(v));
        product |= ["calculator", "program", "tool"].iter().any(|v| word.eq_ignore_ascii_case(v));
    }
    count >= 2 && (application_noun && ["application", "app"].iter().any(|v| last.eq_ignore_ascii_case(v)) || ui && product)
}

// ------------------------=
// FUNC: application_style
// DESC: Classifies an admitted creation request as terminal or declarative Infinity UI Kit output.
// ------------------=
pub fn application_style(input: &[u8]) -> Option<ApplicationStyle> {
    if !create_application(input) { return None; }
    let text = core::str::from_utf8(input).ok()?;
    Some(if text.split_ascii_whitespace().any(|word| {
        let word = word.trim_matches(|c: char| c == ',' || c == '.');
        ["ui", "gui", "graphical", "windowed", "interface"].iter().any(|value| word.eq_ignore_ascii_case(value))
    }) { ApplicationStyle::Ui } else { ApplicationStyle::Terminal })
}

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
