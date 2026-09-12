#![allow(dead_code)]
#[path = "../kernel/ui/mod.rs"]
mod ui;
use ui::{
    app_assistant::{self as ai, Action, Target},
    editor_tools::{highlight, Language, Token},
    geometry::{Point, Rect},
    text_editor::{
        resolve_unsaved_decision, PendingDocumentAction, TextDocument, UnsavedDecision,
        UnsavedTransition, DOCUMENT_CAPACITY,
    },
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
    d.goto_line(2);
    assert!(d.outdent_line());
    assert_eq!(&d.bytes()[13..], b"let x = 42;\n}\n");
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
// FUNC: dirty_document_lifecycle
// DESC: Verifies that destructive editor transitions require an explicit save, save-as, discard, or cancellation outcome.
// ------------------=
fn dirty_document_lifecycle() {
    for action in [
        PendingDocumentAction::Close,
        PendingDocumentAction::New,
        PendingDocumentAction::Open,
    ] {
        assert_eq!(
            resolve_unsaved_decision(action, UnsavedDecision::Cancel, true),
            UnsavedTransition::Stay
        );
        assert_eq!(
            resolve_unsaved_decision(action, UnsavedDecision::Discard, true),
            UnsavedTransition::Perform(action)
        );
        assert_eq!(
            resolve_unsaved_decision(action, UnsavedDecision::Save, true),
            UnsavedTransition::Save(action)
        );
        assert_eq!(
            resolve_unsaved_decision(action, UnsavedDecision::Save, false),
            UnsavedTransition::SaveAs(action)
        );
    }
}
// ------------------------=
// FUNC: syntax
// DESC: Asserts token categories, escaped strings, multiline comments and filename-based modes rather than prose output.
// ------------------=
fn syntax() {
    let mut tools = ui::editor_tools::Presentation::new();
    for field in [
        ui::editor_tools::Field::Command,
        ui::editor_tools::Field::Find,
        ui::editor_tools::Field::GoTo,
        ui::editor_tools::Field::ReplaceFind,
    ] {
        tools.query_len = 6;
        tools.replacement_len = 3;
        tools.begin(field);
        assert!(tools.field == field);
        assert_eq!((tools.query_len, tools.replacement_len), (0, 0));
        tools.selection = Some((2, 5));
        tools.notice = b"transient";
        tools.finish_tool();
        assert!(tools.field == ui::editor_tools::Field::None);
        assert_eq!(tools.notice.len(), 0);
        assert_eq!(tools.selection, Some((2, 5)));
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
    let reference = Rect {
        x: 0,
        y: 0,
        width: 1250,
        height: 622,
    };
    let geometry = ai::geometry(reference, 1, true);
    assert_eq!(geometry.panel.width, 420);
    assert_eq!(geometry.composer.height, 40);
    assert_eq!(geometry.panel.right(), reference.right() - 1);
    assert_eq!(geometry.panel.bottom(), reference.bottom() - 1);
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
    menus_and_viewport();
    editing();
    dirty_document_lifecycle();
    syntax();
    panels();
    ai::reset();
    for id in 0..ai::PANEL_SLOTS {
        assert!(ai::read(id) == ai::Panel::new());
    }
}

// ------------------------=
// FUNC: menus_and_viewport
// DESC: Exercises executable command mappings, named syntax selection, keyboard state and reflowed hit bounds.
// ------------------=
fn menus_and_viewport() {
    for (width, height) in [(2048, 2048), (1920, 1080), (1280, 720), (3840, 2160)] {
        let state = ui::editor_chrome::default_window(width, height);
        let geometry = ui::system_layout::SystemLayout::new(width, height)
            .desktop_app_window_geometry(state.x, state.y, state.width, state.height, false);
        assert!((geometry.window.width as i64*622-geometry.window.height as i64*1250).abs()<5000,
            "framebuffer={width}x{height}, actual={}x{}",geometry.window.width,geometry.window.height);
    }
    use ui::{
        editor_chrome::{Command, Layout, Menu},
        editor_tools::{Field, Presentation, LANGUAGES},
    };
    let mut state = Presentation::new();
    state.open_menu(Menu::File);
    assert_eq!(state.choose_menu(), Some(Command::New));
    state.open_menu(Menu::File);
    state.menu_step(-1);
    assert_eq!(state.choose_menu(), Some(Command::Close));
    for (index, command) in [
        Command::New,
        Command::Open,
        Command::Save,
        Command::SaveAs,
        Command::Delete,
        Command::Close,
    ]
    .iter()
    .enumerate()
    {
        assert_eq!(Menu::File.entry(index).unwrap().command, *command);
    }
    for (index, language) in LANGUAGES.iter().enumerate() {
        state.open_menu(Menu::Syntax);
        state.menu_index = index;
        assert_eq!(state.choose_menu(), Some(Command::Language(index)));
        assert_eq!(state.language, *language);
        assert_eq!(state.menu, Menu::None);
        state.open_menu(Menu::Syntax);
        assert_eq!(state.menu_index, index);
    }
    for scale in [1, 2] {
        for width in [480, 800, 1440] {
            let window = Rect {
                x: 100,
                y: 100,
                width: width * scale,
                height: 700 * scale,
            };
            let plain = Layout::new(window, scale as usize, false, Field::None);
            let docked = Layout::new(window, scale as usize, true, Field::None);
            assert!(docked.columns() < plain.columns());
            assert!(docked.body.right() <= ai::geometry(window, scale as usize, true).panel.x);
            let find = Layout::new(window, scale as usize, true, Field::Find);
            assert!(find.rows() < docked.rows());
            for layout in [plain, docked, find] {
                let dialog = ui::editor_chrome::unsaved_dialog_geometry(layout.body, scale as usize);
                for rect in [dialog.sheet, dialog.cancel, dialog.discard, dialog.save] {
                    assert!(rect.x >= layout.body.x && rect.right() <= layout.body.right());
                    assert!(rect.y >= layout.body.y && rect.bottom() <= layout.body.bottom());
                }
                assert!(dialog.cancel.right() < dialog.discard.x);
                assert!(dialog.discard.right() < dialog.save.x);
                if layout.menu_bar.width > 700 * scale {
                    for index in 0..3 {
                        let action = layout.toolbar_action(index);
                        assert!(action.x >= layout.menu_bar.x);
                        assert!(action.right() <= layout.menu_bar.right());
                        assert!(action.y >= layout.menu_bar.y);
                        assert!(action.bottom() <= layout.menu_bar.bottom());
                    }
                }
                let tab=layout.document_tab();
                let close=layout.tab_close();
                let new=layout.new_document();
                assert!(close.x>=tab.x && close.right()<=tab.right());
                assert!(new.x>tab.right() && new.right()<=layout.document.right());
                assert!(close.bottom()<=layout.document.bottom() && new.bottom()<=layout.document.bottom());
                for menu in [
                    Menu::File,
                    Menu::Edit,
                    Menu::Selection,
                    Menu::View,
                    Menu::Syntax,
                ] {
                    let popup = layout.popup(menu);
                    assert!(popup.x >= window.x && popup.right() <= layout.body.right());
                    assert!(popup.y >= window.y && popup.bottom() <= window.bottom());
                    for index in 0..menu.count() {
                        let row = layout.menu_row(menu, index);
                        let point = Point {
                            x: row.x + 2,
                            y: row.y + 2,
                        };
                        assert_eq!(layout.row_at(menu, point), Some(index));
                    }
                }
                let scrollbar = layout.scrollbar(100, 100);
                assert!(scrollbar.thumb.bottom() <= scrollbar.track.bottom());
                assert!(scrollbar.track.right() <= layout.body.right());
            }
        }
    }
}
