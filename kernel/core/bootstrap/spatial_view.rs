//! Native spatial panel. Decorative assets never substitute for interactive state.
use super::*;
use crate::ui::spatial::{item_card, overview_card, world_card, SpatialState, TABS};

#[cfg(any(target_arch = "aarch64", target_arch = "x86_64"))]
static mut OPEN: bool = false;

// ------------------------=
// FUNC: close
// DESC: Restores the captured desktop and forces the next ordinary scene to synchronize.
// ------------------=
pub fn close() {
    #[cfg(any(target_arch = "aarch64", target_arch = "x86_64"))]
    unsafe {
        if let Some(c) = (*(&raw mut CONSOLE)).as_mut() {
            c.restore_cursor();
            c.display.clear_render_clip();
            if OPEN {
                launcher_backdrop::restore(&mut c.display);
            }
            OPEN = false;
            c.last_system_screen = 255;
            c.display
                .mark_dirty_rect(0, 0, c.display.width, c.display.height);
        }
    }
}

// ------------------------=
// FUNC: present
// DESC: Composes a functional native panel over a frozen desktop without invoking application paint paths.
// ------------------=
pub fn present(
    state: &SpatialState,
    tab: usize,
    focus: usize,
    visible: [bool; 5],
    notice: &[u8],
    progress: u8,
    x: i32,
    y: i32,
    editing: Option<(&[u8], usize)>,
    damage: Option<(usize, usize, usize, usize)>,
    zoom: u8,
) {
    #[cfg(any(target_arch = "aarch64", target_arch = "x86_64"))]
    unsafe {
        if let Some(c) = (*(&raw mut CONSOLE)).as_mut() {
            c.restore_cursor();
            c.display.clear_render_clip();
            if !OPEN {
                launcher_backdrop::capture(&c.display);
                OPEN = true;
            }
            let d = &mut c.display;
            if let Some((a, b, w, h)) = damage.filter(|_| progress == 255) {
                d.set_render_clip(
                    a * d.width / 1000,
                    b * d.height / 1000,
                    w * d.width / 1000 + 2,
                    h * d.height / 1000 + 2,
                );
            } else {
                d.set_render_clip(
                    d.width * 4 / 100,
                    d.height * 7 / 100,
                    d.width * 92 / 100,
                    d.height * 89 / 100,
                );
            }
            launcher_backdrop::restore(d);
            let lift = (255 - usize::from(progress)) * 35 / 255;
            let (dw, dh) = (d.width, d.height);
            let rect = |a: usize, b: usize, w: usize, h: usize| {
                (
                    a * dw / 1000,
                    (b + lift) * dh / 1000,
                    w * dw / 1000,
                    h * dh / 1000,
                )
            };
            let panel = rect(45, 80, 910, 830);
            d.glass_panel(panel.0, panel.1, panel.2, panel.3, true);
            let p = rect(70, 98, 0, 0);
            d.ui_text_strong(p.0, p.1, b"SPATIAL DESKTOP", 200, 236, 255, 1);
            let p = rect(897, 95, 44, 38);
            d.window_control(p.0 + (p.2.saturating_sub(p.3)) / 2, p.1, p.3, 2, false);
            for (i, label) in TABS.iter().enumerate() {
                let p = rect(70 + i * 176, 150, 164, 45);
                d.polished_button(p.0, p.1, p.2, p.3, label, tab == i, false);
            }
            let p = rect(80, 205, 0, 0);
            d.ui_text(p.0, p.1, notice, 155, 190, 209, 1);
            if tab == 0 {
                let labels = [
                    b"Files".as_slice(),
                    b"Command",
                    b"Text Editor",
                    b"Task Manager",
                    b"Settings",
                ];
                for i in (0..5)
                    .filter(|i| *i != focus)
                    .chain(core::iter::once(focus.min(4)))
                {
                    let label = labels[i];
                    let (a, b, w, h) = overview_card(i, focus, zoom);
                    let p = rect(a, b, w, h);
                    d.glass_panel(p.0, p.1, p.2, p.3, false);
                    let thumb = (
                        p.0 + 8,
                        p.1 + 8,
                        p.2.saturating_sub(16),
                        p.3.saturating_sub(36),
                    );
                    if visible[i] {
                        d.spatial_preview(i, thumb);
                    } else {
                        d.desktop_app_icon(p.0 + p.2 / 2 - 16, p.1 + 12, 32, i, false);
                    }
                    d.ui_text(
                        p.0 + 12,
                        p.1 + p.3.saturating_sub(24),
                        label,
                        220,
                        237,
                        247,
                        1,
                    );
                    if focus == i {
                        d.outline_rounded_rect(p.0, p.1, p.2, p.3, 12, 110, 214, 255);
                    }
                }
            } else if tab == 1 {
                for (i, world) in state.worlds.iter().enumerate() {
                    let (a, b, w, h) = world_card(i);
                    let p = rect(a, b, w, h);
                    let label = if world.name.get().is_empty() {
                        [b"Home".as_slice(), b"Create", b"Research", b"Explore"][i]
                    } else {
                        world.name.get()
                    };
                    d.glass_panel(p.0, p.1, p.2, p.3, focus == i);
                    let radius = (p.2 / 4).max(12);
                    let cx = p.0 + p.2 / 2;
                    let cy = p.1 + p.3 / 4;
                    d.icon_circle(cx as i32, cy as i32, radius as i32, (56, 143, 194), 32);
                    d.icon_circle(cx as i32, cy as i32, (radius + 8) as i32, (23, 77, 113), 32);
                    d.desktop_app_icon(
                        cx - 24,
                        cy - 24,
                        48,
                        [0, 2, 1, 4][i],
                        state.active_world as usize == i,
                    );
                    d.ui_text_elided_strong(
                        p.0 + 16,
                        p.1 + p.3 / 2,
                        p.2.saturating_sub(32),
                        label,
                        224,
                        244,
                        255,
                    );
                    let caption: &[u8] = if state.active_world as usize == i {
                        b"Active environment"
                    } else if world.layout.is_some() {
                        b"Saved environment"
                    } else {
                        b"Save a layout here"
                    };
                    d.ui_text_elided_strong(
                        p.0 + 16,
                        p.1 + p.3 / 2 + 36,
                        p.2.saturating_sub(32),
                        caption,
                        126,
                        186,
                        217,
                    );
                    d.ui_text_elided_strong(
                        p.0 + 16,
                        p.1 + p.3 / 2 + 74,
                        p.2.saturating_sub(32),
                        world.location.get(),
                        136,
                        182,
                        206,
                    );
                    if focus == i {
                        d.outline_rounded_rect(p.0, p.1, p.2, p.3, 16, 101, 213, 255);
                    }
                }
            } else {
                if tab == 4 {
                    for (i, item) in state.items.iter().enumerate() {
                        if let Some(item) = item {
                            let (a, b, w, h) = item_card(i, item);
                            let p = rect(a + w / 2, b + h / 2, 0, 0);
                            for j in i + 1..16 {
                                if item.links & (1 << j) != 0 {
                                    if let Some(target) = state.items[j] {
                                        let (a, b, w, h) = item_card(j, &target);
                                        let q = rect(a + w / 2, b + h / 2, 0, 0);
                                        d.line(
                                            p.0 as i32, p.1 as i32, q.0 as i32, q.1 as i32, 58,
                                            157, 204,
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
                for (i, item) in state.items.iter().enumerate() {
                    if let Some(item) = item {
                        let (a, b, w, h) = item_card(i, item);
                        let p = rect(a, b, w, h);
                        d.glass_panel(p.0, p.1, p.2, p.3, false);
                        d.desktop_app_icon(
                            p.0 + 12,
                            p.1 + 12,
                            32,
                            if item.object == [0; 16] { 2 } else { 1 },
                            false,
                        );
                        d.ui_text_elided_strong(
                            p.0 + 12,
                            p.1 + 52,
                            p.2.saturating_sub(24),
                            item.name.get(),
                            218,
                            238,
                            248,
                        );
                        if tab == 2 {
                            d.ui_text(
                                p.0 + 12,
                                p.1 + 76,
                                [b"Home".as_slice(), b"Create", b"Research", b"Explore"]
                                    [item.collection as usize],
                                120,
                                191,
                                226,
                                1,
                            );
                        }
                        if focus == i {
                            d.outline_rounded_rect(p.0, p.1, p.2, p.3, 12, 110, 214, 255);
                        }
                    }
                }
                if state.items.iter().all(Option::is_none) {
                    let p = rect(240, 410, 0, 0);
                    d.ui_text_strong(
                        p.0,
                        p.1,
                        b"Your ideas, deliberately connected.",
                        211,
                        235,
                        248,
                        1,
                    );
                    let p = rect(180, 460, 0, 0);
                    d.ui_text(
                        p.0,
                        p.1,
                        b"Collect a selected file, or add a text clipping in Matter Shelf.",
                        151,
                        193,
                        214,
                        1,
                    );
                }
                if tab == 2 {
                    for (i, label) in [b"Home".as_slice(), b"Create", b"Research", b"Explore"]
                        .iter()
                        .enumerate()
                    {
                        let p = rect(80 + i * 210, 750, 190, 35);
                        d.polished_button(p.0, p.1, p.2, p.3, label, false, false);
                    }
                }
            }
            let labels: [&[u8]; 4] = match tab {
                0 => [b"Open / focus", b"", b"", b"Reduced motion"],
                1 => [b"Switch", b"Save layout", b"Rename", b"Reduced motion"],
                2 => [
                    b"Collect selected",
                    b"Next collection",
                    b"Remove reference",
                    b"Open",
                ],
                3 => [
                    b"Collect selected",
                    b"Add text",
                    b"Remove reference",
                    b"Insert in editor",
                ],
                _ => [
                    b"Collect selected",
                    b"Link / unlink",
                    b"Remove reference",
                    b"Open",
                ],
            };
            for (i, label) in labels.iter().enumerate() {
                if !label.is_empty() {
                    let p = rect(80 + i * 210, 805, 190, 48);
                    d.polished_button(p.0, p.1, p.2, p.3, label, false, false);
                }
            }
            let p = rect(80, 870, 0, 0);
            d.ui_text(
                p.0,
                p.1,
                match tab {
                    0=>b"Tab: views   Arrows: focus   Wheel / +/-: zoom   Enter: open   M: motion   Esc: close".as_slice(),
                    1=>b"Tab: views   Arrows: focus   Enter: switch   S: save   R: rename   M: motion   Esc: close",
                    2=>b"C: collect   Drag: arrange / drop into a collection   G: next collection   Del: remove",
                    3=>b"C: collect selection   T: add text   Enter: insert into editor   Del: remove   Esc: close",
                    _=>b"C: collect   Drag: arrange   L: link / unlink   Enter: open   Del: remove   Esc: close",
                },
                143,
                185,
                208,
                1,
            );
            if let Some((text, caret)) = editing {
                let p = rect(100, 390, 800, 150);
                d.glass_panel(p.0, p.1, p.2, p.3, true);
                d.ui_text(
                    p.0 + 20,
                    p.1 + 20,
                    b"Enter to save / Escape to cancel",
                    140,
                    207,
                    243,
                    1,
                );
                let start = caret.saturating_sub(48).min(text.len());
                let shown = &text[start..];
                d.ui_text_elided_strong(
                    p.0 + 20,
                    p.1 + 65,
                    p.2.saturating_sub(40),
                    shown,
                    229,
                    244,
                    255,
                );
                let advance = d
                    .ui_text_width(&text[start..caret.min(text.len())], 1)
                    .min(p.2.saturating_sub(44));
                if crate::ui::performance::monotonic_ns().unwrap_or(0) / 500_000_000 % 2 == 0 {
                    d.line(
                        (p.0 + 20 + advance) as i32,
                        (p.1 + 62) as i32,
                        (p.0 + 20 + advance) as i32,
                        (p.1 + 86) as i32,
                        185,
                        238,
                        255,
                    );
                }
            }
            launcher_backdrop::fade(d, progress);
            d.clear_render_clip();
            c.save_and_draw_cursor(x, y);
            c.display.present_damage();
        }
    }
    #[cfg(target_arch = "x86")]
    let _ = (
        state, tab, focus, visible, notice, progress, x, y, editing, damage, zoom,
    );
}
