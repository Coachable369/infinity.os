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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RepairIntent {
    CurrentDocument,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceLanguage { C, Python }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UpdateIntent<'a> { pub target: &'a [u8], pub launch: bool }

// ------------------------=
// FUNC: language
// DESC: Selects an explicitly requested source language without inferring execution authority.
// ------------------=
pub fn language(input: &[u8]) -> SourceLanguage {
    if input.split(|b| b.is_ascii_whitespace()).any(|word|
        word.strip_suffix(b".").unwrap_or(word).eq_ignore_ascii_case(b"python")
            || word.strip_suffix(b".").unwrap_or(word).eq_ignore_ascii_case(b"python3")) {
        SourceLanguage::Python
    } else { SourceLanguage::C }
}

// ------------------------=
// FUNC: update
// DESC: Extracts a bounded direct existing-source modification and explicit optional relaunch, never a quoted or explanatory command.
// ------------------=
pub fn update(input: &[u8]) -> Option<UpdateIntent<'_>> {
    if input.len()>256 {return None;}
    let mut text=core::str::from_utf8(input).ok()?.trim().trim_end_matches('.');
    if text.get(..7).is_some_and(|prefix|prefix.eq_ignore_ascii_case("please ")) {text=text[7..].trim_start();}
    let (verb,body)=text.split_once(' ')?;
    if !["update","modify","extend","enhance","improve"].iter().any(|v|verb.eq_ignore_ascii_case(v)) {return None;}
    if body.bytes().any(|b|!b.is_ascii() || matches!(b,b'\n'|b'\r'|b';'|b'`'|b'"')) {return None;}
    if body.split_ascii_whitespace().any(|word|["never","don't","not"].iter().any(|v|word.eq_ignore_ascii_case(v))) {return None;}
    let boundary=[b" and ".as_slice(),b" with ",b" to "].iter()
        .filter_map(|separator|body.as_bytes().windows(separator.len()).position(|part|part.eq_ignore_ascii_case(separator)))
        .min()?;
    let mut target=body[..boundary].trim();
    for article in ["the ","my "] {if target.get(..article.len()).is_some_and(|head|head.eq_ignore_ascii_case(article)) {target=target[article.len()..].trim();break;}}
    for suffix in [" application"," program"," app"] {if target.len()>suffix.len() && target[target.len()-suffix.len()..].eq_ignore_ascii_case(suffix) {target=target[..target.len()-suffix.len()].trim_end();break;}}
    if target.is_empty() || target.len()>95 || !target.bytes().all(|b|b.is_ascii_alphanumeric() || matches!(b,b' '|b'.'|b'/'|b'_'|b'-')) {return None;}
    let launch=[" and launch"," and relaunch"," and run"," and then launch"," and then relaunch"]
        .iter().any(|suffix|text.len()>suffix.len() && text[text.len()-suffix.len()..].eq_ignore_ascii_case(suffix));
    Some(UpdateIntent{target:target.as_bytes(),launch})
}

// ------------------------=
// FUNC: software_workflow
// DESC: Routes creation and explicit existing-source changes through the same bounded DevKit plan.
// ------------------=
pub fn software_workflow(input: &[u8]) -> bool { create_application(input) || update(input).is_some() }

// ------------------------=
// FUNC: requests_launch
// DESC: Preserves creation launch behavior while requiring explicit run authority for an existing-source update.
// ------------------=
pub fn requests_launch(input: &[u8]) -> bool { create_application(input) || update(input).is_some_and(|intent|intent.launch) }

// ------------------------=
// FUNC: may_relaunch
// DESC: Grants replacement only for an explicitly launched update of the exact same source owned by the current session.
// ------------------=
pub fn may_relaunch(request: &[u8], requested_path: &[u8], running_path: &[u8], same_owner: bool) -> bool {
    same_owner && !requested_path.is_empty() && requested_path==running_path
        && update(request).is_some_and(|intent|intent.launch)
}

// ------------------------=
// FUNC: source_name
// DESC: Creates a bounded discoverable source basename without replacing an existing object or trusting model-suggested paths.
// ------------------=
pub fn source_name(request: &[u8], language: SourceLanguage, nonce: u64, output: &mut [u8;47]) -> usize {
    let mut length=0;
    for word in request.split(|b|!b.is_ascii_alphanumeric()).filter(|word|!word.is_empty()) {
        if [b"create".as_slice(),b"build",b"generate",b"write",b"please",b"a",b"an",b"the",b"simple",b"basic",
            b"app",b"application",b"program",b"in",b"python",b"python3",b"c"].iter().any(|skip|word.eq_ignore_ascii_case(skip)) {continue;}
        if length>0 && length<24 {output[length]=b'-';length+=1;}
        for byte in word {if length==24 {break;}output[length]=byte.to_ascii_lowercase();length+=1;}
        if length==24 {break;}
    }
    if length==0 {output[..3].copy_from_slice(b"app");length=3;}
    output[length]=b'-';length+=1;
    for shift in (0..16).rev() {output[length]=b"0123456789abcdef"[((nonce>>(shift*4))&15) as usize];length+=1;}
    let extension=if language==SourceLanguage::Python {b".py".as_slice()}else{b".c".as_slice()};
    output[length..length+extension.len()].copy_from_slice(extension);length+extension.len()
}

// ------------------------=
// FUNC: repair
// DESC: Admits one direct current-document repair request, excluding quoted, negated, explanatory and compound instructions.
// ------------------=
pub fn repair(input: &[u8]) -> Option<RepairIntent> {
    if input.len() > 256 { return None; }
    let text=core::str::from_utf8(input).ok()?.trim().trim_end_matches('.');
    let mut words=text.split_ascii_whitespace();
    let mut verb=words.next()?;
    if verb.eq_ignore_ascii_case("please") { verb=words.next()?; }
    if !["fix","repair","correct"].iter().any(|value|verb.eq_ignore_ascii_case(value)) {return None;}
    let mut tail=[0u8;256];
    let mut length=0;
    for word in words {
        if length>0 {tail[length]=b' ';length+=1;}
        if length+word.len()>tail.len(){return None;}
        for byte in word.bytes(){tail[length]=byte.to_ascii_lowercase();length+=1;}
    }
    let tail=&tail[..length];
    for subject in [b"code".as_slice(),b"the code",b"my code",b"this code",b"the current code",
        b"this program",b"the program",b"my program",b"this file",b"the current file",
        b"compiler errors",b"the compiler errors",b"compilation errors",b"the compilation errors"] {
        if tail==subject {return Some(RepairIntent::CurrentDocument);}
        if let Some(location)=tail.strip_prefix(subject) {
            if [b" in code editor".as_slice(),b" in the code editor",b" in text editor",b" in the text editor",b" in editor",
                b" in the editor",b" in this document",b" in the current document"]
                .iter().any(|value|location==*value) {return Some(RepairIntent::CurrentDocument);}
        }
    }
    None
}

// ------------------------=
// FUNC: source_request
// DESC: Identifies explicit user authority for source generation or current-document repair and compile verification, never automatic repair launch.
// ------------------=
pub fn source_request(input: &[u8]) -> bool {
    software_workflow(input) || repair(input).is_some()
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
    let mut tail = [""; 4];
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
        tail = [tail[1], tail[2], tail[3], word];
        application_noun |= ["application", "app", "program", "tool", "game"]
            .iter()
            .any(|value| word.eq_ignore_ascii_case(value));
        ui |= ["ui", "uikit", "ui-kit", "gui", "graphical", "windowed", "interface", "game", "chess"]
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
    let play_tail = (count >= 4 && tail[1].eq_ignore_ascii_case("and")
        && tail[2].eq_ignore_ascii_case("play") && tail[3].eq_ignore_ascii_case("me"))
        || (count >= 5 && tail[0].eq_ignore_ascii_case("and")
            && tail[1].eq_ignore_ascii_case("play") && tail[2].eq_ignore_ascii_case("against")
            && tail[3].eq_ignore_ascii_case("me"));
    let named_ending = ["application", "app", "game"]
        .iter()
        .any(|value| tail[3].eq_ignore_ascii_case(value));
    let python=language(text.as_bytes())==SourceLanguage::Python;
    if count < 2 || !(application_noun && named_ending || ui && product || play_tail || python) {
        return None;
    }
    Some(if game || play_tail {
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
    if language(input)==SourceLanguage::Python {return Some(ApplicationStyle::Terminal);}
    let text = core::str::from_utf8(input).ok()?;
    let terminal=text.split_ascii_whitespace().any(|word|["terminal","cli","command-line"]
        .iter().any(|value|word.trim_matches([',','.']).eq_ignore_ascii_case(value)));
    Some(
        if !terminal && (application_kind(input) == Some(ApplicationKind::Game)
            || application_kind(input) == Some(ApplicationKind::General)
            || text.split_ascii_whitespace().any(|word| {
            let word = word.trim_matches(|c: char| c == ',' || c == '.');
            ["ui", "uikit", "ui-kit", "gui", "graphical", "windowed", "interface"]
                .iter()
                .any(|value| word.eq_ignore_ascii_case(value))
        })) {
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
