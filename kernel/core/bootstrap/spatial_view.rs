//! Native spatial panel. Decorative assets never substitute for interactive state.
use super::*;
use crate::ui::spatial::{item_card, overview_bounds, world_card, Preview, SpatialState, TABS};
static REFRESHING: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);
#[cfg(any(target_arch = "aarch64", target_arch = "x86_64"))]
static mut ARRIVAL: [u32; 3840 * 2160] = [0; 3840 * 2160];
static mut ARRIVAL_SIZE: usize = 0;

// ------------------------=
// FUNC: arrival_begin
// DESC: Makes world restoration atomic while preserving the source-world backdrop for blending.
// ------------------=
pub fn arrival_begin() {
    REFRESHING.store(true, core::sync::atomic::Ordering::Relaxed);
}
// ------------------------=
// FUNC: arrival_cancel
// DESC: Releases presentation suppression when a workspace operation fails before switching.
// ------------------=
pub fn arrival_cancel() {
    REFRESHING.store(false, core::sync::atomic::Ordering::Relaxed);
}
// ------------------------=
// FUNC: arrival_capture
// DESC: Retains the fully composed destination once; no allocation or app paint is needed per animation frame.
// ------------------=
pub fn arrival_capture() {
    #[cfg(any(target_arch = "aarch64", target_arch = "x86_64"))]
    unsafe {
        ARRIVAL_SIZE = 0;
        if let Some(c) = (*(&raw mut CONSOLE)).as_mut() {
            c.restore_cursor();
            let size = c.display.stride * c.display.height;
            if size <= 3840 * 2160 {
                core::ptr::copy_nonoverlapping(
                    c.display.buffer,
                    (&raw mut ARRIVAL).cast::<u32>(),
                    size,
                );
                ARRIVAL_SIZE = size;
            }
        }
    }
    arrival_cancel();
}
// ------------------------=
// FUNC: arrival_present
// DESC: Crossfades retained world frames and presents the new scene without rebuilding application contents.
// ------------------=
pub fn arrival_present(opacity: u8, x: i32, y: i32) {
    #[cfg(any(target_arch = "aarch64", target_arch = "x86_64"))]
    unsafe {
        if let Some(c) = (*(&raw mut CONSOLE)).as_mut() {
            c.restore_cursor();
            c.display.clear_render_clip();
            if ARRIVAL_SIZE == c.display.stride * c.display.height && ARRIVAL_SIZE != 0 {
                core::ptr::copy_nonoverlapping(
                    (&raw const ARRIVAL).cast::<u32>(),
                    c.display.buffer,
                    ARRIVAL_SIZE,
                );
                launcher_backdrop::fade(&mut c.display, opacity);
            }
            c.display
                .mark_dirty_rect(0, 0, c.display.width, c.display.height);
            c.save_and_draw_cursor(x, y);
            c.display.present_damage();
            if opacity == 255 {
                ARRIVAL_SIZE = 0;
            }
        }
    }
    #[cfg(target_arch = "x86")]
    let _ = (opacity, x, y);
}

// ------------------------=
// FUNC: refreshing
// DESC: Prevents an intermediate desktop frame from reaching scanout beneath an open overlay.
// ------------------=
pub(super) fn refreshing() -> bool {
    REFRESHING.load(core::sync::atomic::Ordering::Relaxed)
}
// ------------------------=
// FUNC: refresh_begin
// DESC: Restores the clean desktop in the persistent back buffer and starts an atomic background refresh.
// ------------------=
pub fn refresh_begin() {
    REFRESHING.store(true, core::sync::atomic::Ordering::Relaxed);
    #[cfg(any(target_arch = "aarch64", target_arch = "x86_64"))]
    unsafe {
        if let Some(c) = (*(&raw mut CONSOLE)).as_mut() {
            c.restore_cursor();
            c.display.clear_render_clip();
            launcher_backdrop::restore(&mut c.display);
        }
    }
}
// ------------------------=
// FUNC: refresh_end
// DESC: Captures updated surfaces without the cursor before the spatial overlay is recomposed and presented.
// ------------------=
pub fn refresh_end() {
    #[cfg(any(target_arch = "aarch64", target_arch = "x86_64"))]
    unsafe {
        if let Some(c) = (*(&raw mut CONSOLE)).as_mut() {
            c.restore_cursor();
            c.display.clear_render_clip();
            launcher_backdrop::capture(&c.display);
        }
    }
    REFRESHING.store(false, core::sync::atomic::Ordering::Relaxed);
}

#[cfg(any(target_arch = "aarch64", target_arch = "x86_64"))]
static mut OPEN: bool = false;
#[cfg(any(target_arch = "aarch64", target_arch = "x86_64"))]
static mut LAST_TAB: usize = usize::MAX;

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
    previews: &[Preview],
    notice: &[u8],
    progress: u8,
    x: i32,
    y: i32,
    editing: Option<(&[u8], usize)>,
    damage: Option<(usize, usize, usize, usize)>,
    zoom: u8,
    dragging: Option<usize>,
    pending_drop: Option<crate::ui::spatial::DropRequest>,
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
            let changed_tab = LAST_TAB != tab;
            LAST_TAB = tab;
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
            if let Some((a, b, w, h)) = damage {
                d.mark_dirty_rect(
                    a * d.width / 1000,
                    b * d.height / 1000,
                    w * d.width / 1000 + 2,
                    h * d.height / 1000 + 2,
                );
            }
            if changed_tab || damage.is_none() {
                d.mark_dirty_rect(
                    d.width * 4 / 100,
                    d.height * 7 / 100,
                    d.width * 92 / 100,
                    d.height * 89 / 100,
                );
            }
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
            let shelf = tab == 3;
            let panel = if shelf {
                rect(45, 570, 910, 340)
            } else {
                rect(45, 80, 910, 830)
            };
            d.glass_panel(panel.0, panel.1, panel.2, panel.3, true);
            let p = rect(70, if shelf { 588 } else { 98 }, 0, 0);
            d.ui_text_strong(p.0, p.1, b"SPATIAL DESKTOP", 200, 236, 255, 1);
            let p = rect(897, if shelf { 585 } else { 95 }, 44, 38);
            d.window_control(p.0 + (p.2.saturating_sub(p.3)) / 2, p.1, p.3, 2, false);
            for (i, label) in TABS.iter().enumerate() {
                let p = rect(70 + i * 176, if shelf { 635 } else { 150 }, 164, 45);
                d.polished_button(p.0, p.1, p.2, p.3, label, tab == i, false);
            }
            if !shelf {
                let p = rect(80, 205, 0, 0);
                d.ui_text(p.0, p.1, notice, 155, 190, 209, 1);
            }
            if tab == 0 {
                for i in (0..previews.len())
                    .filter(|i| *i != focus)
                    .chain(core::iter::once(
                        focus.min(previews.len().saturating_sub(1)),
                    ))
                {
                    let Some(preview) = previews.get(i) else {
                        continue;
                    };
                    let label = preview.label.get();
                    let (a, b, w, h) = overview_bounds(i, focus, zoom, previews.len());
                    let p = rect(a, b, w, h);
                    d.glass_panel(p.0, p.1, p.2, p.3, false);
                    let thumb = (
                        p.0 + 8,
                        p.1 + 8,
                        p.2.saturating_sub(16),
                        p.3.saturating_sub(36),
                    );
                    if preview.visible {
                        d.spatial_preview(preview.slot, thumb);
                    } else {
                        let role = [4, 25, 49, 19, 26][preview.app.min(4) as usize];
                        let _ = d.themed_icon(p.0 + p.2 / 2, p.1 + 28, role, 32);
                    }
                    d.ui_text_elided_strong(
                        p.0 + 12,
                        p.1 + p.3.saturating_sub(24),
                        p.2.saturating_sub(24),
                        label,
                        220,
                        237,
                        247,
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
                if tab == 2 {
                    let center = rect(500, 475, 0, 0);
                    for radius in [105, 175, 240] {
                        d.icon_circle(
                            center.0 as i32,
                            center.1 as i32,
                            (radius * dh / 1000) as i32,
                            (20, 76, 108),
                            64,
                        );
                    }
                }
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
                        let (a, b, w, h) = if shelf {
                            let Some(bounds) = crate::ui::spatial::shelf_card(i, focus) else {
                                continue;
                            };
                            bounds
                        } else {
                            item_card(i, item)
                        };
                        let p = rect(a, b, w, h);
                        d.glass_panel(p.0, p.1, p.2, p.3, false);
                        let _ = d.themed_icon(
                            p.0 + 28,
                            p.1 + 28,
                            if item.object == [0; 16] { 49 } else { 4 },
                            32,
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
                    let p = rect(240, if shelf { 710 } else { 410 }, 0, 0);
                    d.ui_text_strong(
                        p.0,
                        p.1,
                        b"Your ideas, deliberately connected.",
                        211,
                        235,
                        248,
                        1,
                    );
                    let p = rect(180, if shelf { 755 } else { 460 }, 0, 0);
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
                    3=>notice,
                    _=>b"C: collect   Drag: arrange   L: link / unlink   Enter: open   Del: remove   Esc: close",
                },
                143,
                185,
                208,
                1,
            );
            if shelf {
                if let Some(item) = dragging.and_then(|i| state.items.get(i)).and_then(|i| *i) {
                    let p = rect(
                        x.clamp(45, 745) as usize,
                        y.clamp(80, 760) as usize,
                        190,
                        70,
                    );
                    d.glass_panel(p.0, p.1, p.2, p.3, true);
                    d.ui_text_elided_strong(
                        p.0 + 12,
                        p.1 + 20,
                        p.2.saturating_sub(24),
                        item.name.get(),
                        220,
                        242,
                        255,
                    );
                }
            }
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
            if let Some(request) = pending_drop {
                use crate::ui::spatial::DropTarget;
                let p = rect(140, 350, 720, 260);
                d.glass_panel(p.0, p.1, p.2, p.3, true);
                let title: &[u8] = match request.target {
                    DropTarget::Collection(_) => b"Gather this reference?",
                    DropTarget::Editor => b"Open this file in Text Editor?",
                    DropTarget::Folder(_) => b"Copy this file to the selected folder?",
                };
                let p = rect(170, 380, 660, 0);
                d.ui_text_elided_strong(p.0, p.1, p.2, title, 225, 244, 255);
                if let Some(item) = state.items[request.index] {
                    let p = rect(170, 430, 660, 0);
                    d.ui_text_elided_strong(p.0, p.1, p.2, item.name.get(), 136, 213, 250);
                }
                let destination: &[u8] = match &request.target {
                    DropTarget::Collection(group) => {
                        [b"Home".as_slice(), b"Create", b"Research", b"Explore"][*group as usize]
                    }
                    DropTarget::Editor => b"Existing unsaved text will not be replaced.",
                    DropTarget::Folder(path) => path.get(),
                };
                let p = rect(170, 470, 660, 0);
                d.ui_text_elided_strong(p.0, p.1, p.2, destination, 174, 204, 222);
                let p = rect(170, 505, 660, 0);
                d.ui_text(
                    p.0,
                    p.1,
                    b"Original files stay in place. Enter confirms; Esc cancels.",
                    145,
                    188,
                    210,
                    1,
                );
                for (x, label, primary) in
                    [(170, b"Cancel".as_slice(), false), (540, b"Confirm", true)]
                {
                    let p = rect(x, 540, 290, 48);
                    d.polished_button(p.0, p.1, p.2, p.3, label, primary, false);
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
        state,
        tab,
        focus,
        previews,
        notice,
        progress,
        x,
        y,
        editing,
        damage,
        zoom,
        dragging,
        pending_drop,
    );
}
