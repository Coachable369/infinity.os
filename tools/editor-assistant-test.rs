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
    assert_eq!(geometry.toggle.x, reference.right() - 1);
    assert_eq!(geometry.toggle.width, ai::TAB_WIDTH as u32);
    assert_eq!(geometry.toggle.height, ai::TAB_HEIGHT as u32);
    assert!(geometry.toggle.right() > reference.right());
    let collapsed_geometry = ai::geometry(reference, 1, false);
    assert_eq!(collapsed_geometry.toggle.height, ai::TAB_HEIGHT as u32);
    let maximized = ai::geometry_in_viewport(Rect{x:0,y:0,width:1920,height:1080},1920,1,false);
    assert!(maximized.toggle.x>=0);
    assert!(maximized.toggle.right()<=1920);
    assert_eq!(maximized.toggle.height,ai::TAB_HEIGHT as u32);
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
                assert_eq!(g.toggle.x, window.right() - scale as i32);
                assert!(g.toggle.right() > window.right());
                assert!(!g.tab_left);
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
    let edge_window = Rect {
        x: 760,
        y: 30,
        width: 240,
        height: 600,
    };
    let edge_geometry = ai::geometry_in_viewport(edge_window, 1000, 1, false);
    assert!(edge_geometry.tab_left);
    assert_eq!(edge_geometry.toggle.right(), edge_window.x + 1);
    assert!(edge_geometry.toggle.x < edge_window.x);
    assert_eq!(
        ai::hit(
            edge_geometry,
            false,
            Point {
                x: edge_geometry.toggle.x + 2,
                y: edge_geometry.toggle.y + 2,
            },
        ),
        Some(Target::Toggle)
    );
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
    let command = b"open folder /home/default/documents";
    p.input[..command.len()].copy_from_slice(command);
    p.length = command.len();
    assert!(p.propose(false, 7));
    assert_eq!(p.take_action(7), Action::Navigate);
    assert_eq!(&p.argument[..p.argument_len], b"/home/default/documents");
    assert_eq!(p.take_action(7), Action::None);
    p.expanded = true;
    ai::write(2, p);
    assert!(ai::read(2).expanded);
    assert!(!ai::read(1).expanded);
    let mut hover = ai::Panel::new();
    hover.hovered = true;
    ai::write(12, hover);
    assert!(ai::set_hovered(Some(12)) || ai::read(12).hovered);
    let mut stale = ai::Panel::new();
    stale.hovered = true;
    ai::write(11, stale);
    assert!(ai::set_hovered(Some(12)));
    assert!(!ai::read(11).hovered);
    assert!(ai::read(12).hovered);
    let revision = ai::revision();
    assert!(ai::animation_tick());
    assert!(ai::revision() > revision);
    assert_eq!(ai::read(12).glow_phase, 1);
    assert!(ai::glow_intensity(15) > ai::glow_intensity(0));
    assert_eq!(ai::glow_intensity(15), ai::glow_intensity(16));
    hover = ai::read(12);
    hover.hovered = false;
    ai::write(12, hover);
    assert!(ai::animation_tick());
    assert_eq!(ai::read(12).glow_phase, 0);
    assert!(!ai::animation_tick());
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
    let floating = server
        .create(
            owner,
            ui::window::SurfaceId(2),
            Rect {
                x: 30,
                y: 40,
                width: 600,
                height: 400,
            },
            ui::window::ZOrderClass::Floating,
        )
        .unwrap();
    assert!(server
        .inspect(floating)
        .unwrap()
        .assistant_geometry(1, false)
        .is_some());
    let desktop = server
        .create(
            owner,
            ui::window::SurfaceId(3),
            Rect {
                x: 0,
                y: 0,
                width: 1280,
                height: 720,
            },
            ui::window::ZOrderClass::Desktop,
        )
        .unwrap();
    assert!(server
        .inspect(desktop)
        .unwrap()
        .assistant_geometry(1, false)
        .is_none());
}
// ------------------------=
// FUNC: main
// DESC: Runs deterministic editor and universal-assistant acceptance against production typed implementations.
// ------------------=
fn main() {
    assistant_controls();
    contextual_searches();
    generated_editor_actions();
    attached_assistant_damage();
    caret_damage_is_bounded();
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
// FUNC: assistant_controls
// DESC: Exercises real scoped action selection, close hit regions, and proportional scroll bounds at multiple scales.
// ------------------=
fn assistant_controls() {
    let mut proposal=ai::Panel::new();
    proposal.input[..5].copy_from_slice(b"draft");proposal.length=5;proposal.caret=2;
    assert!(proposal.accept_for_owner(0,b"ACTION\nnew tab\nEND_ACTION",true));
    assert_eq!(proposal.pending,Action::NewTab);assert_eq!(&proposal.input[..proposal.length],b"draft");assert_eq!(proposal.caret,2);
    assert!(!proposal.accept_for_owner(5,b"ACTION\nnew tab\nEND_ACTION",true));
    assert_eq!(proposal.pending,Action::None);
    assert!(!proposal.accept_for_owner(0,b"ACTION\nnew tab",true));
    assert!(!proposal.accept_for_owner(0,b"ACTION\nnew tab\nEND_ACTION",false));
    assert!(!proposal.accept_for_owner(4,b"ACTION\nopen folder /\nEND_ACTION",true));
    assert!(!ai::supports(12,Action::Maximize));assert!(ai::supports(12,Action::Close));
    for (owner,text,expected) in [(0,b"new tab".as_slice(),Action::NewTab),(0,b"zoom in",Action::ZoomIn),
        (5,b"grid view",Action::GridView),(3,b"next task",Action::NextTask),(4,b"next icon set",Action::CycleIcons),
        (1,b"type echo Hello",Action::DraftCommand)] {
        let mut panel=ai::Panel::new();panel.input[..text.len()].copy_from_slice(text);panel.length=text.len();
        assert!(panel.propose_contextual(owner));assert_eq!(panel.take_action(0),expected);
        assert_eq!(panel.take_action(0),Action::None);
        assert!(!panel.propose_contextual(15));
    }
    for s in 1..=3 {
        let g=ai::geometry(Rect{x:32,y:32,width:800*s,height:600*s},s as usize,true);
        assert_eq!(ai::hit(g,true,Point{x:g.close.x+1,y:g.close.y+1}),Some(Target::Close));
        assert_eq!(ai::hit(g,false,Point{x:g.close.x+1,y:g.close.y+1}),None);
        assert!(g.body.bottom()<g.apply.y);assert!(g.close.bottom()<g.body.y);
        let first=ai::scroll_thumb(g,0,2000);let last=ai::scroll_thumb(g,2000,2000);
        assert_eq!(first.y,g.scrollbar.y);assert_eq!(last.bottom(),g.scrollbar.bottom());
        assert_eq!(ai::scroll_thumb(g,u32::MAX,2000),last);
        assert_eq!(ai::hit(g,true,Point{x:g.scrollbar.x+1,y:g.scrollbar.y+1}),Some(Target::Scrollbar));
    }
}

// ------------------------=
// FUNC: contextual_searches
// DESC: Verifies app isolation, argument fidelity, whitespace, and one-shot contextual command dispatch.
// ------------------=
fn contextual_searches() {
    ai::reset();
    let before = ai::revision();
    assert!(!ai::expanded(0));
    assert!(!ai::expanded(ai::PANEL_SLOTS));
    for _ in 0..1000 { assert!(!ai::expanded(0)); }
    assert_eq!(ai::revision(), before);
    let mut visible = ai::Panel::new(); visible.expanded = true;
    ai::write(0, visible);
    assert!(ai::expanded(0));
    ai::reset();
    for (owner, input, action, argument) in [
        (0, "Search google for Hello World", Action::SearchWeb, "Hello World"),
        (5, "Find hello.c", Action::FindFile, "hello.c"),
        (7, " please FIND file Hello.c  ", Action::FindFile, "Hello.c"),
    ] {
        let mut panel = ai::Panel::new();
        panel.input[..input.len()].copy_from_slice(input.as_bytes());
        panel.length = input.len();
        assert!(panel.propose_contextual(owner));
        assert_eq!(&panel.argument[..panel.argument_len], argument.as_bytes());
        assert_eq!(panel.generation, ai::GenerationStatus::Idle);
        assert_eq!(panel.take_action(0), action);
        assert_eq!(panel.take_action(0), Action::None);
    }
    for (owner, input) in [(2,"Find hello.c"),(0,"Find hello.c"),(5,"Search google for Hello"),(0,"google   ")] {
        let mut panel = ai::Panel::new();
        panel.input[..input.len()].copy_from_slice(input.as_bytes()); panel.length=input.len();
        assert!(!panel.propose_contextual(owner));
        assert_eq!(panel.pending,Action::None);
    }
}

// ------------------------=
// FUNC: generated_editor_actions
// DESC: Exercises complete model edits, exact document bytes, undo, command aliases, stale replies and malformed output rejection.
// ------------------=
fn generated_editor_actions() {
    let mut panel = ai::Panel::new();
    let mut document = TextDocument::new();
    assert!(document.open(b"hello world"));
    document.select(6,11);
    panel.document_revision = 12;
    assert!(panel.accept_generated(b"INSERT\nRust\nEND_ACTION", true, true));
    let action = panel.take_action(12);
    assert_eq!(action, Action::Insert);
    assert!(panel.edit_document(action, &mut document));
    assert_eq!(document.bytes(), b"hello Rust");
    assert!(document.undo());
    assert_eq!(document.bytes(), b"hello world");
    assert!(document.is_saved());
    assert!(panel.accept_generated(b"REPLACE\nfn main() {\n    println!(\"Hi\");\n}\n\nEND_ACTION", true, true));
    let action = panel.take_action(12);
    assert!(panel.edit_document(action, &mut document));
    assert_eq!(document.bytes(), b"fn main() {\n    println!(\"Hi\");\n}\n");
    assert!(!document.is_saved());
    assert!(document.undo());
    assert_eq!(document.bytes(), b"hello world");
    for command in [b"clear text".as_slice(), b"clear document", b"clear"] {
        panel.input[..command.len()].copy_from_slice(command);
        panel.length = command.len();
        assert!(panel.propose(true, 12));
        let action = panel.take_action(12);
        assert_eq!(action, Action::Clear);
        assert!(panel.edit_document(action, &mut document));
        assert!(document.bytes().is_empty());
        assert!(document.undo());
        assert_eq!(document.bytes(), b"hello world");
    }
    for command in [b"save".as_slice(), b"save file", b"save document"] {
        panel.input[..command.len()].copy_from_slice(command);
        panel.length = command.len();
        assert!(panel.propose(true, 12));
        assert_eq!(panel.take_action(12), Action::Save);
        assert_eq!(panel.take_action(12), Action::None);
    }
    assert!(!panel.accept_generated(b"INSERT\npartial", true, false));
    assert_eq!(panel.pending, Action::None);
    assert!(!panel.accept_generated(b"INSERT\nx\nEND_ACTION", true, false));
    for invalid in [b"SAVE\n/path\nEND_ACTION".as_slice(),
        b"EXEC\nrm -rf /\nEND_ACTION", b"INSERT\n\xff\nEND_ACTION", b"INSERT\n\nEND_ACTION"] {
        assert!(!panel.accept_generated(invalid, true, true));
        assert_eq!(panel.pending, Action::None);
        assert_eq!(document.bytes(), b"hello world");
    }
    assert!(!panel.accept_generated(b"INSERT\nx\nEND_ACTION", false, true));
    assert!(panel.accept_generated(b"INSERT\nx\nEND_ACTION", true, true));
    assert_eq!(panel.take_action(13), Action::None);
    assert!(panel.accept_generated(b"CHAT\nHello", true, true));
    assert_eq!(panel.pending, Action::None);
    let oversized = [b"INSERT\n".as_slice(), &[b'x'; 4097], b"\nEND_ACTION"].concat();
    assert!(!panel.accept_generated(&oversized, true, true));
    assert!(document.open(&[b'a'; DOCUMENT_CAPACITY]));
    document.set_cursor(4);
    assert!(panel.accept_generated(b"INSERT\nx\nEND_ACTION", true, true));
    assert!(!panel.edit_document(Action::Insert, &mut document));
    assert_eq!(document.bytes().len(), DOCUMENT_CAPACITY);
    assert_eq!(document.cursor(), 4);
}

// ------------------------=
// FUNC: attached_assistant_damage
// DESC: Exercises expansion, editing, and collapse revisions and verifies bounded damage covers the complete rail and halo.
// ------------------=
fn attached_assistant_damage() {
    ai::reset();
    for scale in [1usize, 2, 3] {
        let display = Rect { x: 0, y: 0, width: 1920 * scale as u32, height: 1080 * scale as u32 };
        let window = Rect { x: 120 * scale as i32, y: 140 * scale as i32,
            width: 1250 * scale as u32, height: 700 * scale as u32 };
        let mut panel = ai::Panel::new();
        for stage in 0..3 {
            let previous = ai::revision();
            panel.expanded = stage != 2;
            if stage == 1 { panel.input[0] = b'A'; panel.length = 1; }
            ai::write(5, panel);
            let current = ai::revision();
            for screen in [2, 4, 8, 9, 10, 11] {
                let owner_damage = ui::redraw::assistant_requires_owner_damage(screen, previous, current);
                assert!(owner_damage);
                assert!(!ui::redraw::desktop_chat_content_requires_bounded_redraw(screen, true, owner_damage));
                assert!(ui::redraw::chat_requires_independent_widget_damage(screen, true));
            }
            let geometry = ai::geometry_in_viewport(window, display.width as usize, scale, panel.expanded);
            let damage = ui::system_layout::window_transition_damage(window, window, display,
                ((ai::TAB_WIDTH + 12) * scale) as u32);
            assert_eq!(damage.intersection(geometry.panel), geometry.panel);
            let halo = Rect { x: geometry.toggle.x - 4 * scale as i32,
                y: geometry.toggle.y - 4 * scale as i32,
                width: geometry.toggle.width + 8 * scale as u32,
                height: geometry.toggle.height + 8 * scale as u32 };
            assert_eq!(damage.intersection(halo), halo);
            assert!(damage.width as u64 * (damage.height as u64) < display.width as u64 * display.height as u64);
            assert!(!ui::redraw::assistant_requires_owner_damage(2, current, current));
        }
    }
    assert!(ui::redraw::desktop_chat_content_requires_bounded_redraw(2, true, false));
    assert!(!ui::redraw::assistant_requires_owner_damage(5, 0, 1));
}

// ------------------------=
// FUNC: caret_damage_is_bounded
// DESC: Verifies blinking damages only visible caret strips across wrapping, scrolling and scale changes.
// ------------------=
fn caret_damage_is_bounded() {
    for scale in 1..=3 {
        let g = ui::editor_chrome::Layout::new(
            Rect {
                x: 10,
                y: 20,
                width: 1000 * scale,
                height: 650 * scale,
            },
            scale as usize,
            false,
            ui::editor_tools::Field::None,
        );
        let input = vec![b'a'; g.columns() * 100];
        for cursor in [0, 3, g.columns(), input.len()] {
            let rects: Vec<_> = g.caret_damage(&input, cursor, 0).collect();
            assert!(rects.len() <= 2);
            assert!(rects
                .iter()
                .all(|r| r.width == scale && r.height == 19 * scale));
            assert!(rects.iter().map(|r| r.width * r.height).sum::<u32>() <= 38 * scale * scale);
            if cursor == input.len() {
                assert!(rects.is_empty());
            }
        }
        assert!(g.caret_damage(&input, 0, 50).next().is_none());
        let empty: Vec<_> = g.caret_damage(b"", 0, 0).collect();
        assert_eq!(empty.len(), 1);
        assert_eq!(
            empty[0].x,
            g.body.x + (ui::editor_chrome::CODE_INSET * scale as usize) as i32
        );
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
        assert!(
            (geometry.window.width as i64 * 622 - geometry.window.height as i64 * 1250).abs()
                < 5000,
            "framebuffer={width}x{height}, actual={}x{}",
            geometry.window.width,
            geometry.window.height
        );
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
                let dialog =
                    ui::editor_chrome::unsaved_dialog_geometry(layout.body, scale as usize);
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
                let tab = layout.document_tab();
                let close = layout.tab_close();
                let new = layout.new_document();
                assert!(close.x >= tab.x && close.right() <= tab.right());
                assert!(new.x > tab.right() && new.right() <= layout.document.right());
                assert!(
                    close.bottom() <= layout.document.bottom()
                        && new.bottom() <= layout.document.bottom()
                );
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
