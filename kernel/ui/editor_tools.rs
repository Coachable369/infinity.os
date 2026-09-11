//! Native code-editor presentation and allocation-free lexical classification.
pub const CELL_WIDTH: usize = 12;
pub const FONT_WIDTH: usize = 18;
pub const FONT_HEIGHT: usize = 24;
pub const FONT_ATLAS: &[u8; FONT_WIDTH * FONT_HEIGHT * 95] =
    include_bytes!("../../assets/fonts/InfinityEditor-Mono-18.atlas");
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Language {
    Plain,
    Rust,
    Python,
    JavaScript,
    C,
    Json,
    Shell,
    Html,
    Css,
}
pub const LANGUAGES: [Language; 9] = [
    Language::Plain,
    Language::Rust,
    Language::Python,
    Language::JavaScript,
    Language::C,
    Language::Json,
    Language::Shell,
    Language::Html,
    Language::Css,
];
impl Language {
    // ------------------------=
    // FUNC: name
    // DESC: Returns the syntax mode shown in the status rail.
    // ------------------=
    pub fn name(self) -> &'static [u8] {
        match self {
            Self::Plain => b"Plain text",
            Self::Rust => b"Rust",
            Self::Python => b"Python",
            Self::JavaScript => b"JavaScript",
            Self::C => b"C / C++",
            Self::Json => b"JSON",
            Self::Shell => b"Shell",
            Self::Html => b"HTML",
            Self::Css => b"CSS",
        }
    }
    // ------------------------=
    // FUNC: detect
    // DESC: Resolves a language from the actual opened or saved filename.
    // ------------------=
    pub fn detect(name: &[u8]) -> Self {
        let ext = name.rsplit(|b| *b == b'.').next().unwrap_or(b"");
        match ext {
            b"rs" => Self::Rust,
            b"py" => Self::Python,
            b"js" | b"ts" | b"jsx" | b"tsx" => Self::JavaScript,
            b"c" | b"h" | b"cpp" | b"hpp" => Self::C,
            b"json" => Self::Json,
            b"sh" | b"bash" | b"zsh" => Self::Shell,
            b"html" | b"xml" => Self::Html,
            b"css" => Self::Css,
            _ => Self::Plain,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Token {
    Text,
    Keyword,
    String,
    Number,
    Comment,
    Punctuation,
}
impl Token {
    // ------------------------=
    // FUNC: color
    // DESC: Supplies the shared IDesign syntax palette.
    // ------------------=
    pub fn color(self) -> (u8, u8, u8) {
        match self {
            Self::Text => (221, 232, 244),
            Self::Keyword => (73, 215, 249),
            Self::String => (139, 220, 167),
            Self::Number => (247, 194, 115),
            Self::Comment => (116, 146, 165),
            Self::Punctuation => (188, 163, 232),
        }
    }
}
// ------------------------=
// FUNC: highlight
// DESC: Classifies the full document so scrolled strings and multiline comments retain their lexical state.
// ------------------=
pub fn highlight(bytes: &[u8], language: Language, out: &mut [Token]) {
    let n = bytes.len().min(out.len());
    out[..n].fill(Token::Text);
    if language == Language::Plain {
        return;
    }
    let hash_comments = matches!(language, Language::Python | Language::Shell);
    let slash_comments = matches!(
        language,
        Language::Rust | Language::JavaScript | Language::C | Language::Css
    );
    let keywords: &[&[u8]] = match language {
        Language::Rust => &[
            b"fn", b"let", b"mut", b"pub", b"use", b"mod", b"impl", b"struct", b"enum", b"match",
            b"if", b"else", b"for", b"while", b"return", b"self", b"Self", b"const", b"true",
            b"false", b"Some", b"None", b"Ok", b"Err",
        ],
        Language::Python => &[
            b"def", b"class", b"import", b"from", b"as", b"if", b"else", b"elif", b"for", b"in",
            b"while", b"return", b"try", b"except", b"with", b"True", b"False", b"None", b"and",
            b"or", b"not", b"async", b"await",
        ],
        Language::JavaScript => &[
            b"function",
            b"const",
            b"let",
            b"var",
            b"class",
            b"new",
            b"return",
            b"if",
            b"else",
            b"for",
            b"while",
            b"import",
            b"export",
            b"from",
            b"async",
            b"await",
            b"true",
            b"false",
            b"null",
            b"undefined",
            b"throw",
            b"try",
            b"catch",
        ],
        Language::C => &[
            b"int",
            b"char",
            b"void",
            b"float",
            b"double",
            b"struct",
            b"class",
            b"const",
            b"static",
            b"return",
            b"if",
            b"else",
            b"while",
            b"for",
            b"unsigned",
            b"bool",
            b"true",
            b"false",
            b"include",
            b"define",
        ],
        Language::Json => &[b"true", b"false", b"null"],
        Language::Shell => &[
            b"if",
            b"then",
            b"else",
            b"fi",
            b"for",
            b"do",
            b"done",
            b"case",
            b"esac",
            b"function",
            b"export",
            b"local",
        ],
        _ => &[],
    };
    let mut i = 0;
    while i < n {
        let start = i;
        let tail = &bytes[i..n];
        let token = if (hash_comments && bytes[i] == b'#')
            || (slash_comments && tail.starts_with(b"//"))
        {
            while i < n && bytes[i] != b'\n' {
                i += 1;
            }
            Token::Comment
        } else if slash_comments && tail.starts_with(b"/*") {
            i += 2;
            while i < n && !bytes[i..n].starts_with(b"*/") {
                i += 1;
            }
            i = (i + 2).min(n);
            Token::Comment
        } else if language == Language::Html && tail.starts_with(b"<!--") {
            i += 4;
            while i < n && !bytes[i..n].starts_with(b"-->") {
                i += 1;
            }
            i = (i + 3).min(n);
            Token::Comment
        } else if matches!(bytes[i], b'"' | b'\'' | b'`') {
            let quote = bytes[i];
            i += 1;
            while i < n {
                if bytes[i] == b'\\' {
                    i = (i + 2).min(n);
                } else if bytes[i] == quote {
                    i += 1;
                    break;
                } else {
                    i += 1;
                }
            }
            Token::String
        } else if bytes[i].is_ascii_digit() {
            i += 1;
            while i < n && (bytes[i].is_ascii_alphanumeric() || matches!(bytes[i], b'.' | b'_')) {
                i += 1;
            }
            Token::Number
        } else if bytes[i].is_ascii_alphabetic() || bytes[i] == b'_' {
            i += 1;
            while i < n && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            if keywords.contains(&&bytes[start..i]) {
                Token::Keyword
            } else {
                Token::Text
            }
        } else {
            i += 1;
            if b"{}[]():;=<>+-*/!&,.$".contains(&bytes[start]) {
                Token::Punctuation
            } else {
                Token::Text
            }
        };
        out[start..i].fill(token);
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Field {
    None,
    Find,
    ReplaceFind,
    ReplaceWith,
    GoTo,
    Command,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Presentation {
    pub menu: super::editor_chrome::Menu,
    pub menu_index: usize,
    pub filename: [u8; 64],
    pub filename_len: usize,
    pub language: Language,
    pub selection: Option<(usize, usize)>,
    pub cursor: usize,
    pub field: Field,
    pub query: [u8; 96],
    pub query_len: usize,
    pub replacement: [u8; 96],
    pub replacement_len: usize,
    pub notice: &'static [u8],
    pub selecting: bool,
}
impl Presentation {
    // ------------------------=
    // FUNC: open_menu
    // DESC: Opens one menu with focus on the selected syntax or first command.
    // ------------------=
    pub fn open_menu(&mut self, menu: super::editor_chrome::Menu) {
        self.menu = menu;
        self.menu_index = if menu == super::editor_chrome::Menu::Syntax {
            LANGUAGES
                .iter()
                .position(|l| *l == self.language)
                .unwrap_or(0)
        } else {
            0
        };
    }
    // ------------------------=
    // FUNC: menu_step
    // DESC: Wraps keyboard focus within executable entries.
    // ------------------=
    pub fn menu_step(&mut self, delta: isize) {
        let count = self.menu.count();
        if count > 0 {
            self.menu_index =
                (self.menu_index as isize + delta).rem_euclid(count as isize) as usize;
        }
    }
    // ------------------------=
    // FUNC: choose_menu
    // DESC: Closes the menu and resolves its focused command, applying syntax selections directly.
    // ------------------=
    pub fn choose_menu(&mut self) -> Option<super::editor_chrome::Command> {
        let command = self.menu.entry(self.menu_index)?.command;
        self.menu = super::editor_chrome::Menu::None;
        if let super::editor_chrome::Command::Language(i) = command {
            self.language = LANGUAGES[i];
        }
        Some(command)
    }

    // ------------------------=
    // FUNC: begin
    // DESC: Starts a fresh inline tool without carrying a command name or line number into its search fields.
    // ------------------=
    pub fn begin(&mut self, field: Field) {
        self.field = field;
        self.query_len = 0;
        self.replacement_len = 0;
    }
    // ------------------------=
    // FUNC: new
    // DESC: Creates independent editor-tool state without touching the document.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            menu: super::editor_chrome::Menu::None,
            menu_index: 0,
            filename: [0; 64],
            filename_len: 0,
            language: Language::Plain,
            selection: None,
            cursor: 0,
            field: Field::None,
            query: [0; 96],
            query_len: 0,
            replacement: [0; 96],
            replacement_len: 0,
            notice: b"",
            selecting: false,
        }
    }
}
static mut VIEW: Presentation = Presentation::new();
static REVISION: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
// ------------------------=
// FUNC: publish
// DESC: Publishes bounded presentation on the serialized native UI thread.
// ------------------=
pub fn publish(value: Presentation) {
    unsafe {
        if *(&raw const VIEW) != value {
            *(&raw mut VIEW) = value;
            REVISION.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        }
    }
}
// ------------------------=
// FUNC: revision
// DESC: Invalidates retained editor pixels only after an observable presentation change.
// ------------------=
pub fn revision() -> u32 {
    REVISION.load(core::sync::atomic::Ordering::Relaxed)
}
// ------------------------=
// FUNC: current
// DESC: Reads the editor presentation without exposing document ownership.
// ------------------=
pub fn current() -> Presentation {
    unsafe { *(&raw const VIEW) }
}
