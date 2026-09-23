//! Native theme-aware glass shelf; all controls and glyphs remain live framebuffer UI.
use super::*;
use crate::ui::app_launcher::minimized_shelf::{self as shelf, Geometry};

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
impl DisplayDevice {
    // ------------------------=
    // FUNC: minimized_app_shelf
    // DESC: Composes minimized windows above app surfaces, using selected-theme icons and a left-opening menu.
    // ------------------=
    pub(super) fn minimized_app_shelf(&mut self) {
        let state = shelf::current();
        let g = Geometry::new(self.width, self.height, state);
        let r = g.rail;
        let x = r.x.max(0) as usize;
        let y = r.y.max(0) as usize;
        let w = r.width as usize;
        let h = r.height as usize;
        self.glass_panel(x, y, w, h, true);
        let snap = state.drag.and_then(|(px, _, _, _)| {
            if px <= 35 {
                Some(true)
            } else if px + (w * 1000 / self.width) as i32 >= 965 {
                Some(false)
            } else {
                None
            }
        });
        if let Some(left) = snap {
            let edge = if left { x + 3 } else { x + w - 6 };
            self.fill_rounded_rect_alpha(
                edge,
                y + 12,
                3,
                h.saturating_sub(24),
                1,
                117,
                223,
                255,
                220,
            );
        }
        self.fill_rounded_rect_alpha(
            x + 2,
            y + 2,
            w.saturating_sub(4),
            (w / 3).max(8),
            8,
            135,
            212,
            255,
            14,
        );
        self.ui_text_centered(
            x,
            w,
            y + w / 10,
            if snap.is_some() {
                b"SNAP"
            } else if state.drag.is_some() {
                b"MOVE"
            } else {
                b"DRAG"
            },
            137,
            199,
            228,
            1,
        );
        if state.count() == 0 {
            self.ui_text_centered(x, w, y + h / 2 + w / 3, b"None", 143, 174, 195, 1);
        }
        for index in 0..g.capacity {
            let Some(id) = state.item(state.offset + index) else {
                break;
            };
            let tile = g.tile_rect(index);
            let tx = tile.x.max(0) as usize;
            let ty = tile.y.max(0) as usize;
            let tw = tile.width as usize;
            let selected = state.menu == Some(id);
            let hovered = state.hover == Some(id);
            self.fill_rounded_rect_alpha(
                tx,
                ty,
                tw,
                tw,
                10,
                13,
                42,
                65,
                if selected {
                    220
                } else if hovered {
                    185
                } else {
                    100
                },
            );
            self.outline_rounded_rect(
                tx,
                ty,
                tw,
                tw,
                10,
                if selected || hovered { 114 } else { 38 },
                if selected || hovered { 215 } else { 88 },
                if selected || hovered { 255 } else { 116 },
            );
            self.launcher_icon(tx + tw / 2, ty + tw / 2, shelf::icon(id), tw * 4 / 5);
            self.fill_rounded_rect_alpha(tx + tw - 8, ty + tw - 8, 5, 5, 2, 63, 221, 174, 255);
            if id < shelf::NAVIGATORS {
                self.fill_rounded_rect_alpha(
                    tx + tw - 22,
                    ty + tw - 22,
                    24,
                    27,
                    8,
                    16,
                    52,
                    75,
                    255,
                );
                self.ui_text(
                    tx + tw - 17,
                    ty + tw - 20,
                    &[b'1' + id as u8],
                    221,
                    242,
                    252,
                    1,
                );
            }
        }
        if state.count() > g.capacity {
            self.ui_text_centered(x, w, y + h - 25, b"v", 123, 209, 251, 1);
        }
        if let Some(id) = state.menu.or(state.hover) {
            let m = g.menu;
            let mx = m.x.max(0) as usize;
            let my = m.y.max(0) as usize;
            let mw = m.width as usize;
            let row = g.row_height as usize;
            self.glass_panel(
                mx,
                my,
                mw,
                if state.menu.is_some() {
                    m.height as usize
                } else {
                    row
                },
                true,
            );
            self.ui_text(
                mx + 14,
                my + (row.saturating_sub(20)) / 2,
                shelf::label(id),
                130,
                191,
                223,
                1,
            );
            if id < shelf::NAVIGATORS {
                self.ui_text(
                    mx + mw - 28,
                    my + row.saturating_sub(20) / 2,
                    &[b'1' + id as u8],
                    130,
                    191,
                    223,
                    1,
                );
            }
            if state.menu.is_none() {
                return;
            }
            self.fill_rect(mx + 12, my + row - 1, mw.saturating_sub(24), 1, 37, 82, 110);
            for (index, label) in [
                b"Restore".as_slice(),
                b"Maximize".as_slice(),
                b"Close".as_slice(),
            ]
            .iter()
            .enumerate()
            {
                let ry = my + (index + 1) * row;
                if state.row == Some(index) {
                    self.fill_rounded_rect_alpha(
                        mx + 5,
                        ry + 2,
                        mw - 10,
                        row - 4,
                        6,
                        48,
                        134,
                        187,
                        145,
                    );
                }
                self.ui_text(
                    mx + 16,
                    ry + row.saturating_sub(20) / 2,
                    label,
                    if index == 2 { 255 } else { 223 },
                    if index == 2 { 158 } else { 236 },
                    if index == 2 { 157 } else { 245 },
                    1,
                );
            }
        }
    }
}
