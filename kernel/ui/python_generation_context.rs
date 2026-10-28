//! Small task-relevant guidance; discovery and execution authority remain separate.
use super::editor_compile_intent::{self, ApplicationKind, ApplicationStyle};

pub const CONTEXT_BYTES: usize = 2048;
pub const MAX_FAMILIES: usize = 2;
const BASE: &[u8] = include_bytes!("../../sdk/python/generation-libraries.txt");
const NATIVE_UI: &[u8] = b"\nNative GUI: import infinity as ui; do not implement the SDK. ui.window(title,width,height) once, then ui.run(render,handle). render() calls ui.state(ai_turn,terminal,description) FIRST; draws ui.button/label/display(id,x,y,w,h,text); offers legal ui.choice(id,action,label,a=0,b=0,c=0,d=0); ends ui.publish(). Stable positive IDs match controls/actions. handle(event) receives a dict: kind=ui.CLICK or ui.AI, id=control/action ID; validate, change app data, return True only for changes. run handles Close, yield, redraw and AI acknowledgement. Publish current truthful state and only legal actions. No tkinter/turtle: their installed widget gate is pending.\n";
const GAME: &[u8] = b"\nComplete the game's legal moves/rules, turn/state updates, win/draw detection and restart/quit. Accept real user input in a working interaction loop. Implement requested behavior, not empty classes or scaffolding.\n";
const OPPONENT: &[u8] = b"Human moves first. Human and AI are different sides; accept CLICK only on human turn and AI only on AI turn. state(ai_turn=True) only on AI turn; terminal=True only at game end, with no actions. Implement rules, reset and win/draw checks. Hermes chooses offered actions; do not invent an automatic opponent.\n";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Family { Arithmetic, Dates, Data, Models, Text, Compression, InMemorySql }
const FAMILIES: [Family; 7] = [Family::Arithmetic, Family::Dates, Family::Data,
    Family::Models, Family::Text, Family::Compression, Family::InMemorySql];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Interface { Terminal, NativeUi }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Selection {
    pub families: [Option<Family>; MAX_FAMILIES],
    pub interface: Interface,
    pub game: bool,
    pub opponent: bool,
}

impl Family {
    // ------------------------=
    // FUNC: modules
    // DESC: Names the small evidence-backed default subset associated with one existing inventory family.
    // ------------------=
    pub fn modules(self) -> &'static [&'static [u8]] {
        match self {
            Self::Arithmetic => &[b"math", b"decimal", b"fractions", b"statistics"],
            Self::Dates => &[b"datetime", b"calendar", b"zoneinfo"],
            Self::Data => &[b"json", b"csv", b"configparser", b"tomllib"],
            Self::Models => &[b"collections", b"itertools", b"functools", b"dataclasses", b"enum", b"heapq", b"bisect", b"random"],
            Self::Text => &[b"re", b"textwrap", b"html", b"urllib", b"xml"],
            Self::Compression => &[b"zlib", b"gzip", b"zipfile", b"tarfile", b"bz2", b"lzma"],
            Self::InMemorySql => &[b"sqlite3"],
        }
    }
    // ------------------------=
    // FUNC: terms
    // DESC: Matches generic functional needs, never particular application or game names.
    // ------------------=
    fn terms(self) -> &'static [&'static [u8]] {
        match self {
            Self::Arithmetic => &[b"arithmetic", b"calculate", b"calculator", b"decimal", b"statistics", b"math"],
            Self::Dates => &[b"calendar", b"date", b"dates", b"timezone", b"schedule", b"scheduling"],
            Self::Data => &[b"csv", b"json", b"records", b"configuration", b"config", b"toml"],
            Self::Models => &[b"random", b"shuffle", b"queue", b"combinations", b"permutations"],
            Self::Text => &[b"regex", b"regexp", b"parse", b"parser", b"text", b"html", b"xml"],
            Self::Compression => &[b"compress", b"compression", b"zip", b"gzip", b"archive"],
            Self::InMemorySql => &[b"sql", b"sqlite", b"sqlite3", b"database"],
        }
    }
    // ------------------------=
    // FUNC: hint
    // DESC: Gives a short optional purpose hint instead of enumerating the installed library directory.
    // ------------------=
    fn hint(self) -> &'static [u8] {
        match self {
            Self::Arithmetic => b"\nOptional arithmetic helpers: math, decimal.",
            Self::Dates => b"\nOptional date helpers: datetime, calendar.",
            Self::Data => b"\nOptional data parsers: json, csv.",
            Self::Models => b"\nUse lists/dicts for state; collections for specialized containers.",
            Self::Text => b"\nOptional text helpers: re, textwrap.",
            Self::Compression => b"\nOptional archive helpers: zipfile, gzip.",
            Self::InMemorySql => b"\nOptional SQL: sqlite3 in-memory only; durable database acceptance pending.",
        }
    }
}

// ------------------------=
// FUNC: word
// DESC: Tests bounded objective tokens without interpreting the objective as library availability or authority.
// ------------------=
fn word(text: &[u8], values: &[&[u8]]) -> bool {
    text.split(|b| !b.is_ascii_alphanumeric() && *b != b'_')
        .any(|part| values.iter().any(|value| part.eq_ignore_ascii_case(value)))
}

// ------------------------=
// FUNC: imports
// DESC: Finds literal import roots outside Python comments and strings without importing or executing any source.
// ------------------=
pub fn imports(source: &[u8], module: &[u8]) -> bool {
    let mut i = 0;
    let mut import_list = false;
    let mut want_root = false;
    let mut from_statement = false;
    while i < source.len() {
        let byte = source[i];
        if byte == b'#' {
            while i < source.len() && source[i] != b'\n' { i += 1; }
            continue;
        }
        if byte == b'\'' || byte == b'"' {
            let triple = source.get(i..i + 3).is_some_and(|v| v == [byte; 3]);
            i += if triple { 3 } else { 1 };
            while i < source.len() {
                if source[i] == b'\\' { i = (i + 2).min(source.len()); continue; }
                if source[i] == byte && (!triple || source.get(i..i + 3).is_some_and(|v| v == [byte; 3])) {
                    i += if triple { 3 } else { 1 }; break;
                }
                i += 1;
            }
            continue;
        }
        if byte.is_ascii_alphabetic() || byte == b'_' {
            let start = i; i += 1;
            while i < source.len() && (source[i].is_ascii_alphanumeric() || source[i] == b'_') { i += 1; }
            let token = &source[start..i];
            if want_root {
                if token == module { return true; }
                want_root = false;
            }
            if token == b"from" { import_list = false; want_root = true; from_statement = true; }
            else if token == b"import" && !from_statement { import_list = true; want_root = true; }
            continue;
        }
        if byte == b'\\' && source.get(i + 1) == Some(&b'\n') { i += 2; continue; }
        if byte == b'\n' || byte == b';' { import_list = false; want_root = false; from_statement = false; }
        else if byte == b',' && import_list { want_root = true; }
        else if byte == b'.' && want_root { want_root = false; }
        i += 1;
    }
    false
}

// ------------------------=
// FUNC: select
// DESC: Selects at most two relevant library families and only the requested or existing native GUI contract.
// ------------------=
pub fn select(request: &[u8], source: &[u8]) -> Selection {
    let explicit_ui = word(request, &[b"ui", b"uikit", b"gui", b"graphical", b"windowed", b"buttons"]);
    let explicit_terminal = word(request, &[b"terminal", b"cli", b"console"]);
    let current_ui = imports(source, b"infinity");
    let kind = editor_compile_intent::application_kind(request);
    let interface = if current_ui || !explicit_terminal && (explicit_ui
        || editor_compile_intent::application_style(request) == Some(ApplicationStyle::Ui)) {
        Interface::NativeUi
    } else { Interface::Terminal };
    let mut families = [None; MAX_FAMILIES];
    let mut count = 0;
    // Existing literal imports win for repairs; then explicit functional needs,
    // then the existing typed application kind. Selection is advice, not a ban.
    for priority in 0..3 {
        for family in FAMILIES {
            let relevant = match priority {
                0 => family.modules().iter().any(|module| imports(source, module)),
                1 => word(request, family.terms()),
                _ => family == Family::Arithmetic && kind == Some(ApplicationKind::Calculator)
                    || family == Family::Models && kind == Some(ApplicationKind::Game),
            };
            if relevant && count < MAX_FAMILIES && !families.contains(&Some(family)) {
                families[count] = Some(family); count += 1;
            }
        }
    }
    Selection { families, interface, game: kind == Some(ApplicationKind::Game), opponent: interface == Interface::NativeUi
        && editor_compile_intent::requests_opponent(request) }
}

// ------------------------=
// FUNC: format
// DESC: Writes one bounded shared guide with typed selection metadata, rejecting undersized buffers before any write.
// ------------------=
pub fn format(selection: Selection, output: &mut [u8]) -> Option<usize> {
    let ui = if selection.interface == Interface::NativeUi { NATIVE_UI } else { b"".as_slice() };
    let game = if selection.game { GAME } else { b"".as_slice() };
    let opponent = if selection.opponent { OPPONENT } else { b"".as_slice() };
    let size = BASE.len() + ui.len() + game.len() + opponent.len()
        + selection.families.iter().flatten().map(|family| family.hint().len()).sum::<usize>();
    if size > output.len().min(CONTEXT_BYTES) { return None; }
    output[..BASE.len()].copy_from_slice(BASE);
    let mut n = BASE.len();
    for family in selection.families.iter().flatten() {
        let hint = family.hint(); output[n..n + hint.len()].copy_from_slice(hint); n += hint.len();
    }
    for part in [game, ui, opponent] { output[n..n + part.len()].copy_from_slice(part); n += part.len(); }
    Some(n)
}

// ------------------------=
// FUNC: append_directory
// DESC: Inserts trusted discovery bytes before the exact final objective without searching source text, duplicating user instructions or partially writing on failure.
// ------------------=
pub fn append_directory(output: &mut [u8], length: usize, objective: &[u8], directory: &[u8]) -> Option<usize> {
    const HEADER: &[u8] = b"\nUSER REQUEST:\n";
    let suffix = HEADER.len().checked_add(objective.len())?;
    let at = length.checked_sub(suffix)?;
    let end = length.checked_add(directory.len())?;
    if directory.len() > 512 || end > output.len()
        || output.get(at..at + HEADER.len())? != HEADER
        || output.get(at + HEADER.len()..length)? != objective {
        return None;
    }
    output.copy_within(at..length, at + directory.len());
    output[at..at + directory.len()].copy_from_slice(directory);
    Some(end)
}
