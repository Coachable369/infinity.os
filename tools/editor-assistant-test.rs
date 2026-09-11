#![allow(dead_code)]
#[path = "../kernel/ui/mod.rs"]
mod ui;
use ui::{
    app_assistant::{self as ai, Action, Target},
    editor_tools::{highlight, Language, Token},
    geometry::{Point, Rect},
    text_editor::{TextDocument, DOCUMENT_CAPACITY},
};
// ------------------------=
// FUNC: editing
// DESC: Exercises real selection, atomic history, search, overflow rejection, indentation, and saved-state transitions.
// ------------------=
fn editing() {
    let mut d = TextDocument::new();
    assert!(d.open(b"fn main() {\n    let x = 42;\n}\n"));
    assert!(d.is_saved());
    d.select(3, 7);
    assert!(d.replace_selection(b"start"));
    assert!(!d.is_saved());
    assert!(d.undo());
    assert!(d.is_saved());
    assert!(d.redo());
    assert!(!d.is_saved());
    d.save();
    assert!(d.is_saved());
    assert!(d.find(b"LET", false));
    assert_eq!(d.selection(), Some((17, 20)));
    assert!(d.replace_selection(b"const"));
    assert!(d.undo());
    assert!(d.is_saved());
    assert_eq!(d.replace_all(b"x", b"counter"), Some(1));
    assert!(d.undo());
    assert!(d.is_saved());
    d.goto_line(2);
    assert_eq!(d.cursor(), 13);
    d.set_cursor(17);
    assert!(d.newline_indented());
    assert_eq!(&d.bytes()[13..22], b"    \n    ");
    assert!(d.undo());
    assert!(d.open(&[b'a'; DOCUMENT_CAPACITY]));
    d.select(1, 2);
    assert!(!d.replace_selection(b"too large"));
    assert_eq!(d.selection(), Some((1, 2)));
    assert_eq!(d.bytes().len(), DOCUMENT_CAPACITY);
    assert_eq!(d.replace_all(b"a", b"aa"), None);
    assert_eq!(d.bytes().len(), DOCUMENT_CAPACITY);
    assert!(d.replace_selection(b"z"));
    assert!(d.undo());
    assert!(d.is_saved());
    for _ in 0..20 {
        d.select(0, 1);
        assert!(d.replace_selection(b"b"));
    }
    for _ in 0..8 {
        assert!(d.undo());
    }
    assert!(!d.undo());
    for _ in 0..8 {
        assert!(d.redo());
    }
    assert!(!d.redo());
    assert!(d.open(b"\tcode\r\n"));
    assert!(d.is_saved());
    assert!(!d.open(&[0xff]));
    assert_eq!(d.bytes(), b"\tcode\r\n");
}
// ------------------------=
// FUNC: syntax
// DESC: Asserts token categories, escaped strings, multiline comments and filename-based modes rather than prose output.
// ------------------=
fn syntax() {
    let mut tools = ui::editor_tools::Presentation::new();
    for field in [ui::editor_tools::Field::Command, ui::editor_tools::Field::Find, ui::editor_tools::Field::GoTo, ui::editor_tools::Field::ReplaceFind] {
        tools.query_len = 6;
        tools.replacement_len = 3;
        tools.begin(field);
        assert!(tools.field == field);
        assert_eq!((tools.query_len, tools.replacement_len), (0, 0));
    }
    // Bundled code-font ink must fit the same cells used for pointer and caret geometry.
    for glyph in 0..95 {
        for row in 0..ui::editor_tools::FONT_HEIGHT {
            for column in ui::editor_tools::CELL_WIDTH..ui::editor_tools::FONT_WIDTH {
                assert!(
                    ui::editor_tools::FONT_ATLAS[row * ui::editor_tools::FONT_WIDTH * 95
                        + glyph * ui::editor_tools::FONT_WIDTH
                        + column]
                        <= 8
                );
            }
        }
    }
    let input = b"let x = 42; /* a\nb */ \"//not comment\"";
    let mut tokens = [Token::Text; 128];
    highlight(input, Language::Rust, &mut tokens);
    assert_eq!(tokens[0], Token::Keyword);
    assert_eq!(tokens[8], Token::Number);
    assert_eq!(tokens[16], Token::Comment);
    assert_eq!(tokens[24], Token::String);
    highlight(b"# note\nx = 'a\\'b'", Language::Python, &mut tokens);
    assert_eq!(tokens[0], Token::Comment);
    assert_eq!(tokens[14], Token::String);
    for (file, language) in [
        (b"a.rs".as_slice(), Language::Rust),
        (b"a.py", Language::Python),
        (b"a.ts", Language::JavaScript),
        (b"a.cpp", Language::C),
        (b"a.json", Language::Json),
        (b"a.sh", Language::Shell),
        (b"a.html", Language::Html),
        (b"a.css", Language::Css),
        (b"a.txt", Language::Plain),
    ] {
        assert_eq!(Language::detect(file), language);
    }
}
// ------------------------=
// FUNC: panels
// DESC: Verifies independent window state, action allowlists, explicit approval, stale-context rejection and hit geometry.
// ------------------=
fn panels() {
    for scale in 1..=2 {
        for width in [480, 800, 1200] {
            let window = Rect {
                x: 20,
                y: 30,
                width: width * scale,
                height: 640 * scale,
            };
            for expanded in [false, true] {
                let g = ai::geometry(window, scale as usize, expanded);
                assert!(window.contains(Point {
                    x: g.toggle.x,
                    y: g.toggle.y
                }));
                assert!(g.toggle.right() <= window.right());
                assert_eq!(
                    ai::hit(
                        g,
                        expanded,
                        Point {
                            x: g.toggle.x + 2,
                            y: g.toggle.y + 2
                        }
                    ),
                    Some(Target::Toggle)
                );
                if expanded {
                    for r in [g.panel, g.composer, g.send, g.apply, g.dismiss] {
                        assert!(
                            r.x >= window.x
                                && r.right() <= window.right()
                                && r.y >= window.y
                                && r.bottom() <= window.bottom()
                        );
                    }
                    assert_eq!(
                        ai::hit(
                            g,
                            true,
                            Point {
                                x: g.composer.x + 1,
                                y: g.composer.y + 1
                            }
                        ),
                        Some(Target::Composer)
                    );
                }
            }
        }
    }
    let mut p = ai::Panel::new();
    p.input[..12].copy_from_slice(b"insert hello");
    p.length = 12;
    assert!(!p.propose(false, 7));
    assert_eq!(p.pending, Action::None);
    assert!(p.propose(true, 7));
    assert_eq!(p.pending, Action::Insert);
    assert_eq!(p.take_action(8), Action::None);
    assert_eq!(p.pending, Action::None);
    assert!(p.propose(true, 7));
    assert_eq!(p.take_action(7), Action::Insert);
    assert_eq!(p.take_action(7), Action::None);
    p.input[..8].copy_from_slice(b"rm -rf /");
    p.length = 8;
    assert!(!p.propose(true, 7));
    p.expanded = true;
    ai::write(2, p);
    assert!(ai::read(2).expanded);
    assert!(!ai::read(1).expanded);
    assert_ne!(
        ai::fingerprint(b"abc", 1, None),
        ai::fingerprint(b"abc", 2, None)
    );
    let mut server = ui::window::WindowServer::new();
    let owner = ui::window::ContextId(1);
    let id = server
        .create(
            owner,
            ui::window::SurfaceId(1),
            Rect {
                x: 0,
                y: 0,
                width: 800,
                height: 600,
            },
            ui::window::ZOrderClass::Normal,
        )
        .unwrap();
    assert!(server
        .inspect(id)
        .unwrap()
        .assistant_geometry(1, false)
        .is_some());
}
// ------------------------=
// FUNC: main
// DESC: Runs deterministic editor and universal-assistant acceptance against production typed implementations.
// ------------------=
fn main() {
    editing();
    syntax();
    panels();
    ai::reset();
    for id in 0..ai::PANEL_SLOTS {
        assert!(ai::read(id) == ai::Panel::new());
    }
}
