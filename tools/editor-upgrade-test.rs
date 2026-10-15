#![allow(dead_code)]
#[path = "../kernel/ui/mod.rs"]
mod ui;
use ui::{
    editor_tools::{
        self,
        actions::{self, Action},
        preferences::{self, Preferences},
        Language, Token,
    },
    text_editor::TextDocument,
};
// ------------------------=
// FUNC: main
// DESC: Exercises real transformations, rollback, syntax tokens, font records and paged hit geometry.
// ------------------=
fn main() {
    let cases: &[(Action, &[u8], &[u8])] = &[
        (Action::DeleteLine, b"one\ntwo\n", b"two\n"),
        (Action::MoveDown, b"one\ntwo", b"two\none"),
        (Action::Join, b"one\n  two\n", b"one two\n"),
        (Action::Trim, b"one \t\ntwo\n", b"one\ntwo\n"),
        (Action::Indent, b"one\ntwo\n", b"    one\ntwo\n"),
        (Action::Outdent, b"    one\ntwo\n", b"one\ntwo\n"),
        (Action::Comment, b"one\ntwo\n", b"// one\ntwo\n"),
    ];
    for (action, input, expected) in cases {
        let mut d = TextDocument::new();
        assert!(d.open(input));
        d.set_cursor(0);
        assert!(actions::apply(&mut d, *action, b"// "));
        assert_eq!(d.bytes(), *expected);
        assert!(d.undo());
        assert_eq!(d.bytes(), *input);
        assert!(d.redo());
        assert_eq!(d.bytes(), *expected);
    }
    let mut d = TextDocument::new();
    d.open(b"one\ntwo");
    d.set_cursor(4);
    assert!(actions::apply(&mut d, Action::MoveUp, b""));
    assert_eq!(d.bytes(), b"two\none");
    d.open(b"z\na\nb\n");
    d.select(0, 6);
    assert!(actions::apply(&mut d, Action::Sort, b""));
    assert_eq!(d.bytes(), b"a\nb\nz\n");
    d.open(b"one two");
    d.set_cursor(1);
    assert!(actions::apply(&mut d, Action::Word, b""));
    assert_eq!(d.selection(), Some((0, 3)));
    assert!(actions::apply(&mut d, Action::Upper, b""));
    assert_eq!(d.bytes(), b"ONE two");
    assert!(actions::apply(&mut d, Action::Lower, b""));
    assert_eq!(d.bytes(), b"one two");
    d.open(b"one\ntwo");
    d.set_cursor(1);
    assert!(actions::apply(&mut d, Action::Line, b""));
    assert_eq!(d.selection(), Some((0, 4)));
    d.open(&[b'x'; ui::text_editor::DOCUMENT_CAPACITY]);
    d.select(0, 4);
    assert!(!actions::apply(&mut d, Action::Indent, b""));
    assert_eq!(d.bytes().len(), ui::text_editor::DOCUMENT_CAPACITY);
    assert_eq!(d.selection(), Some((0, 4)));
    for (syntax, source) in [
        (Language::Ruby, b"def f # hi\n".as_slice()),
        (Language::Go, b"func f // hi\n"),
        (Language::Java, b"class X // hi\n"),
        (Language::Swift, b"func f // hi\n"),
        (Language::Kotlin, b"fun f // hi\n"),
        (Language::CSharp, b"using X // hi\n"),
        (Language::Sql, b"SELECT x -- hi\n"),
        (Language::Lua, b"local x -- hi\n"),
        (Language::Yaml, b"true # hi\n"),
        (Language::Toml, b"false # hi\n"),
    ] {
        let mut tokens = [Token::Text; 32];
        editor_tools::highlight(source, syntax, &mut tokens);
        assert_eq!(tokens[0], Token::Keyword);
        assert_eq!(tokens[source.len() - 2], Token::Comment);
    }
    for scheme in 0..10 {
        for font in 0..4 {
            for size in 0..6 {
                let p = Preferences { scheme, font, size };
                assert_eq!(Preferences::decode(&p.encode()), Some(p));
                assert!(p.cell() > 0 && p.cell() <= 24);
                assert!(p.line() <= 36);
                assert!(p.record()[4..].iter().any(|p| *p != 0));
            }
        }
    }
    assert!(Preferences::decode(&[0; 8]).is_none());
    let mut bad = Preferences::new().encode();
    bad[4] = 10;
    assert!(Preferences::decode(&bad).is_none());
    use ui::{
        editor_chrome::{Layout, Menu},
        geometry::{Point, Rect},
    };
    for scale in 1..=3 {
        let g = Layout::new(
            Rect {
                x: 0,
                y: 0,
                width: 800 * scale,
                height: 500 * scale,
            },
            scale as usize,
            false,
            editor_tools::Field::None,
        );
        for menu in [
            Menu::Syntax,
            Menu::Actions,
            Menu::Schemes,
            Menu::Fonts,
            Menu::Sizes,
        ] {
            for index in 0..menu.count() {
                let mut v = editor_tools::Presentation::new();
                v.menu = menu;
                v.menu_index = index;
                editor_tools::publish(v);
                let visible=ui::app_menu::window(menu.count(),g.menu_capacity(),index);
                let r = g.menu_row(menu, index-visible.first);
                assert_eq!(
                    g.row_at(
                        menu,
                        Point {
                            x: r.x + 1,
                            y: r.y + 1
                        }
                    ),
                    Some(ui::editor_chrome::MenuHit::Entry(index))
                );
                assert!(r.bottom() <= g.status.y);
                assert!(menu.entry(index).is_some());
            }
        }
    }
    preferences::publish(Preferences::new());
    println!("Editor upgrade behavioral checks passed");
}
