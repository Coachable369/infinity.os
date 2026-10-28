#![allow(dead_code)]
#[path = "../kernel/ui/mod.rs"]
mod ui;
#[path = "editor-source-frame-tests.rs"]
mod source_frame_tests;
#[path = "python-generation-context-tests.rs"]
mod python_generation_context_tests;
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
    for scale in 1..=4 {
        let window=Rect{x:100,y:80,width:800*scale as u32,height:500*scale as u32};
        let g=ai::geometry_for_owner(0,window,4000,scale,false);
        assert_eq!(g.toggle.width,32*scale as u32);
        assert!(g.toggle.right()<=window.right()+((ai::TAB_WIDTH+12)*scale) as i32);
        assert_eq!(g.toggle.height,g.toggle.width);
        assert_eq!(g.toggle.y,window.y+12*scale as i32);
        assert_eq!(ai::hit(g,false,Point{x:g.toggle.x+20*scale as i32,y:g.toggle.y+20*scale as i32}),Some(Target::Toggle));
        let other=ai::geometry_for_owner(2,window,4000,scale,false);
        assert_eq!(other.toggle.height,ai::TAB_HEIGHT as u32*scale as u32);
        let maximized=ai::geometry_for_owner(0,Rect{x:0,y:0,width:1920,height:1080},1920,scale,false);
        assert!(maximized.toggle.y>=48*scale as i32);
        assert!(maximized.toggle.right()<=1920);
    }
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
    python_generation_context_tests::run();
    surgical_source_transactions();
    source_frame_tests::run();
    python_and_existing_source_workflows();
    raw_python_generation_frames();
    prefaced_source_generation_frames();
    continued_source_transactions();
    generated_repairs_preserve_source_identity();
    rejected_creation_retains_terminal_state();
    terminal_generation_failure_is_preserved();
    generated_redraw_batching();
    application_creation();
    current_document_repair();
    runtime_repair_requires_current_source_and_never_replays();
    runtime_repair_rejects_unchanged_source();
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
// FUNC: runtime_repair_requires_current_source_and_never_replays
// DESC: Exercises shared retry limits, stale-source rejection, atomic preservation and validation-only authority after real runtime feedback.
// ------------------=
fn runtime_repair_requires_current_source_and_never_replays() {
    use ai::{Panel, GenerationStatus as S};
    let request=b"Create a Python application and launch it";
    let source=b"print(int('oops'))\n";
    let mut panel=Panel::new();
    panel.request[..request.len()].copy_from_slice(request);panel.request_len=request.len();
    panel.generation=S::Complete;panel.document_revision=17;panel.workflow_goal=Some(42);
    panel.source_language=ai::editor_compile_intent::SourceLanguage::Python;
    assert!(panel.permits_generated_launch());
    let before=panel;
    assert!(!panel.queue_runtime_repair(request,17,17,b"ValueError"));assert!(panel==before);
    panel.source_binding=Some(([3;16],5));let before=panel;
    assert!(!panel.queue_runtime_repair(request,17,18,b"ValueError"));assert!(panel==before);
    assert!(!panel.queue_runtime_repair(b"other request",17,17,b"ValueError"));assert!(panel==before);
    assert!(!panel.queue_runtime_repair(request,17,17,b""));assert!(panel==before);
    assert!(panel.queue_runtime_repair(request,17,17,b"ValueError: invalid literal at line 1"));
    assert_eq!(panel.workflow_goal,None);assert_eq!(panel.repair_attempts,1);
    assert_eq!(panel.source_binding,Some(([3;16],5)));assert!(panel.runtime_repair);
    assert!(!panel.permits_generated_launch());assert!(panel.preserves_existing_source());
    let mut document=TextDocument::new();assert!(document.open(source));
    assert!(!panel.apply_existing_source(&mut document,17,None));assert_eq!(document.bytes(),source);
    panel.generation=S::Complete;
    assert!(panel.queue_compiler_repair(request,17,17,b"SyntaxError"));
    assert!(!panel.permits_generated_launch());assert_eq!(panel.repair_attempts,2);
    panel.generation=S::Complete;let exhausted=panel;
    assert!(!panel.queue_runtime_repair(request,17,17,b"TypeError"));assert!(panel==exhausted);
    panel.cancel_delivery();assert!(!panel.queue_runtime_repair(request,17,17,b"ValueError"));
    assert_eq!(document.bytes(),source);
}

// ------------------------=
// FUNC: runtime_repair_rejects_unchanged_source
// DESC: Checks real editor no-op rejection, original-diagnostic retention, remaining retry bounds, cancellation and successful atomic replacement without relaunch authority.
// ------------------=
fn runtime_repair_rejects_unchanged_source() {
    use ai::{Panel, GenerationStatus as S};
    // ------------------------=
    // FUNC: completed
    // DESC: Supplies one completed source proposal without executing or altering candidate bytes.
    // ------------------=
    fn completed(panel: &mut Panel, source: &[u8]) {
        panel.argument[..source.len()].copy_from_slice(source);
        panel.argument_len = source.len();
        panel.pending = Action::ReplaceCompile;
        panel.generation = S::Complete;
    }
    for (request, original, changed) in [
        (b"Create a Python application".as_slice(),
            b"print(int('bad'))\n".as_slice(), b"print(4)\n".as_slice()),
        (b"Create a C application".as_slice(),
            b"int main(void) { return *(int *)0; }\n".as_slice(), b"int main(void) { return 4; }\n".as_slice()),
    ] {
        let diagnostics = [7u8,31,85,112,4];
        let mut base = Panel::new();
        base.request[..request.len()].copy_from_slice(request); base.request_len = request.len();
        base.generation = S::Complete; base.document_revision = 11;
        base.source_binding = Some(([7;16],3)); base.workflow_goal = Some(23);
        assert!(base.queue_runtime_repair(request,11,11,&diagnostics));
        assert_eq!(base.repair_attempts,1); assert!(!base.runtime_repair_noop);
        for candidate in [original.to_vec(), original.trim_ascii_end().to_vec(),
            [original.trim_ascii_end(), b"\r\n \t\n"].concat()] {
            let mut panel = base;
            let mut document = TextDocument::new(); assert!(document.open(original));
            document.set_cursor(3);
            let cursor = document.cursor(); let selection = document.selection();
            completed(&mut panel,&candidate);
            let stale = panel;
            assert!(!panel.apply_existing_source(&mut document,12,None));
            assert!(panel==stale);
            assert!(!panel.apply_existing_source(&mut document,11,None));
            assert_eq!(panel.generation,S::Queued);
            assert_eq!(panel.repair_attempts,2);
            assert!(panel.runtime_repair_noop && panel.runtime_repair);
            assert_eq!(panel.source_binding,Some(([7;16],3)));
            assert_eq!(panel.workflow_goal,None);
            assert_eq!(&panel.repair_diagnostics[..panel.repair_diagnostics_len],&diagnostics);
            assert_eq!(panel.argument_len,0); assert_eq!(panel.pending,Action::None);
            assert!(!panel.apply_generated_compile_requested() && !panel.permits_generated_launch());
            assert_eq!(document.bytes(),original);
            assert_eq!(document.cursor(),cursor); assert_eq!(document.selection(),selection);
            assert!(document.is_saved()); assert!(!document.undo());
            let queued = panel;
            assert!(!panel.apply_existing_source(&mut document,11,None)); assert!(panel==queued);
            assert!(!panel.queue_compiler_repair(request,11,11,&diagnostics));
            let mut prompt = [0u8;16384];
            assert!(panel.compiler_repair_prompt(document.bytes(),&mut prompt).is_some());

            let mut cancelled = panel;
            cancelled.cancel_delivery();
            assert!(!cancelled.stream_model_output(2,b"REPLACE_COMPILE\nprint(4)\nEND_ACTION").progressed);
            assert!(!cancelled.apply_existing_source(&mut document,11,None));
            assert_eq!(cancelled.generation,S::Cancelled);
            assert_eq!(cancelled.repair_attempts,2);
            assert_eq!(document.bytes(),original);

            completed(&mut panel,&candidate);
            assert!(!panel.apply_existing_source(&mut document,11,None));
            assert_eq!(panel.generation,S::Failed);
            assert_eq!(panel.repair_attempts,2);
            assert_eq!(panel.pending,Action::None); assert_eq!(panel.argument_len,0);
            assert_eq!(&panel.repair_diagnostics[..panel.repair_diagnostics_len],&diagnostics);
            assert!(!panel.apply_generated_compile_requested() && !panel.permits_generated_launch());
            assert_eq!(document.bytes(),original); assert!(document.is_saved());
            let exhausted = panel;
            assert!(!panel.apply_existing_source(&mut document,11,None)); assert!(panel==exhausted);
            assert!(!panel.queue_runtime_repair(request,11,11,&diagnostics));

            let mut corrected = queued;
            completed(&mut corrected,changed);
            assert!(corrected.apply_existing_source(&mut document,11,None));
            assert_eq!(document.bytes(),changed); assert_eq!(corrected.repair_attempts,2);
            assert!(!corrected.permits_generated_launch());
            assert!(document.undo()); assert_eq!(document.bytes(),original);
        }
        let mut unbound = base; unbound.source_binding = None;
        let mut document = TextDocument::new(); assert!(document.open(original));
        completed(&mut unbound,original);
        assert!(!unbound.apply_existing_source(&mut document,11,None));
        assert_eq!(unbound.generation,S::Failed); assert_eq!(unbound.repair_attempts,1);
        assert_eq!(document.bytes(),original);
    }
}

// ------------------------=
// FUNC: continued_source_transactions
// DESC: Drives real panel parsing and bounded editor delivery across arbitrary mid-token, indentation and literal continuations without duplicate bytes or premature compile admission.
// ------------------=
fn continued_source_transactions() {
    use ai::GenerationStatus as S;
    // ------------------------=
    // FUNC: panel
    // DESC: Creates one explicitly authorized Python generation proposal with stable request and source identities.
    // ------------------=
    fn panel() -> ai::Panel {
        let mut panel = ai::Panel::new(); let request = b"Create a chess game in Python";
        panel.request[..request.len()].copy_from_slice(request); panel.request_len = request.len();
        panel.generation = S::Running; panel.workflow_goal = Some(19); panel.document_revision = 7;
        panel
    }
    // ------------------------=
    // FUNC: drain
    // DESC: Applies only production-selected contiguous preview ranges, asserting the same delivery cap used by the installed editor.
    // ------------------=
    fn drain(panel: &mut ai::Panel, document: &mut TextDocument) {
        while let Some(range) = panel.stream.ready(&panel.argument[..panel.argument_len], None, true) {
            assert!(range.len() <= ai::generation_stream::MAX_FLUSH_BYTES);
            assert!(document.append_generation_preview(&panel.argument[range.clone()], range.start == 0));
            assert!(panel.stream.committed(range.end, None));
        }
    }
    let source = b"import math\ndef calculate(value):\n    text = '''literal\nEND_ACTION\ninside data'''\n    return value + 1\nprint(calculate(4))";
    for cut in 1..source.len() {
        let mut panel = panel(); let mut document = TextDocument::new();
        let first = [b"REPLACE_COMPILE\n".as_slice(), &source[..cut]].concat();
        panel.stream_model_output(2, &first); drain(&mut panel, &mut document);
        assert!(!panel.accept_for_owner(2, &first, true));
        assert_eq!(panel.pending, Action::None); assert!(!panel.apply_generated_compile_requested());
        assert!(panel.prepare_source_continuation(&first), "cut {cut}");
        assert_eq!(panel.continuation_source(), &source[..cut]);
        assert_eq!(panel.continuation_count(), 1); assert_eq!(panel.workflow_goal, Some(19));
        assert_eq!(panel.document_revision, 7);
        let mut prompt = [0; 16384];
        assert!(panel.source_continuation_prompt(&mut prompt).is_some());
        assert!(panel.source_continuation_prompt(&mut [0; 8]).is_none());
        drain(&mut panel, &mut document);
        let second = [b"CONTINUE_SOURCE\n".as_slice(), &source[cut..], b"\nEND_ACTION"].concat();
        for end in 1..=second.len() {
            panel.stream_model_output(2, &second[..end]); drain(&mut panel, &mut document);
            assert_eq!(panel.generation, S::Running, "cut {cut}, end {end}");
            assert_eq!(panel.pending, Action::None);
        }
        assert_eq!(panel.completed_generation_output_length(&second), Some(second.len()));
        assert!(panel.accept_for_owner(2, &second, true)); panel.generation = S::Complete;
        drain(&mut panel, &mut document);
        assert_eq!(document.bytes(), source); assert!(panel.apply_generated_compile_requested());
    }
    let mut bounded = panel();
    assert!(bounded.prepare_source_continuation(b"REPLACE_COMPILE\nvalue = "));
    assert!(bounded.prepare_source_continuation(b"CONTINUE_SOURCE\n1\nvalue += "));
    assert!(bounded.prepare_source_continuation(b"CONTINUE_SOURCE\n2\nprint("));
    assert_eq!(bounded.continuation_source(), b"value = 1\nvalue += 2\nprint(");
    assert!(!bounded.prepare_source_continuation(b"CONTINUE_SOURCE\nvalue"));
    assert_eq!(bounded.generation, S::Failed); assert_eq!(bounded.pending, Action::None);
    let mut stalled = panel(); assert!(stalled.prepare_source_continuation(b"REPLACE_COMPILE\nvalue = "));
    assert!(!stalled.prepare_source_continuation(b"CONTINUE_SOURCE\n")); assert_eq!(stalled.generation, S::Failed);
    let mut stopped = panel(); assert!(stopped.prepare_source_continuation(b"REPLACE_COMPILE\nvalue = "));
    stopped.cancel_delivery(); assert!(stopped.continuation_source().is_empty());
    assert!(!stopped.prepare_source_continuation(b"CONTINUE_SOURCE\n1"));
    stopped.reset_delivery(); assert_eq!(stopped.continuation_count(), 0); assert_eq!(stopped.argument_len, 0);
    assert!(!stopped.stream_model_output(2, b"CONTINUE_SOURCE\n1\nEND_ACTION").progressed);
    assert_eq!(stopped.argument_len, 0); assert_eq!(stopped.pending, Action::None);
    for invalid in [b"REPLACE_COMPILE\nprint(1)".as_slice(), b"```python\nprint(1)", b"CONTINUE_SOURCE\nREPLACE_COMPILE\nprint(1)"] {
        let mut bad = panel(); assert!(bad.prepare_source_continuation(b"REPLACE_COMPILE\nvalue = 1\n"));
        assert!(!bad.accept_for_owner(2, invalid, true)); assert_eq!(bad.pending, Action::None);
    }
    let mut repeated = panel();
    let prefix = b"import math\nvalue = math.sqrt(16)\nprint(value)\n";
    assert!(repeated.prepare_source_continuation(&[b"REPLACE_COMPILE\n".as_slice(), prefix].concat()));
    let retained=repeated.argument[..repeated.argument_len].to_vec();
    for end in 0..prefix.len() {
        assert!(!repeated.stream_model_output(2, &[b"CONTINUE_SOURCE\n".as_slice(), &prefix[..end]].concat()).progressed);
        assert_eq!(repeated.generation,S::Running);
        assert_eq!(&repeated.argument[..repeated.argument_len],retained.as_slice());
    }
    repeated.stream_model_output(2, &[b"CONTINUE_SOURCE\n".as_slice(), prefix].concat());
    assert_eq!(repeated.generation, S::Failed);
    for invalid in ["REPLACE_COMPILE\nprint('♟')".as_bytes().to_vec(),
        [b"REPLACE_COMPILE\n".as_slice(), &vec![b'x'; ai::GENERATED_SOURCE_CAPACITY + 1]].concat()] {
        let mut bad = panel(); bad.stream_model_output(2, &invalid);
        assert_eq!(bad.generation, S::Failed); assert_eq!(bad.pending, Action::None); assert!(bad.response_len > 0);
    }
    // Existing source must remain untouched while a continuation is incomplete,
    // then apply exactly one revision-fenced and undoable transaction.
    let original = b"value = 1\nprint(value)"; let mut document = TextDocument::new(); assert!(document.open(original));
    let mut update = ai::Panel::new();
    assert!(update.queue_user_update(b"update current file to add one", 7, original,
        ai::editor_compile_intent::SourceLanguage::Python, 19)); update.generation = S::Running;
    assert!(update.prepare_source_continuation(b"REPLACE_COMPILE\nvalue = 1\nvalue += "));
    assert!(!update.apply_existing_source(&mut document, 7, None)); assert_eq!(document.bytes(), original);
    assert!(update.accept_for_owner(2, b"CONTINUE_SOURCE\n1\nprint(value)\nEND_ACTION", true)); update.generation = S::Complete;
    assert!(!update.apply_existing_source(&mut document, 8, None)); assert_eq!(document.bytes(), original);
    assert!(update.apply_existing_source(&mut document, 7, None)); assert_eq!(document.bytes(), b"value = 1\nvalue += 1\nprint(value)");
    assert!(document.undo()); assert_eq!(document.bytes(), original);
}

// ------------------------=
// FUNC: surgical_source_transactions
// DESC: Exercises completed editor updates, stale/cancelled rejection, stable untouched bytes/caret, atomic undo and oversized rollback.
// ------------------=
fn surgical_source_transactions() {
    let original = b"# keep this comment\ndef value():\n    return 1\n\ndef untouched():\n    return 42\n";
    let changed = b"# keep this comment\ndef value():\n    return 12345\n\ndef untouched():\n    return 42\n";
    let mut document = TextDocument::new();
    assert!(document.open(original));
    document.set_cursor(original.len() - 3);
    let mut panel = ai::Panel::new();
    assert!(panel.queue_user_update(b"update current file to return a larger value", 7, original,
        ai::editor_compile_intent::SourceLanguage::Python, 3));
    panel.argument[..changed.len()].copy_from_slice(changed);
    panel.argument_len = changed.len();
    panel.pending = Action::ReplaceCompile;
    for state in [ai::GenerationStatus::Queued, ai::GenerationStatus::Running,
        ai::GenerationStatus::Cancelled, ai::GenerationStatus::Failed] {
        panel.generation = state;
        assert!(!panel.apply_existing_source(&mut document, 7, Some(1)));
        assert_eq!(document.bytes(), original);
        assert!(document.is_saved());
    }
    panel.generation = ai::GenerationStatus::Complete;
    assert!(!panel.apply_existing_source(&mut document, 8, Some(1)));
    assert_eq!(document.bytes(), original);
    assert!(panel.apply_existing_source(&mut document, 7, Some(1)));
    assert_eq!(document.bytes(), changed);
    assert_eq!(document.cursor(), changed.len() - 3);
    assert_eq!(panel.stream.delivered, changed.len());
    assert_eq!(panel.stream.flushes, 1);
    assert!(!panel.apply_existing_source(&mut document, 7, Some(2)));
    assert!(document.undo());
    assert_eq!(document.bytes(), original);
    assert_eq!(document.cursor(), original.len() - 3);
    assert!(document.is_saved());
    assert!(!document.undo());
    assert!(document.redo());
    assert_eq!(document.bytes(), changed);
    assert!(!document.apply_source_update(&[b'x'; DOCUMENT_CAPACITY + 1]));
    assert!(!document.apply_source_update(b"bad\0source"));
    assert_eq!(document.bytes(), changed);
    assert!(document.apply_source_update(changed));
    assert!(document.undo());
    assert_eq!(document.bytes(), original);

    // A current-target update uses the live buffer while separately retaining
    // its exact saved base. Undo history eviction must not erase that base.
    assert!(document.saved_source_matches(original));
    assert!(!document.saved_source_matches(changed));
    document.set_cursor(0);
    assert!(document.replace_selection(b"# local unsaved work\n"));
    let dirty = document.bytes().to_vec();
    assert!(!document.is_saved());
    assert!(document.saved_source_matches(original));
    assert!(!document.saved_source_matches(&dirty));
    let mut dirty_changed = b"# local unsaved work\n".to_vec();
    dirty_changed.extend_from_slice(changed);
    assert!(panel.queue_user_update(b"update current file to return a larger value", 9, &dirty,
        ai::editor_compile_intent::SourceLanguage::Python, 4));
    panel.argument[..dirty_changed.len()].copy_from_slice(&dirty_changed);
    panel.argument_len = dirty_changed.len();
    panel.pending = Action::ReplaceCompile;
    panel.generation = ai::GenerationStatus::Complete;
    assert!(!panel.apply_existing_source(&mut document, 10, Some(3)));
    assert_eq!(document.bytes(), dirty);
    assert!(panel.apply_existing_source(&mut document, 9, Some(3)));
    assert_eq!(document.bytes(), dirty_changed);
    assert!(document.saved_source_matches(original));
    assert!(document.undo());
    assert_eq!(document.bytes(), dirty);
    assert!(!document.is_saved());
    assert!(document.redo());
    document.save();
    assert!(document.saved_source_matches(&dirty_changed));
    assert!(!document.saved_source_matches(original));
    assert!(document.undo());
    assert!(document.saved_source_matches(&dirty_changed));
    document.clear();
    assert!(document.saved_source_matches(b""));
    assert!(!document.saved_source_matches(original));

    // Full-length replacements must finish in one transaction, not remain
    // pending behind the 256-byte incremental-generation flush limit.
    assert!(document.open(&[b'a'; 1024]));
    panel.document_revision = 7;
    panel.stream = ai::generation_stream::Stream::new();
    panel.argument[..1024].fill(b'b'); panel.argument_len = 1024;
    assert!(panel.apply_existing_source(&mut document, 7, Some(3)));
    assert_eq!(document.bytes(), &[b'b'; 1024]);
    assert_eq!(panel.stream.delivered, 1024);
    assert!(document.undo());
    assert_eq!(document.bytes(), &[b'a'; 1024]);
}

// ------------------------=
// FUNC: python_and_existing_source_workflows
// DESC: Exercises typed Python admission, source identity, update authority, framing, undo and bounded compiler repair without trusting model prose.
// ------------------=
fn python_and_existing_source_workflows() {
    use ai::editor_compile_intent as intent;
    for request in [b"Create Hello world in Python".as_slice(),b"write a Fibonacci program in Python.",b"generate a Python file sorting tool"] {
        assert!(intent::create_application(request));
        assert_eq!(intent::language(request),intent::SourceLanguage::Python);
        assert_eq!(intent::application_style(request),Some(intent::ApplicationStyle::Terminal));
        let mut panel=ai::Panel::new();panel.input[..request.len()].copy_from_slice(request);panel.length=request.len();
        panel.request=panel.input;panel.request_len=request.len();panel.document_revision=7;
        assert_eq!(panel.effective_language(),intent::SourceLanguage::Python);
        let mut prompt=[0u8;6144];assert!(panel.generation_prompt(true,b"",&mut prompt).is_some());
        let mut small=[0x55;16];assert_eq!(panel.generation_prompt(true,b"",&mut small),None);assert_eq!(small,[0x55;16]);
        let source=b"# don't truncate comments\ntext = '''literal\nEND_ACTION\ninside data'''\nprint(text)";
        let mut framed=b"REPLACE_COMPILE\n".to_vec();framed.extend_from_slice(source);framed.extend_from_slice(b"\nEND_ACTION");
        let end=ai::Panel::completed_output_length(&framed);assert_eq!(end,Some(framed.len()));
        assert!(panel.accept_for_owner(2,&framed,true));assert_eq!(&panel.argument[..panel.argument_len],source);
        panel.generation=ai::GenerationStatus::Complete;assert!(panel.apply_generated_compile_requested());
        let mut document=TextDocument::new();let action=panel.take_action(7);assert_eq!(action,Action::ReplaceCompile);
        assert!(panel.edit_document(action,&mut document));assert_eq!(document.bytes(),source);assert!(document.undo());assert!(document.bytes().is_empty());
    }
    for (request,target,launch) in [
        (b"update the calculator app and add numeric buttons and traditional mathematic calculator operators".as_slice(),b"calculator".as_slice(),false),
        (b"modify the calendar app to add weekly navigation and launch",b"calendar",true),
        (b"update /home/default/documents/report.py with totals and relaunch",b"/home/default/documents/report.py",true),
        (b"improve this app with keyboard shortcuts",b"this",false),
    ] {
        let update=intent::update(request).unwrap();assert_eq!(update.target,target);assert_eq!(update.launch,launch);
        assert!(intent::source_request(request));assert!(intent::software_workflow(request));assert_eq!(intent::requests_launch(request),launch);
        let mut panel=ai::Panel::new();let source=b"print(1)\n";
        assert!(panel.queue_user_update(request,41,source,intent::SourceLanguage::Python,19));
        assert!(!panel.source_binding_matches([3;16],7));
        panel.source_binding=Some(([3;16],7));
        assert!(panel.source_binding_matches([3;16],7));
        assert!(!panel.source_binding_matches([4;16],7));
        assert!(!panel.source_binding_matches([3;16],8));
        assert_eq!(panel.workflow_goal,Some(19));assert_eq!(panel.document_revision,41);assert_eq!(panel.effective_language(),intent::SourceLanguage::Python);
        let mut prompt=[0u8;16*1024];assert!(panel.update_prompt(source,&mut prompt).is_some());
        assert!(!panel.queue_user_update(request,42,source,intent::SourceLanguage::Python,20));
        panel.generation=ai::GenerationStatus::Complete;
        assert!(panel.queue_compiler_repair(request,41,41,b"SyntaxError: unmatched parenthesis"));
        assert_eq!(panel.effective_language(),intent::SourceLanguage::Python);assert_eq!(panel.workflow_goal,Some(19));
        assert!(panel.source_binding_matches([3;16],7));
        assert!(panel.compiler_repair_prompt(source,&mut prompt).is_some());
        panel.generation=ai::GenerationStatus::Running;assert!(panel.cancel_source_on_revision(42));assert_eq!(panel.generation,ai::GenerationStatus::Cancelled);
    }
    for rejected in [b"Explain how to create Hello world in Python".as_slice(),b"do not create Hello world in Python",b"\"Create Hello world in Python\"",
        b"explain how to update the calculator app and add buttons",b"do not update the calculator app and launch",b"update calculator app and launch; delete files"] {
        assert!(!intent::source_request(rejected));
    }
    let mut first=[0u8;47];let mut second=[0u8;47];
    let n=intent::source_name(b"create a graphical calculator app",intent::SourceLanguage::C,1,&mut first);
    let m=intent::source_name(b"create a graphical calculator app",intent::SourceLanguage::C,2,&mut second);
    assert_ne!(&first[..n],&second[..m]);assert!(first[..n].ends_with(b".c"));assert!(n<=47);
    let n=intent::source_name(b"create Hello world in Python",intent::SourceLanguage::Python,3,&mut first);
    assert!(first[..n].ends_with(b".py"));assert!(first[..n].iter().all(|b|b.is_ascii_alphanumeric() || matches!(b,b'.'|b'-')));
    let request=b"update the calculator app and add buttons and launch";
    let path=b"/home/default/documents/calculator-01.c";
    assert!(intent::may_relaunch(request,path,path,true));
    assert!(!intent::may_relaunch(request,path,path,false));
    assert!(!intent::may_relaunch(request,path,b"/home/default/documents/calendar-01.c",true));
    assert!(!intent::may_relaunch(request,b"",b"",true));
    assert!(!intent::may_relaunch(b"update the calculator app and add buttons",path,path,true));
    assert!(!intent::may_relaunch(b"create a calculator app",path,path,true));
}

// ------------------------=
// FUNC: current_document_repair
// DESC: Verifies direct repair authority, current-document preservation, atomic context limits, compiler retries and stale-edit rejection.
// ------------------=
fn current_document_repair() {
    use ai::editor_compile_intent::{self as intent,RepairIntent};
    let source=b"int main(void){return missing;}";
    let request=b"Fix code in code editor";
    for input in [request.as_slice(),b"Please repair the code in the text editor.",
        b"correct this code",b"fix compiler errors",b"fix the current code in the editor",
        b"fix   my code"] {
        assert_eq!(intent::repair(input),Some(RepairIntent::CurrentDocument));
        assert!(intent::source_request(input));assert!(!intent::create_application(input));
        assert!(!intent::requests_opponent(input));
    }
    for input in [b"Do not fix code".as_slice(),b"explain how to fix code",b"\"fix code\"",
        b"fix code then delete files",b"fix code without compiling",b"fix code and launch",
        b"fix compiler errors and change permissions",b"fix",b"fix the calendar"] {
        assert_eq!(intent::repair(input),None);assert!(!intent::source_request(input));
    }
    let mut panel=ai::Panel::new();
    let empty=panel;
    assert!(!panel.queue_user_repair(request,8,b"",b""));assert!(panel==empty);
    assert!(!panel.queue_user_repair(b"explain how to fix code",8,source,b""));assert!(panel==empty);
    assert!(panel.queue_user_repair(request,8,source,b""));
    assert_eq!(panel.generation,ai::GenerationStatus::Queued);
    assert_eq!(panel.document_revision,8);assert_eq!(panel.workflow_goal,None);
    assert_eq!(panel.repair_attempts,0);assert_eq!(panel.argument_len,0);
    let queued=panel;
    assert!(!panel.queue_user_repair(request,9,source,b"diagnostic"));assert!(panel==queued);
    let mut small=[0x5a;16];
    assert_eq!(panel.compiler_repair_prompt(source,&mut small),None);assert_eq!(small,[0x5a;16]);
    assert_eq!(panel.compiler_repair_prompt(b"",&mut small),None);
    let mut prompt=[0;16384];assert!(panel.compiler_repair_prompt(source,&mut prompt).is_some());
    panel.generation=ai::GenerationStatus::Running;
    assert!(panel.accept_for_owner(2,b"REPLACE_COMPILE\nint main(void){return 0;}\nEND_ACTION",true));
    assert!(!panel.apply_generated_compile_requested());
    panel.generation=ai::GenerationStatus::Complete;
    assert!(panel.apply_generated_compile_requested());
    let mut edited=panel;assert!(edited.cancel_source_on_revision(9));
    assert_eq!(edited.generation,ai::GenerationStatus::Cancelled);assert_eq!(edited.take_action(9),Action::None);
    assert_eq!(panel.take_action(8),Action::ReplaceCompile);
    let mut document=TextDocument::new();assert!(document.open(source));
    assert!(document.replace_generation_preview(&panel.argument[..panel.argument_len],true));
    assert_eq!(document.bytes(),b"int main(void){return 0;}");
    assert!(document.undo());assert_eq!(document.bytes(),source);
    for attempt in 1..=2 {
        assert!(panel.queue_compiler_repair(request,8,8,b"error: unresolved symbol"));
        assert_eq!(panel.repair_attempts,attempt);assert_eq!(panel.generation,ai::GenerationStatus::Queued);
        assert!(panel.compiler_repair_prompt(source,&mut prompt).is_some());
        panel.generation=ai::GenerationStatus::Complete;
    }
    let final_state=panel;
    assert!(!panel.queue_compiler_repair(request,8,8,b"error: unresolved symbol"));assert!(panel==final_state);
    assert!(!panel.queue_compiler_repair(request,8,9,b"error: unresolved symbol"));assert!(panel==final_state);
}

// ------------------------=
// FUNC: rejected_creation_retains_terminal_state
// DESC: Verifies a pre-dispatch rejection cannot become stale-goal cancellation or mutate existing source and admitted work.
// ------------------=
fn rejected_creation_retains_terminal_state() {
    use ai::{GenerationStatus as S,Panel};
    let mut document=TextDocument::new();
    assert!(document.append_generation_preview(b"int partial",true));
    assert!(document.insert(b' '));
    let original=document.bytes().to_vec();
    let mut panel=Panel::new();
    panel.workflow_goal=Some(9);
    panel.argument[..11].copy_from_slice(b"int partial");panel.argument_len=11;
    panel.pending=Action::ReplaceCompile;panel.awaiting_compile_save=true;
    assert!(panel.reject_unstarted_generation());
    assert_eq!(panel.generation,S::Failed);
    assert_eq!(panel.workflow_goal,None);
    assert_eq!(panel.pending,Action::None);
    assert!(!panel.awaiting_compile_save);
    assert!(!panel.cancel_workflow(9));
    assert_eq!(panel.generation,S::Failed);
    assert!(!panel.apply_generated_compile_requested());
    assert_eq!(&panel.argument[..panel.argument_len],b"int partial");
    assert_eq!(document.bytes(),original);
    assert!(!document.is_saved());
    for state in [S::Queued,S::Running,S::Complete,S::Failed,S::Cancelled] {
        let mut admitted=panel;admitted.workflow_goal=Some(10);admitted.generation=state;
        let before=admitted;
        assert!(!admitted.reject_unstarted_generation());
        assert!(admitted==before);
    }
}

// ------------------------=
// FUNC: generated_repairs_preserve_source_identity
// DESC: Exercises the production save-target policy across C/Python creation, two automatic repairs, named updates and manual unsaved repair.
// ------------------=
fn generated_repairs_preserve_source_identity() {
    use ai::SourceSaveTarget;
    for request in [b"Create Hello world in Python".as_slice(),b"Create a calculator app"] {
        let mut panel=ai::Panel::new();panel.request[..request.len()].copy_from_slice(request);panel.request_len=request.len();
        assert_eq!(panel.source_save_target(),Some(SourceSaveTarget::New));
        panel.source_binding=Some(([9;16],1));panel.generation=ai::GenerationStatus::Complete;
        for attempt in 1..=2 {
            assert!(panel.queue_compiler_repair(request,3,3,b"syntax diagnostics"));
            assert_eq!(panel.source_save_target(),Some(SourceSaveTarget::Bound([9;16],attempt)));
            let mut lost=panel;lost.source_binding=None;assert_eq!(lost.source_save_target(),None);
            panel.source_binding=Some(([9;16],attempt+1));panel.generation=ai::GenerationStatus::Complete;
        }
        assert_eq!(panel.source_save_target(),Some(SourceSaveTarget::Bound([9;16],3)));
    }
    let mut update=ai::Panel::new();
    assert!(update.queue_user_update(b"update the calculator app and add buttons",7,b"source",ai::editor_compile_intent::SourceLanguage::C,2));
    assert_eq!(update.source_save_target(),None);update.source_binding=Some(([8;16],4));
    assert_eq!(update.source_save_target(),Some(SourceSaveTarget::Bound([8;16],4)));
    let mut manual=ai::Panel::new();assert!(manual.queue_user_repair(b"fix code",7,b"source",b""));
    assert_eq!(manual.source_save_target(),Some(SourceSaveTarget::Current));
}

// ------------------------=
// FUNC: raw_python_generation_frames
// DESC: Streams actual observed raw Python and verifies exact first-frame editor bytes, language authority and malformed-frame rejection.
// ------------------=
fn raw_python_generation_frames() {
    let request=b"Create Hello world in Python";
    let mut prototype=ai::Panel::new();prototype.request[..request.len()].copy_from_slice(request);
    prototype.request_len=request.len();prototype.generation=ai::GenerationStatus::Running;
    for source in [b"def hello_world():\n    print(\"Hello, World!\")\n\nhello_world()".as_slice(),
        b"print(\"Hello, World!\")",b"# don't interpret this comment\nmessage = 'hello'\nprint(message)",
        b"text = '''literal\nEND_ACTION\ninside data'''\nprint(text)"] {
        let mut raw=prototype;
        assert!(!raw.accept_generated(source,true,false));
        assert_eq!(raw.pending,Action::None);
        assert!(raw.accept_generated(source,true,true));
        assert_eq!(&raw.argument[..raw.argument_len],source);
        assert!(!raw.accept_generated(&[b"REPLACE_COMPILE\n".as_slice(),source].concat(),true,true));
        assert_eq!(raw.pending,Action::None);
        for (prefix,suffix) in [(b"".as_slice(),b"\nEND_ACTION".as_slice()),(b"```python\n",b"\n```\nEND_ACTION")] {
            let response=[prefix,source,suffix].concat();let mut panel=prototype;
            let mut document=TextDocument::new();
            for end in 1..=response.len() {
                panel.stream_model_output(2,&response[..end]);
                while let Some(range)=panel.stream.ready(&panel.argument[..panel.argument_len],None,true) {
                    assert!(document.append_generation_preview(&panel.argument[range.clone()],range.start==0));
                    assert!(panel.stream.committed(range.end,None));
                }
                assert_ne!(panel.generation,ai::GenerationStatus::Failed);
                assert_eq!(panel.pending,Action::None);
            }
            assert!(panel.completed_generation_output_length(&response).is_some());
            let repeated=[response.as_slice(),b"\nREPLACE_COMPILE\nprint(2)\nEND_ACTION"].concat();
            assert!(panel.accept_generated(&repeated,true,true));assert_eq!(&panel.argument[..panel.argument_len],source);
            assert_eq!(document.bytes(),source);assert_eq!(panel.pending,Action::ReplaceCompile);
        }
    }
    for rejected in [b"Here is your Python program:\nprint(1)\nEND_ACTION".as_slice(),
        b"I cannot create this program.\nEND_ACTION",b"def task():\n pass\nREPLACE_COMPILE\nprint(2)"] {
        let mut panel=prototype;assert!(!panel.accept_generated(rejected,true,true));assert_eq!(panel.pending,Action::None);
    }
    for request in [b"Create a calculator app".as_slice(),b"Explain Python"] {
        let mut panel=ai::Panel::new();panel.request[..request.len()].copy_from_slice(request);panel.request_len=request.len();
        assert!(!panel.stage_generated_source(b"print(1)\nEND_ACTION"));
        assert!(!panel.accept_generated(b"print(1)\nEND_ACTION",true,true));
        assert_eq!(panel.pending,Action::None);
    }
}

// ------------------------=
// FUNC: prefaced_source_generation_frames
// DESC: Verifies bounded introductory prose stays outside exact source bytes while language authority, single-frame admission, cancellation and atomic repairs remain enforced.
// ------------------=
fn prefaced_source_generation_frames() {
    use ai::GenerationStatus as S;
    // ------------------------=
    // FUNC: panel
    // DESC: Creates an authorized generation fixture using the production request language.
    // ------------------=
    fn panel(python: bool) -> ai::Panel {
        let request = if python { b"Create Hello world in Python".as_slice() }
            else { b"Create a simple calculator app".as_slice() };
        let mut panel = ai::Panel::new();
        panel.request[..request.len()].copy_from_slice(request);
        panel.request_len = request.len();
        panel.generation = S::Running;
        panel
    }
    // ------------------------=
    // FUNC: drain
    // DESC: Delivers only production-selected contiguous source ranges into a real editor document.
    // ------------------=
    fn drain(panel: &mut ai::Panel, document: &mut TextDocument) {
        while let Some(range) = panel.stream.ready(&panel.argument[..panel.argument_len], None, true) {
            assert!(range.len() <= ai::generation_stream::MAX_FLUSH_BYTES);
            assert!(document.append_generation_preview(&panel.argument[range.clone()], range.start == 0));
            assert!(panel.stream.committed(range.end, None));
        }
    }
    for (python, fence, wrong_fence, source) in [
        (false, b"```c\n".as_slice(), b"```python\n".as_slice(),
            b"#include <stdio.h>\n/*\n```\nEND_ACTION\nREPLACE_COMPILE\n*/\nint main(void) { puts(\"END_ACTION\"); return 0; }".as_slice()),
        (true, b"```python\n".as_slice(), b"```c\n".as_slice(),
            b"value = '''data\n```\nEND_ACTION\nREPLACE_COMPILE\n'''\nprint(value)".as_slice()),
    ] {
        let intro = b"Here is the updated program:\n\n";
        let opening = [intro.as_slice(), fence].concat();
        let response = [opening.as_slice(), source, b"\n```\nEND_ACTION"].concat();
        assert!(ai::source_frame::inspect(&response).is_none());
        assert!(ai::source_frame::inspect_python(&response).is_none());
        let final_frame = ai::source_frame::inspect_generated(&response, python).unwrap();
        assert_eq!(final_frame.source, source);
        assert!(final_frame.framed && !final_frame.malformed && !final_frame.continuable);
        assert!(final_frame.completed_end.is_some());
        let mut generated = panel(python);
        let mut document = TextDocument::new();
        for end in 1..=response.len() {
            generated.stream_model_output(2, &response[..end]);
            drain(&mut generated, &mut document);
            assert_ne!(generated.generation, S::Failed);
            assert_eq!(generated.pending, Action::None);
            assert!(source.starts_with(document.bytes()));
            if end <= opening.len() { assert!(document.bytes().is_empty()); }
        }
        assert_eq!(document.bytes(), source);
        assert!(generated.accept_generated(&response, true, true));
        assert_eq!(&generated.argument[..generated.argument_len], source);
        assert_eq!(generated.pending, Action::ReplaceCompile);

        for preamble in [vec![b'x'; 511], b"intro\n".repeat(7)] {
            let valid = [preamble.as_slice(), b"\n", fence, source, b"\n```"].concat();
            let frame = ai::source_frame::inspect_generated(&valid, python).unwrap();
            assert_eq!(frame.source, source);
            assert!(frame.completed_end.is_some() && !frame.malformed);
        }
        let too_long = [vec![b'x'; 512], b"\n".to_vec(), fence.to_vec(),
            source.to_vec(), b"\n```".to_vec()].concat();
        let too_many_lines = [b"intro\n".repeat(9), fence.to_vec(), source.to_vec(), b"\n```".to_vec()].concat();
        for invalid in [
            [intro.as_slice(), source, b"\nEND_ACTION"].concat(),
            [intro.as_slice(), wrong_fence, source, b"\n```"].concat(),
            [intro.as_slice(), b"```\n", source, b"\n```"].concat(),
            [intro.as_slice(), b"```javascript\n", source, b"\n```"].concat(),
            [intro.as_slice(), b"REPLACE_COMPILE\n", fence, source, b"\n```"].concat(),
            [intro.as_slice(), b"CHAT\n", fence, source, b"\n```"].concat(),
            [intro.as_slice(), b"END_ACTION\n", fence, source, b"\n```"].concat(),
            [intro.as_slice(), source, b"\n", fence, source, b"\n```"].concat(),
            [opening.as_slice(), source, b"\n```\n", fence, source, b"\n```"].concat(),
            [response.as_slice(), b"\nREPLACE_COMPILE\n", source, b"\nEND_ACTION"].concat(),
            [response.as_slice(), b"\nEND_ACTION"].concat(),
            [opening.as_slice(), source, b"\n```\nAdditional explanation."].concat(),
            too_long, too_many_lines,
        ] {
            let frame = ai::source_frame::inspect_generated(&invalid, python);
            assert!(frame.is_none_or(|frame| frame.malformed && frame.completed_end.is_none()));
            let mut rejected = panel(python);
            assert!(rejected.completed_generation_output_length(&invalid).is_none());
            assert!(!rejected.accept_generated(&invalid, true, true));
            assert_eq!(rejected.pending, Action::None);
        }
        for end in 1..opening.len() {
            let mut stopped = panel(python);
            assert!(!stopped.stream_model_output(2, &response[..end]).progressed);
            stopped.cancel_delivery();
            assert!(!stopped.stream_model_output(2, &response).progressed);
            assert_eq!(stopped.argument_len, 0);
            assert_eq!(stopped.pending, Action::None);
        }
        let mut stopped = panel(python);
        let mut partial = TextDocument::new();
        stopped.stream_model_output(2, &response[..opening.len() + 12]);
        drain(&mut stopped, &mut partial);
        let visible = partial.bytes().to_vec();
        stopped.cancel_delivery();
        assert!(!stopped.stream_model_output(2, &response).progressed);
        drain(&mut stopped, &mut partial);
        assert_eq!(partial.bytes(), visible);
        stopped.reset_delivery(); stopped.generation = S::Running;
        stopped.stream_model_output(2, &response);
        drain(&mut stopped, &mut partial);
        assert_eq!(partial.bytes(), source);

        let mut repair = panel(python);
        repair.generation = S::Complete; repair.document_revision = 7;
        repair.source_binding = Some(([4;16],2)); repair.workflow_goal = Some(9);
        let request = repair.request[..repair.request_len].to_vec();
        assert!(repair.queue_runtime_repair(&request,7,7,b"actual native failure diagnostics"));
        assert!(repair.preserves_existing_source() && !repair.permits_generated_launch());
        repair.generation = S::Running;
        let original = if python { b"print(0)".as_slice() } else { b"int main(void) { return 1; }".as_slice() };
        let mut existing = TextDocument::new(); assert!(existing.open(original));
        for end in 1..=response.len() {
            repair.stream_model_output(2,&response[..end]);
            assert!(!repair.apply_existing_source(&mut existing,7,None));
            assert_eq!(existing.bytes(),original);
        }
        assert!(repair.accept_generated(&response,true,true));
        repair.generation = S::Complete;
        assert!(repair.apply_existing_source(&mut existing,7,None));
        assert_eq!(existing.bytes(),source);
        assert!(!repair.permits_generated_launch());
        assert!(existing.undo()); assert_eq!(existing.bytes(),original);
    }
    let mut conversation = ai::Panel::new();
    let request = b"Explain this program";
    conversation.request[..request.len()].copy_from_slice(request);
    conversation.request_len = request.len();
    let output = b"Here is an example:\n```python\nprint(1)\n```";
    assert!(!conversation.stage_generated_source(output));
    assert!(!conversation.accept_generated(output,true,true));
    assert_eq!(conversation.pending,Action::None);
    assert_eq!(conversation.argument_len,0);
}

// ------------------------=
// FUNC: terminal_generation_failure_is_preserved
// DESC: Keeps the original terminal failure and its source bytes when the enclosing plan subsequently becomes inactive.
// ------------------=
fn terminal_generation_failure_is_preserved() {
    let mut panel=ai::Panel::new();panel.workflow_goal=Some(3);
    panel.generation=ai::GenerationStatus::Failed;
    panel.reply(&[71,83,4,89]);panel.argument[..3].copy_from_slice(b"abc");panel.argument_len=3;
    let failed=panel;assert!(!panel.cancel_workflow(3));assert!(panel==failed);
    assert!(!panel.apply_generated_compile_requested());
    for status in [ai::GenerationStatus::Queued,ai::GenerationStatus::Running,ai::GenerationStatus::Complete] {
        let mut active=failed;active.generation=status;active.pending=Action::ReplaceCompile;
        assert!(active.cancel_workflow(3));assert_eq!(active.generation,ai::GenerationStatus::Cancelled);
        assert_eq!(active.pending,Action::None);assert_eq!(active.argument_len,0);
    }
}

// ------------------------=
// FUNC: generated_redraw_batching
// DESC: Exercises production stream updates and document delivery to prove hidden token changes do not redraw, while first/status/conversation updates and bounded final draining remain visible.
// ------------------=
fn generated_redraw_batching() {
    use ai::GenerationStatus;
    let request=b"Create a simple graphical calculator app";
    let mut panel=ai::Panel::new();
    panel.request[..request.len()].copy_from_slice(request);
    panel.request_len=request.len();panel.generation=GenerationStatus::Running;
    let first=panel.stream_model_output(2,b"REPLACE_");
    assert!(!first.progressed && first.redraw);
    let prefix=b"REPLACE_COMPILE\n";
    let header=panel.stream_model_output(2,prefix);
    assert!(!header.progressed && !header.redraw);
    let source=b"int main(void) {\n    return 0;\n}";
    let mut response=prefix.to_vec();response.extend_from_slice(source);
    let update=panel.stream_model_output(2,&response[..prefix.len()+15]);
    assert!(update.progressed && !update.redraw);
    let mut document=TextDocument::new();
    assert!(document.open(b"original"));
    let first=panel.stream.ready(&panel.argument[..panel.argument_len],Some(1),false).unwrap();
    assert!(document.append_generation_preview(&panel.argument[first.clone()],true));
    assert!(panel.stream.committed(first.end,Some(1)));
    let first_length=document.bytes().len();
    let mut progressed=0;
    for end in prefix.len()+16..=response.len() {
        let update=panel.stream_model_output(2,&response[..end]);
        progressed+=usize::from(update.progressed);
        assert!(!update.redraw);
        assert!(panel.stream.ready(&panel.argument[..panel.argument_len],Some(end as u64*1_000_000),false).is_none());
        assert_eq!(document.bytes().len(),first_length);
        assert_eq!(panel.pending,Action::None);
    }
    assert!(progressed>1);
    assert!(panel.argument_len>first_length);
    // A separately published checkpoint must still transition visibly back to
    // the streaming status, once, without re-rendering every following token.
    panel.reply(b"checkpoint");
    assert!(panel.stream_model_output(2,&response).redraw);
    assert!(!panel.stream_model_output(2,&response).redraw);
    response.extend_from_slice(b"\nEND_ACTION");
    assert!(panel.accept_for_owner(2,&response,true));
    panel.generation=GenerationStatus::Complete;
    assert!(panel.apply_generated_compile_requested());
    while let Some(range)=panel.stream.ready(&panel.argument[..panel.argument_len],Some(2),true) {
        assert!(range.len()<=ai::generation_stream::MAX_FLUSH_BYTES);
        assert!(document.append_generation_preview(&panel.argument[range.clone()],false));
        assert!(panel.stream.committed(range.end,Some(2)));
    }
    assert_eq!(document.bytes(),source);
    assert_eq!(panel.stream.delivered,panel.argument_len);
    assert!(document.undo());assert_eq!(document.bytes(),b"original");
    // Ordinary editor conversation and non-editor application streams retain
    // their previous token-by-token presentation contract.
    for owner in [0,1,2,3,12] {
        let mut conversation=ai::Panel::new();
        if owner!=2 {
            conversation.request[..request.len()].copy_from_slice(request);
            conversation.request_len=request.len();
        }
        for output in [b"x".as_slice(),b"xy",b"xyz"] {
            let update=conversation.stream_model_output(owner,output);
            assert!(update.progressed && update.redraw);
            assert_eq!(conversation.response_len,output.len());
            assert_eq!(conversation.argument_len,0);
        }
    }
}

// ------------------------=
// FUNC: application_creation
// DESC: Checks explicit creation authority, complete-source admission, revision fencing and exactly-once application.
// ------------------=
fn application_creation() {
    use ai::editor_compile_intent::ApplicationStyle;
    use ai::GenerationBudget::{ApplicationSource, Conversation};
    for request in [b"Create a simple uikit calculator app".as_slice(),
        b"Update the calculator app and add numeric buttons and traditional mathematic calculator operators",
        b"Fix code in the text editor",b"Create Hello world in Python"] {
        assert_eq!(ai::generation_budget(2,request),ApplicationSource);
        for owner in [0,1,3,12] { assert_eq!(ai::generation_budget(owner,request),Conversation); }
    }
    for request in [b"Explain this code".as_slice(),b"Do not update the calculator app",b"What does create Hello world in Python mean?"] {
        assert_eq!(ai::generation_budget(2,request),Conversation);
    }
    for budget in [Conversation,ApplicationSource] {
        assert!(!ai::generation_timed_out(Some(1),Some(70_000_000_000),Some(100_000_000_000),budget));
        assert!(ai::generation_timed_out(Some(1),Some(1),Some(90_000_000_001),budget));
        assert!(!ai::generation_timed_out(Some(1),Some(535_000_000_000),Some(536_000_000_000),budget));
    }
    assert!(ai::generation_timed_out(Some(1),Some(599_000_000_000),Some(600_000_000_001),Conversation));
    assert!(!ai::generation_timed_out(Some(1),Some(599_000_000_000),Some(600_000_000_001),ApplicationSource));
    assert!(!ai::generation_timed_out(Some(1),Some(1_199_000_000_000),Some(1_200_000_000_000),ApplicationSource));
    assert!(ai::generation_timed_out(Some(1),Some(1_199_000_000_000),Some(1_200_000_000_001),ApplicationSource));
    assert_eq!(ai::editor_compile_intent::application_style(b"build a simple UI C based calculator"),Some(ApplicationStyle::Ui));
    assert_eq!(ai::editor_compile_intent::application_style(b"create a basic calculator application"),Some(ApplicationStyle::Terminal));
    assert_eq!(ai::editor_compile_intent::application_style(b"write a graphical calculator app"),Some(ApplicationStyle::Ui));
    assert_eq!(ai::editor_compile_intent::application_style(b"create a chess game and play me"),Some(ApplicationStyle::Ui));
    assert_eq!(ai::editor_compile_intent::application_style(b"build a graphical tic-tac-toe game"),Some(ApplicationStyle::Ui));
    assert_eq!(ai::editor_compile_intent::application_style(b"create a graphical paint application"),Some(ApplicationStyle::Ui));
    for request in [b"Create a simple uikit calculator app".as_slice(),b"create a UI Kit calculator",b"build a UI-Kit calculator"] {
        assert_eq!(ai::editor_compile_intent::application_kind(request),Some(ai::editor_compile_intent::ApplicationKind::Calculator));
        assert_eq!(ai::editor_compile_intent::application_style(request),Some(ApplicationStyle::Ui));
    }
    for (request,launch) in [(b"update the calculator app with UIKit buttons".as_slice(),false),(b"update the calculator app with UI Kit controls and launch",true)] {
        let update=ai::editor_compile_intent::update(request).unwrap();
        assert_eq!(update.target,b"calculator");assert_eq!(update.launch,launch);
        assert!(ai::editor_compile_intent::source_request(request));
        assert_eq!(ai::editor_compile_intent::requests_launch(request),launch);
    }
    for (output, expected) in [
        (b"REPLACE_COMPILE\nint main(void){return 0;}\nEND_ACTION".as_slice(), Some(52)),
        (b"CHAT\nDone\nEND_ACTION\n  ", Some(20)),
        (b"REPLACE_COMPILE\nint main(void){return 0;}", None),
        (b"prose\nEND_ACTION", None),
    ] { assert_eq!(ai::Panel::completed_output_length(output), expected); }
    let mut long = ai::Panel::new();
    let response = [b'x'; 3072];
    long.reply(&response);
    assert_eq!(long.response_len, response.len());
    assert_eq!(&long.response[..long.response_len], &response);
    long.request[..30].copy_from_slice(b"create a simple calculator app");long.request_len=30;
    assert!(long.stage_generated_source(b"REPLACE_COMPILE\nint main(void) {\n    return 0;\n}\nEND_ACT"));
    assert_eq!(&long.argument[..long.argument_len],b"int main(void) {\n    return 0;\n}");
    let mut preview=TextDocument::new();
    assert!(preview.replace_generation_preview(&long.argument[..long.argument_len],true));
    assert!(!preview.is_saved());
    assert!(long.stage_generated_source(b"REPLACE_COMPILE\nint main(void) {\n    return 54;\n}\nEND_ACTION"));
    assert!(preview.replace_generation_preview(&long.argument[..long.argument_len],false));
    assert_eq!(preview.bytes(),b"int main(void) {\n    return 54;\n}");
    assert!(preview.undo());assert!(preview.bytes().is_empty());
    let mut large = [b'x'; 5000];
    large[..16].copy_from_slice(b"REPLACE_COMPILE\n");
    assert!(long.stage_generated_source(&large));
    assert_eq!(long.argument_len, large.len() - 16);
    assert!(long.stage_generated_source(b" \n\tREPLACE_COMPILE\nint main(void){return 9;}"));
    assert_eq!(&long.argument[..long.argument_len], b"int main(void){return 9;}");
    for request in [b"create a basic calculator application".as_slice(), b"Please build a calculator app.", b"generate a C application", b"create a chess game and play me", b"build a graphical tic-tac-toe game", b"create a graphical paint application"] {
        assert!(ai::editor_compile_intent::create_application(request));
        let mut panel=ai::Panel::new();
        panel.request[..request.len()].copy_from_slice(request);panel.request_len=request.len();
        panel.document_revision=8;
        assert!(panel.accept_for_owner(2,b"REPLACE\nint main(void){return 7;}\nEND_ACTION",true));
        panel.generation=ai::GenerationStatus::Running;
        assert!(!panel.apply_generated_compile_requested());
        panel.generation=ai::GenerationStatus::Complete;
        assert!(panel.apply_generated_compile_requested());
        assert_eq!(panel.pending, Action::ReplaceCompile);
        assert_eq!(panel.take_action(9),Action::None);
        assert!(panel.accept_for_owner(2,b"REPLACE\nint main(void){return 7;}\nEND_ACTION",true));
        assert_eq!(panel.take_action(8),Action::ReplaceCompile);
        assert_eq!(panel.take_action(8),Action::None);
        for source in [b"#include <stdio.h>\nint main(void){return 54;}".as_slice(),
            b"int main(void){return 54;}", b"```c\nint main(void){return 54;}\n```"] {
            assert!(!panel.accept_for_owner(2,source,false));
            assert!(!panel.accept_for_owner(1,source,true));
            assert!(panel.accept_for_owner(2,source,true));
            let complete=[source,b"\nEND_ACTION"].concat();
            assert!(panel.accept_for_owner(2,&complete,true));
            assert_eq!(panel.take_action(8),Action::ReplaceCompile);
            assert_eq!(panel.take_action(8),Action::None);
        }
        panel.request_len=0;
        assert!(!panel.accept_for_owner(2,b"int main(void){return 54;}",true));
    }
    for request in [b"explain how to create a calculator app".as_slice(), b"do not create a calculator app",
        b"create only source for a calculator app", b"create a calculator app then delete files",
        b"\"create a calculator app\"", b"create an app without compiling", b"create application"] {
        assert!(!ai::editor_compile_intent::create_application(request));
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
    assert!(!panel.accept_generated(b"REPLACE_COMPILE\nint main(void){return 7;}\nEND_ACTION",false,true));
    assert!(!panel.accept_generated(b"REPLACE_COMPILE\nint main(void){return 7;}",true,false));
    assert!(panel.accept_generated(b"REPLACE_COMPILE\nint main(void){return 7;}\nEND_ACTION",true,true));
    assert!(!ai::supports(0,panel.pending));
    assert!(ai::supports(2,panel.pending));
    assert_eq!(panel.take_action(13),Action::None);
    assert_eq!(document.bytes(),b"hello world");
    assert!(panel.accept_generated(b"REPLACE_COMPILE\nint main(void){return 7;}\nEND_ACTION",true,true));
    let action=panel.take_action(12);
    assert_eq!(action,Action::ReplaceCompile);
    assert!(panel.edit_document(action,&mut document));
    assert_eq!(document.bytes(),b"int main(void){return 7;}");
    assert!(document.undo());
    assert_eq!(document.bytes(),b"hello world");
    for (request, expected) in [
        (b"Write and compile a simple C program with int main(void) that returns 54. Replace the entire document.".as_slice(), Action::ReplaceCompile),
        (b"Please create and compile a C program", Action::ReplaceCompile),
        (b"Generate and compile a C program", Action::ReplaceCompile),
        (b"Write a C program", Action::Replace),
        (b"Explain how to write and compile a C program", Action::Replace),
        (b"Do not write and compile a C program", Action::Replace),
        (b"Write and compile a C program but do not compile it yet", Action::Replace),
        (b"\"Write and compile a C program\"", Action::Replace),
    ] {
        panel.request[..request.len()].copy_from_slice(request);
        panel.request_len = request.len();
        assert!(!panel.accept_for_owner(2, b"REPLACE\nint main(void){return 54;}", false));
        assert_eq!(panel.pending, Action::None);
        assert!(!panel.accept_for_owner(0, b"REPLACE\nint main(void){return 54;}\nEND_ACTION", true));
        assert_eq!(panel.pending, Action::None);
        assert!(panel.accept_for_owner(2, b"REPLACE\nint main(void){return 54;}\nEND_ACTION", true));
        assert_eq!(panel.pending, expected);
        panel.generation = ai::GenerationStatus::Running;
        assert!(!panel.apply_generated_compile_requested());
        panel.generation = ai::GenerationStatus::Complete;
        assert_eq!(panel.apply_generated_compile_requested(), expected == Action::ReplaceCompile);
        assert_eq!(document.bytes(), b"hello world"); // Proposal never applies itself.
        assert_eq!(panel.take_action(13), Action::None); // Stale document is still protected.
        assert!(panel.accept_for_owner(2, b"REPLACE\nint main(void){return 54;}\nEND_ACTION", true));
        assert_eq!(panel.take_action(12), expected);
        assert!(!panel.apply_generated_compile_requested());
        assert_eq!(panel.take_action(12), Action::None);
    }
    panel.request_len = 0;
    for command in [b"save and compile my code".as_slice(), b"Please save and compile this file.",
        b"  SAVE THEN COMPILE MY PROGRAM  "] {
        panel.input[..command.len()].copy_from_slice(command); panel.length = command.len();
        assert!(!panel.propose(false, 12));
        assert!(panel.propose(true, 12));
        assert_eq!(panel.take_action(13), Action::None);
        assert!(panel.propose(true, 12));
        assert_eq!(panel.take_action(12), Action::SaveCompile);
        assert!(ai::supports(2, Action::SaveCompile));
        assert!(!ai::supports(0, Action::SaveCompile));
        panel.defer_compile_until_saved();
        assert_eq!(panel.take_action(12), Action::None);
        assert!(panel.finish_compile_save(true, 12));
        assert!(!panel.finish_compile_save(true, 12));
        // Save As validates the original source, then rebinds to the new path's fingerprint.
        panel.document_revision = 14;
        assert_eq!(panel.take_action(14), Action::Compile);
        panel.document_revision = 12;
        assert_eq!(panel.take_action(12), Action::None);
        panel.defer_compile_until_saved();
        assert!(!panel.finish_compile_save(false, 12));
        assert!(!panel.finish_compile_save(true, 12));
        panel.defer_compile_until_saved();
        assert!(!panel.finish_compile_save(true, 13));
        assert_eq!(panel.pending, Action::None);
        assert_eq!(document.bytes(), b"hello world");
    }
    for command in [b"do not save and compile my code".as_slice(), b"explain save and compile my code",
        b"\"save and compile my code\"", b"save and compile my code then run it"] {
        assert_eq!(ai::editor_compile_intent::parse(command), None);
    }
    panel.input[..7].copy_from_slice(b"compile");panel.length=7;
    assert!(panel.propose(true,12));
    assert_eq!(panel.take_action(12),Action::Compile);
    assert_eq!(panel.take_action(12),Action::None);
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
    let oversized_body = vec![b'x'; ai::GENERATED_SOURCE_CAPACITY + 1];
    let oversized = [b"INSERT\n".as_slice(), &oversized_body, b"\nEND_ACTION"].concat();
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
                .all(|r| r.width == scale && r.height == (ui::editor_tools::preferences::current().line() as u32 - 4) * scale));
            assert!(rects.iter().map(|r| r.width * r.height).sum::<u32>() <= 2 * (ui::editor_tools::preferences::current().line() as u32 - 4) * scale * scale);
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
                        let mut view = ui::editor_tools::Presentation::new();
                        view.menu = menu;
                        view.menu_index = index;
                        ui::editor_tools::publish(view);
                        let visible = ui::app_menu::window(menu.count(), layout.menu_capacity(), index);
                        let row = layout.menu_row(menu, index - visible.first);
                        let point = Point {
                            x: row.x + 2,
                            y: row.y + 2,
                        };
                        assert_eq!(layout.row_at(menu, point), Some(ui::editor_chrome::MenuHit::Entry(index)));
                        if visible.overflow {
                            let up = layout.menu_scroll_row(menu, false);
                            let down = layout.menu_scroll_row(menu, true);
                            assert_eq!(layout.row_at(menu, Point{x:up.x+2,y:up.y+2}),
                                Some(ui::editor_chrome::MenuHit::ScrollUp));
                            assert_eq!(layout.row_at(menu, Point{x:down.x+2,y:down.y+2}),
                                Some(ui::editor_chrome::MenuHit::ScrollDown));
                        }
                    }
                }
                let scrollbar = layout.scrollbar(100, 100);
                assert!(scrollbar.thumb.bottom() <= scrollbar.track.bottom());
                assert!(scrollbar.track.right() <= layout.body.right());
            }
        }
    }
}
