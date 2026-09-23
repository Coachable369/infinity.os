//! Shared UI-kit glass cards for settled and animated app-switcher surfaces.
use super::*;
use crate::ui::spatial::Preview;
// ------------------------=
// FUNC: paint_card
// DESC: Paints a real app preview with layered glass chrome and a distinct app-name pill.
// ------------------=
pub(super) fn paint_card(
    d: &mut DisplayDevice,
    preview: &Preview,
    p: (usize, usize, usize, usize),
    selected: bool,
) {
    d.glass_panel_with_palette(p.0, p.1, p.2, p.3, false, (14, 40, 62), (97, 178, 224));
    // Continuous reflection falloff avoids a hard, opaque-looking cap.
    let reflection = (p.3 / 3).max(1);
    for row in 0..reflection {
        let inset = if row < 10 { 12 - row } else { 3 };
        d.fill_rect_alpha(
            p.0 + inset,
            p.1 + 3 + row,
            p.2.saturating_sub(inset * 2),
            1,
            159,
            218,
            255,
            (30 * (reflection - row) / reflection) as u8,
        );
    }
    d.outline_rounded_rect(
        p.0 + 3,
        p.1 + 3,
        p.2.saturating_sub(6),
        p.3.saturating_sub(6),
        11,
        43,
        81,
        107,
    );
    // Text is rendered at scale 1. Its container must use those same metrics,
    // including on 1440p/4K displays where the shell's ui_scale is 2.
    let pill_h = (UI_FONT_CELL_HEIGHT + 16).min(p.3 / 3);
    let thumb = (
        p.0 + 8,
        p.1 + 12,
        p.2.saturating_sub(16),
        p.3.saturating_sub(36 + pill_h),
    );
    if preview.visible {
        d.spatial_preview(preview.slot, thumb);
    } else {
        let role = [4, 25, 49, 19, 26][preview.app.min(4) as usize];
        let size = p.3.saturating_sub(42).min(p.2 / 2).min(160).max(24);
        let _ = d.launcher_icon(p.0 + p.2 / 2, p.1 + p.3.saturating_sub(30) / 2, role, size);
    }
    let app_name: &[u8] = match preview.app {
        0 => b"File Navigator",
        1 => b"Command Window",
        2 => b"Text Editor",
        3 => b"Task Manager",
        _ => b"Settings",
    };
    let pill_w = (d.ui_text_width_weighted(app_name, 1, true) + 32).min(p.2.saturating_sub(32));
    let pill_x = p.0 + (p.2 - pill_w) / 2;
    let pill_y = p.1 + p.3 - pill_h - 16;
    d.glass_label_pill(pill_x, pill_y, pill_w, pill_h, app_name, selected);
    if selected {
        d.outline_rounded_rect(p.0, p.1, p.2, p.3, 12, 110, 214, 255);
        for (width, alpha) in [(80, 24), (56, 55), (32, 150)] {
            let width = width.min(p.2 / 2);
            d.fill_rounded_rect_alpha(
                p.0 + (p.2 - width) / 2,
                p.1 + p.3 - 5,
                width,
                3,
                1,
                122,
                222,
                255,
                alpha,
            );
        }
    }
}
