//! Explicit current-document compiler intent shared by voice and attached chat.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApplicationStyle {
    Terminal,
    Ui,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApplicationKind {
    Calculator,
    Game,
    General,
}

// ------------------------=
// FUNC: requests_opponent
// DESC: Recognizes explicit play-against-me authority without granting autonomous interaction for mere game creation.
// ------------------=
pub fn requests_opponent(input:&[u8])->bool {
    let Ok(text)=core::str::from_utf8(input) else{return false;};
    let text=text.trim().trim_end_matches('.');
    if !text.split_ascii_whitespace().next().is_some_and(|s|s.eq_ignore_ascii_case("play")) && !create_application(input) {return false;}
    if text.split_ascii_whitespace().any(|s|["not","never","don't","without"].iter().any(|v|s.eq_ignore_ascii_case(v))) {return false;}
    let mut words=text.split_ascii_whitespace().rev();
    if !words.next().is_some_and(|s|s.eq_ignore_ascii_case("me")){return false;}
    match words.next() {
        Some(word) if word.eq_ignore_ascii_case("play")=>true,
        Some(word) if word.eq_ignore_ascii_case("against")=>words.next().is_some_and(|s|s.eq_ignore_ascii_case("play")),
        _=>false,
    }
}

// ------------------------=
// FUNC: application_kind
// DESC: Classifies one direct, bounded software-creation request without granting authority to quoted, negated, or explanatory text.
// ------------------=
pub fn application_kind(input: &[u8]) -> Option<ApplicationKind> {
    let Ok(text) = core::str::from_utf8(input) else {
        return None;
    };
    let text = text.trim().trim_end_matches('.');
    if text.len() > 256 {
        return None;
    }
    let mut words = text.split_ascii_whitespace();
    let mut verb = words.next().unwrap_or("");
    if verb.eq_ignore_ascii_case("please") {
        verb = words.next().unwrap_or("");
    }
    if !["create", "build", "generate", "write"]
        .iter()
        .any(|value| verb.eq_ignore_ascii_case(value))
    {
        return None;
    }
    let mut count = 0;
    let mut application_noun = false;
    let mut ui = false;
    let mut product = false;
    let mut calculator = false;
    let mut game = false;
    let mut tail = [""; 3];
    for word in words {
        let word = word.trim_end_matches([',', '.']);
        if word.is_empty()
            || !word.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
            || ["not", "never", "without", "only", "instead", "explain"]
                .iter()
                .any(|value| word.eq_ignore_ascii_case(value))
        {
            return None;
        }
        count += 1;
        tail = [tail[1], tail[2], word];
        application_noun |= ["application", "app", "program", "tool", "game"]
            .iter()
            .any(|value| word.eq_ignore_ascii_case(value));
        ui |= ["ui", "gui", "graphical", "windowed", "interface", "game", "chess"]
            .iter()
            .any(|value| word.eq_ignore_ascii_case(value));
        product |= ["calculator", "program", "tool", "game", "chess"]
            .iter()
            .any(|value| word.eq_ignore_ascii_case(value));
        calculator |= word.eq_ignore_ascii_case("calculator");
        game |= ["game", "chess", "checkers", "solitaire", "puzzle"]
            .iter()
            .any(|value| word.eq_ignore_ascii_case(value));
    }
    let play_tail = tail[0].eq_ignore_ascii_case("and")
        && tail[1].eq_ignore_ascii_case("play")
        && tail[2].eq_ignore_ascii_case("me");
    let named_ending = ["application", "app", "game"]
        .iter()
        .any(|value| tail[2].eq_ignore_ascii_case(value));
    if count < 2 || !(application_noun && (named_ending || play_tail) || ui && product) {
        return None;
    }
    Some(if game {
        ApplicationKind::Game
    } else if calculator {
        ApplicationKind::Calculator
    } else {
        ApplicationKind::General
    })
}

// ------------------------=
// FUNC: create_application
// DESC: Admits a direct application creation request, never quoted, negated, or explanatory text.
// ------------------=
pub fn create_application(input: &[u8]) -> bool {
    application_kind(input).is_some()
}

// ------------------------=
// FUNC: application_style
// DESC: Classifies an admitted creation request as terminal or declarative Infinity UI Kit output.
// ------------------=
pub fn application_style(input: &[u8]) -> Option<ApplicationStyle> {
    if !create_application(input) {
        return None;
    }
    let text = core::str::from_utf8(input).ok()?;
    Some(
        if application_kind(input) == Some(ApplicationKind::Game)
            || text.split_ascii_whitespace().any(|word| {
            let word = word.trim_matches(|c: char| c == ',' || c == '.');
            ["ui", "gui", "graphical", "windowed", "interface"]
                .iter()
                .any(|value| word.eq_ignore_ascii_case(value))
        }) {
            ApplicationStyle::Ui
        } else {
            ApplicationStyle::Terminal
        },
    )
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
