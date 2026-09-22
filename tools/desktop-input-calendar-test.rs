#[path = "../kernel/ui/app_launcher.rs"]
mod app_launcher;
#[path = "../kernel/ui/input_preferences.rs"]
mod input_preferences;
#[path = "../kernel/ui/status_menu.rs"]
mod status_menu;
#[path = "../kernel/ui/mod.rs"]
mod ui;

// ------------------------=
// FUNC: main
// DESC: Exercises input timing, preference serialization, calendar boundaries and immediate launcher scrolling.
// ------------------=
fn main() {
    use ui::geometry::{Point, Rect};
    use ui::system_layout::{scrollbar_thumb_contains, EditorScrollGeometry, SystemLayout};
    let thumb = Rect {
        x: 990,
        y: 100,
        width: 7,
        height: 80,
    };
    assert!(scrollbar_thumb_contains(thumb, Point { x: 987, y: 130 }, 1));
    assert!(!scrollbar_thumb_contains(
        thumb,
        Point { x: 987, y: 200 },
        1
    ));
    let geometry = EditorScrollGeometry {
        track: Rect {
            x: 990,
            y: 100,
            width: 7,
            height: 400,
        },
        thumb,
        maximum_scroll: 1000,
    };
    let layout = SystemLayout::new(1000, 1000);
    assert_eq!(
        layout.desktop_editor_scroll_offset_for_thumb(130, geometry, 30),
        0
    );
    assert_eq!(
        layout.desktop_editor_scroll_offset_for_thumb(290, geometry, 30),
        500
    );
    assert_eq!(
        layout.desktop_editor_scroll_offset_for_thumb(450, geometry, 30),
        1000
    );
    use input_preferences::{Preferences, Repeat};
    let mut p = Preferences::defaults();
    assert_eq!(Preferences::decode([0; 8]), p);
    for row in 0..7 {
        p.cycle(row);
        assert_eq!(Preferences::decode(p.encode()), p);
    }
    input_preferences::apply(p);
    assert_eq!(input_preferences::current(), p);
    assert_eq!(p.wheel(3), -12);
    assert_eq!(p.buttons(5), 6);
    assert_eq!(p.buttons(6), 5);
    let mut repeat = Repeat::new();
    assert!(repeat.press(30, 0, 100, p));
    assert!(!repeat.press(30, 0, 110, p));
    assert_eq!(repeat.poll(100 + p.delay_ms() - 1, p), None);
    assert_eq!(repeat.poll(100 + p.delay_ms(), p), Some((30, 0)));
    assert_eq!(repeat.poll(100 + p.delay_ms(), p), None);
    assert_eq!(repeat.poll(10000, p), Some((30, 0)));
    assert_eq!(repeat.poll(10000, p), None);
    repeat.release(30);
    assert_eq!(repeat.poll(20000, p), None);
    p.cycle(7);
    assert_eq!(p, Preferences::defaults());
    assert_eq!(status_menu::days(1900, 2), 28);
    assert_eq!(status_menu::days(2000, 2), 29);
    assert_eq!(status_menu::weekday(2026, 9, 7), 1);
    status_menu::set_today(2026, 12, 31);
    status_menu::navigate(0);
    assert_eq!(status_menu::month(), (2026, 12, 31));
    status_menu::navigate(1);
    assert_eq!(status_menu::month(), (2027, 1, 0));
    status_menu::navigate(-1);
    assert_eq!(status_menu::month(), (2026, 12, 31));
    for menu in 8..=18 {
        assert!(!status_menu::items(menu).is_empty());
    }
    assert_eq!(status_menu::adjacent(0, false), 15);
    assert_eq!(status_menu::adjacent(16, true), 0);
    assert_eq!(status_menu::adjacent(5, true), 8);
    assert_eq!(status_menu::adjacent(8, false), 5);
    assert_eq!(status_menu::items(17)[0].1, status_menu::Action::Cancel);
    assert_eq!(
        status_menu::items(17)[1].1,
        status_menu::Action::ConfirmRestart
    );
    assert_eq!(
        status_menu::items(18)[1].1,
        status_menu::Action::ConfirmShutdown
    );
    assert!(status_menu::items(15)
        .iter()
        .any(|(_, action)| *action == status_menu::Action::Settings(0)));
    for (width, height) in [(1024, 768), (1920, 1080), (2560, 1600)] {
        use ui::system_layout::{SystemLayout, SystemMenuTarget};
        let layout = SystemLayout::new(width, height);
        for menu in 8..=18 {
            let (x, y, w, h, count) = layout.system_menu_geometry(menu);
            assert!(x + w <= width && y + h <= height);
            let (_, _, saved_width, saved_height) = layout.system_menu_damage_geometry(menu);
            assert!(saved_width <= 640 && saved_height <= 800);
            for row in 0..count {
                let px = x + w / 2;
                let py = y + (20 + row * 34) * layout.scale();
                assert_eq!(
                    layout.system_menu_target(
                        menu,
                        (px * 1000 / width) as i32,
                        (py * 1000 / height) as i32
                    ),
                    SystemMenuTarget::Item(row)
                );
            }
        }
    }
    app_launcher::launcher_open();
    app_launcher::launcher_animation_advance(500, 160);
    assert_eq!(app_launcher::launcher_presentation().transition, 255);
    app_launcher::launcher_scroll_to(300, 500);
    assert_eq!(app_launcher::launcher_presentation().scroll, 300);
    app_launcher::launcher_scroll_by(1000, 500);
    assert_eq!(app_launcher::launcher_presentation().scroll, 500);
    app_launcher::launcher_scroll_by(-1000, 500);
    assert_eq!(app_launcher::launcher_presentation().scroll, 0);
    app_launcher::launcher_begin_close();
    assert!(app_launcher::launcher_animation_advance(500, 160).closed);
}
