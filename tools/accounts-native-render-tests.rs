//! Fixture transport and pixel assertions around the production account renderer.
use super::{DisplayDevice, Region};
pub(super) static DEFAULT_KIT: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);
use crate::ui::{
    accounts::{self, Directory, Entry, Mode, State, Tab, Target, Text, View},
    geometry::Point,
    system_layout::{SettingsWindowState, SystemLayout},
};

// ------------------------=
// FUNC: fixture
// DESC: Supplies explicit records to the real renderer without emulating identity or persistence.
// ------------------=
pub(super) fn fixture() -> View {
    let mut data = Directory::empty();
    data.available = true;
    data.administrator = true;
    data.actor = [1; 16];
    data.users_len = 8;
    data.groups_len = 16;
    for i in 0..data.users_len {
        data.users[i] = Entry {
            id: [i as u8 + 1; 16],
            name: Text::from_bytes(if i == 0 {
                b"Morgan Hayes"
            } else if i == 1 {
                b"Jordan Lee"
            } else {
                b"Workspace member"
            }),
            handle: Text::from_bytes(if i == 0 { b"morgan" } else { b"member" }),
            enabled: i != 2,
            admin: i == 0,
            builtin: false,
            members: 0,
        };
    }
    for i in 0..data.groups_len {
        data.groups[i] = Entry {
            id: [i as u8 + 32; 16],
            name: Text::from_bytes(if i == 0 {
                b"Administrators"
            } else if i == 1 {
                b"Design team"
            } else {
                b"Project collaborators"
            }),
            handle: Text::from_bytes(if i == 0 {
                b"administrators"
            } else {
                b"project"
            }),
            enabled: true,
            admin: false,
            builtin: i == 0,
            members: if i == 0 { 1 } else { 3 },
        };
    }
    let state = State::new();
    View { state, data }
}

// ------------------------=
// FUNC: frame
// DESC: Captures production account pixels with the kernel's interaction geometry.
// ------------------=
fn frame(
    width: usize,
    height: usize,
    state: SettingsWindowState,
    view: &View,
    clip: Option<Region>,
) -> Vec<u32> {
    let mut pixels = vec![0x00182739; width * height];
    let mut display = DisplayDevice {
        buffer: pixels.as_mut_ptr(),
        width,
        height,
        stride: width,
        format: 1,
        render_clip: clip,
        fast_motion_frame: false,
        submissions: 0,
        template_typography: [0; 3],
        recording_surface: false,
    };
    display.render_accounts_view(state, display.ui_scale(), view);
    pixels
}

// ------------------------=
// FUNC: verify
// DESC: Checks real account pixels, clipping, disabled state and keyboard reachability and exports review frames.
// ------------------=
pub(super) fn verify() {
    let previous_default = DEFAULT_KIT.swap(true, std::sync::atomic::Ordering::Relaxed);
    let previous_theme = super::TEST_THEME.swap(0, std::sync::atomic::Ordering::Relaxed);
    for (width, height, window_width, window_height) in
        [(1280, 800, 600, 700), (1600, 1000, 900, 800)]
    {
        let window = SettingsWindowState {
            x: 50,
            y: 80,
            width: window_width,
            height: window_height,
            maximized: false,
            expanded_row: None,
            scroll_offset: 0,
            control_focus: 0,
            row_count: 5,
        };
        let layout = SystemLayout::new(width, height);
        let viewport = layout
            .settings_window_geometry_for_section(window, 2)
            .viewport;
        let mut view = fixture();
        if width==1600 {
            let mut fresh=fixture();fresh.data.users_len=1;fresh.data.groups_len=1;
            fresh.data.users[0].name=Text::from_bytes(b"Jonathan");fresh.state=State::new();
            let pixels=frame(width,height,window,&fresh,None);
            let mut ppm=format!("P6\n{width} {height}\n255\n").into_bytes();
            for pixel in pixels{ppm.extend_from_slice(&[(pixel>>16)as u8,(pixel>>8)as u8,pixel as u8]);}
            std::fs::write("build/accounts-first-open-1600.ppm",ppm).unwrap();
        }
        let first = frame(width, height, window, &view, None);
        assert!(first.iter().any(|p| *p != 0x00182739));
        let mut toggled = view;
        toggled.state.focus = Target::Role;
        let changed = frame(width, height, window, &toggled, None);
        assert!(first.iter().zip(&changed).filter(|(a, b)| a != b).count() > 10);
        let clip = Region {
            left: viewport.x.max(0) as usize + 4,
            top: viewport.y.max(0) as usize + 4,
            right: viewport.right().max(0) as usize - 4,
            bottom: viewport.bottom().max(0) as usize - 4,
        };
        let partial = frame(width, height, window, &view, Some(clip));
        for y in 0..height {
            for x in 0..width {
                assert_eq!(
                    partial[y * width + x],
                    if x >= clip.left && x < clip.right && y >= clip.top && y < clip.bottom {
                        first[y * width + x]
                    } else {
                        0x00182739
                    }
                );
            }
        }
        for (tab, mode, label) in [
            (Tab::Users, Mode::Browse, "users"),
            (Tab::Users, Mode::Create, "create"),
            (Tab::Users, Mode::Password, "password"),
            (Tab::Users, Mode::Delete, "delete"),
            (Tab::Groups, Mode::Browse, "groups"),
        ] {
            view.state.switch(tab);
            view.state.select(&view.data, 0);
            if mode == Mode::Delete {
                view.state.select(&view.data, 1);
            }
            if mode != Mode::Browse {
                view.state.begin(mode, &view.data);
            }
            if mode == Mode::Password {
                view.state.password_lengths = [12, 12];
            }
            let pixels = frame(width, height, window, &view, None);
            assert!(pixels.iter().any(|p| *p != 0x00182739));
            let mut ppm = format!("P6\n{width} {height}\n255\n").into_bytes();
            for pixel in pixels {
                ppm.extend_from_slice(&[(pixel >> 16) as u8, (pixel >> 8) as u8, pixel as u8]);
            }
            std::fs::write(format!("build/accounts-{label}-{width}.ppm"), ppm).unwrap();
            for _ in 0..60 {
                let geometry = accounts::layout(viewport, &view.state, &view.data, layout.scale());
                geometry.tab(&mut view.state, false);
                let geometry = accounts::layout(viewport, &view.state, &view.data, layout.scale());
                let control = geometry.controls[..geometry.count]
                    .iter()
                    .find(|c| c.target == view.state.focus)
                    .unwrap();
                assert!(control.enabled);
                let rect = control.bounds.intersection(control.clip);
                assert!(rect.width > 0 && rect.height > 0);
                assert_eq!(
                    geometry.hit(Point {
                        x: rect.x,
                        y: rect.y
                    }),
                    Some(control.target)
                );
            }
        }
        view.state.switch(Tab::Groups);
        view.state.select(&view.data, 0);
        let geometry = accounts::layout(viewport, &view.state, &view.data, layout.scale());
        assert!(!geometry.controls[..geometry.count]
            .iter()
            .any(|c| c.enabled && matches!(c.target, Target::Delete | Target::Rename)));
        view.state.switch(Tab::Users);
        view.state.select(&view.data, 1);
        view.data.administrator = false;
        let geometry = accounts::layout(viewport, &view.state, &view.data, layout.scale());
        assert!(
            !geometry.controls[..geometry.count].iter().any(|c| c.enabled
                && matches!(
                    c.target,
                    Target::New
                        | Target::Delete
                        | Target::Enabled
                        | Target::Member(_)
                        | Target::Rename
                        | Target::Password
                ))
        );
    }
    super::TEST_THEME.store(previous_theme, std::sync::atomic::Ordering::Relaxed);
    DEFAULT_KIT.store(previous_default, std::sync::atomic::Ordering::Relaxed);
}
