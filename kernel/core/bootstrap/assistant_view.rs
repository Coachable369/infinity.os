//! Shared compact IDesign assistant rail with explicit reviewed local actions.
use super::app_style::{CYAN, MUTED, TEXT};
use crate::ui::{
    app_assistant::{self as assistant, Action},
    geometry::Rect,
};
impl super::DisplayDevice {
    // ------------------------=
    // FUNC: window_assistant
    // DESC: Renders real app-local conversation and reviewed actions in the shared docked rail.
    // ------------------=
    pub(super) fn window_assistant(&mut self, id: usize, window: Rect, s: usize) {
        let panel = assistant::read(id);
        let g = assistant::geometry_in_viewport(window, self.width, s, panel.expanded);
        let clip = self.render_clip;
        let footprint = window.union(g.toggle).union(Rect {
            x: g.toggle.x - 4 * s as i32,
            y: g.toggle.y - 4 * s as i32,
            width: g.toggle.width.saturating_add(8 * s as u32),
            height: g.toggle.height.saturating_add(8 * s as u32),
        });
        self.intersect_render_clip(
            footprint.x.max(0) as usize,
            footprint.y.max(0) as usize,
            footprint.width as usize,
            footprint.height as usize,
        );
        if panel.expanded {
            let p = g.panel;
            let x = p.x as usize;
            let y = p.y as usize;
            self.app_card(p, (11, 22, 37), (33, 58, 85), s);
            self.app_ai_mark(Rect {
                x: p.x + 16 * s as i32,
                y: p.y + 12 * s as i32,
                width: 36 * s as u32,
                height: 40 * s as u32,
            });
            self.app_label(
                Rect {
                    x: p.x + 68 * s as i32,
                    y: p.y + 8 * s as i32,
                    width: p.width.saturating_sub(96 * s as u32),
                    height: 26 * s as u32,
                },
                b"Infinity AI",
                TEXT,
                true,
                s,
            );
            self.app_label(
                Rect {
                    x: p.x + 68 * s as i32,
                    y: p.y + 32 * s as i32,
                    width: p.width.saturating_sub(96 * s as u32),
                    height: 24 * s as u32,
                },
                b"Local / App context",
                MUTED,
                false,
                s,
            );
            self.fill_rect(
                x + 12 * s,
                y + 64 * s,
                p.width as usize - 24 * s,
                s,
                33,
                58,
                85,
            );
            let mut top = p.y + 80 * s as i32;
            if panel.request_len > 0 {
                let r = Rect {
                    x: p.x + 40 * s as i32,
                    y: top,
                    width: p.width.saturating_sub(56 * s as u32),
                    height: (self.assistant_lines(
                        &panel.request[..panel.request_len],
                        p.width.saturating_sub(80 * s as u32) as usize,
                        s,
                    ) * 24
                        * s
                        + 24 * s)
                        .min(104 * s) as u32,
                };
                self.app_card(r, (20, 36, 56), (33, 58, 85), s);
                self.assistant_wrapped(
                    &panel.request[..panel.request_len],
                    inset(r, 12 * s),
                    s,
                    TEXT,
                );
                top = r.bottom() + 12 * s as i32;
            }
            let preview = panel.pending != Action::None && panel.argument_len > 0;
            let available = (g.apply.y - top - 12 * s as i32).max(0) as u32;
            let preview_height = if preview {
                (100 * s as u32).min(available / 3)
            } else {
                0
            };
            let mut r = Rect {
                x: p.x + 16 * s as i32,
                y: top,
                width: p.width.saturating_sub(32 * s as u32),
                height: available
                    .saturating_sub(preview_height + if preview { 12 * s as u32 } else { 0 }),
            };
            let intro = if panel.response_len > 0 {
                &panel.response[..panel.response_len]
            } else if id == 2 {
                b"Local editor assistance\n\nFind text, insert supplied text, undo, redo or save. Review a proposed action before applying it.\n\nType help for available commands. Code generation is not connected.".as_slice()
            } else {
                b"Local window assistance\n\nMaximize, restore, minimize or refresh this app. Review each action before applying it.\n\nType help for available commands."
            };
            r.height = r.height.min(
                (self.assistant_lines(intro, r.width.saturating_sub(24 * s as u32) as usize, s)
                    * 24
                    * s
                    + 24 * s) as u32,
            );
            self.app_card(r, (15, 27, 46), (33, 58, 85), s);
            self.assistant_wrapped(intro, inset(r, 12 * s), s, TEXT);
            if preview {
                let r = Rect {
                    x: r.x,
                    y: r.bottom() + 12 * s as i32,
                    width: r.width,
                    height: preview_height,
                };
                self.app_card(r, (14, 35, 42), (33, 70, 80), s);
                self.app_label(
                    Rect {
                        x: r.x + 12 * s as i32,
                        y: r.y + 4 * s as i32,
                        width: r.width.saturating_sub(24 * s as u32),
                        height: 24 * s as u32,
                    },
                    b"Proposed changes",
                    MUTED,
                    true,
                    s,
                );
                self.assistant_wrapped(
                    &panel.argument[..panel.argument_len],
                    Rect {
                        x: r.x + 12 * s as i32,
                        y: r.y + 28 * s as i32,
                        width: r.width.saturating_sub(24 * s as u32),
                        height: r.height.saturating_sub(36 * s as u32),
                    },
                    s,
                    (105, 233, 179),
                );
            }
            if panel.pending != Action::None {
                self.app_button(g.apply, b"Apply", true, s);
                self.app_button(g.dismiss, b"Dismiss", false, s);
            }
            let c = g.composer;
            self.app_card(
                c,
                (11, 18, 32),
                if panel.focused { CYAN } else { (33, 58, 85) },
                s,
            );
            let mut start = 0;
            while start < panel.length
                && self.app_text_width(&panel.input[start..panel.length], false, s)
                    > c.width.saturating_sub(24 * s as u32) as usize
            {
                start += 1;
            }
            self.app_label(
                Rect {
                    x: c.x + 12 * s as i32,
                    y: c.y + 8 * s as i32,
                    width: c.width.saturating_sub(24 * s as u32),
                    height: c.height.saturating_sub(16 * s as u32),
                },
                if panel.length == 0 {
                    b"Ask this app..."
                } else {
                    &panel.input[start..panel.length]
                },
                if panel.length == 0 { MUTED } else { TEXT },
                false,
                s,
            );
            self.app_card(g.send, (15, 27, 46), CYAN, s);
            self.app_symbol(g.send, b'^', CYAN, s);
        }
        let t = g.toggle;
        let accent = self.active_accent_rgb();
        let accent = (
            ((accent >> 16) & 0xff) as u8,
            ((accent >> 8) & 0xff) as u8,
            (accent & 0xff) as u8,
        );
        let primary = self.active_primary_rgb();
        let surface = (
            (((primary >> 16) & 0xff) as u8).saturating_add(5),
            (((primary >> 8) & 0xff) as u8).saturating_add(7),
            ((primary & 0xff) as u8).saturating_add(11),
        );
        let pulse = if panel.hovered {
            assistant::glow_intensity(panel.glow_phase)
        } else {
            48
        };
        self.assistant_tab_fill(t, surface, if panel.expanded { 248 } else { 232 }, s);
        self.assistant_tab_outline(t, 3 * s as i32, (accent.0 / 5, accent.1 / 5, accent.2 / 5), s);
        self.assistant_tab_outline(t, 1 * s as i32, (accent.0 / 2, accent.1 / 2, accent.2 / 2), s);
        self.assistant_tab_outline(t, 0, accent, s);
        let center = (t.x + t.width as i32 / 2, t.y + t.height as i32 / 2);
        self.glow_color(
            center.0,
            center.1,
            (18 * s) as i32,
            accent.0,
            accent.1,
            accent.2,
            pulse,
        );
        self.assistant_star(center.0, center.1, accent, pulse, s);
        self.render_clip = clip;
    }

    // ------------------------=
    // FUNC: assistant_tab_fill
    // DESC: Paints the reference kit's compact chamfered glass tab outside usable window content.
    // ------------------=
    fn assistant_tab_fill(&mut self, tab: Rect, color: (u8, u8, u8), alpha: u8, s: usize) {
        let cut = (10 * s).min(tab.height as usize / 3);
        for row in 0..tab.height as usize {
            let inset = if row < cut {
                cut - row
            } else if row + cut >= tab.height as usize {
                row + cut + 1 - tab.height as usize
            } else {
                0
            };
            let width = tab.width as usize - inset.saturating_mul(2);
            if width > 0 {
                self.fill_rect_alpha(
                    (tab.x + inset as i32).max(0) as usize,
                    (tab.y + row as i32).max(0) as usize,
                    width,
                    1,
                    color.0,
                    color.1,
                    color.2,
                    alpha,
                );
            }
        }
    }

    // ------------------------=
    // FUNC: assistant_tab_outline
    // DESC: Draws one theme-colored octagonal edge layer so stacked layers read as a restrained electric glow.
    // ------------------=
    fn assistant_tab_outline(&mut self, tab: Rect, spread: i32, color: (u8, u8, u8), s: usize) {
        let left = tab.x - spread;
        let top = tab.y - spread;
        let right = tab.right() + spread - 1;
        let bottom = tab.bottom() + spread - 1;
        let cut = 10 * s as i32 + spread;
        let points = [
            (left, top + cut),
            (left + cut, top),
            (right - cut, top),
            (right, top + cut),
            (right, bottom - cut),
            (right - cut, bottom),
            (left + cut, bottom),
            (left, bottom - cut),
        ];
        for index in 0..points.len() {
            let next = (index + 1) % points.len();
            self.line(
                points[index].0,
                points[index].1,
                points[next].0,
                points[next].1,
                color.0,
                color.1,
                color.2,
            );
        }
    }

    // ------------------------=
    // FUNC: assistant_star
    // DESC: Renders the kit's centered four-point assistant glyph using the current desktop accent.
    // ------------------=
    fn assistant_star(&mut self, x: i32, y: i32, accent: (u8, u8, u8), pulse: u8, s: usize) {
        let radius = 13 * s as i32;
        for distance in -radius..=radius {
            let taper = radius - distance.abs();
            let horizontal = (taper * 4 / radius.max(1)).max(1);
            let vertical = (taper * 3 / radius.max(1)).max(1);
            for offset in -horizontal..=horizontal {
                self.blend_color(x + offset, y + distance, 232, 249, 255, pulse.saturating_add(90));
            }
            for offset in -vertical..=vertical {
                self.blend_color(x + distance, y + offset, accent.0, accent.1, accent.2, pulse.saturating_add(70));
            }
        }
        self.glow_color(x, y, 4 * s as i32, 255, 255, 255, 220);
    }
    // ------------------------=
    // FUNC: assistant_lines
    // DESC: Measures content-fitting bubbles with the same word wrapping used by the native painter.
    // ------------------=
    fn assistant_lines(&self, text: &[u8], width: usize, s: usize) -> usize {
        let mut start = 0;
        let mut lines = 0;
        while start < text.len() {
            let mut end = start;
            let mut space = None;
            while end < text.len() && text[end] != b'\n' {
                if self.app_text_width(&text[start..end + 1], false, s) > width {
                    break;
                }
                if text[end] == b' ' {
                    space = Some(end);
                }
                end += 1;
            }
            if end < text.len() && text[end] != b'\n' {
                end = space.unwrap_or(end.max(start + 1));
            }
            start = end;
            if matches!(text.get(start), Some(b'\n' | b' ')) {
                start += 1;
            }
            lines += 1;
        }
        lines.max(1)
    }
    // ------------------------=
    // FUNC: assistant_wrapped
    // DESC: Wraps compact antialiased prose inside its card without painting over controls.
    // ------------------=
    fn assistant_wrapped(&mut self, text: &[u8], rect: Rect, s: usize, color: (u8, u8, u8)) {
        let clip = self.render_clip;
        self.intersect_render_clip(
            rect.x.max(0) as usize,
            rect.y.max(0) as usize,
            rect.width as usize,
            rect.height as usize,
        );
        let mut start = 0;
        for row in 0..rect.height as usize / (24 * s) {
            if start >= text.len() {
                break;
            }
            let mut end = start;
            let mut space = None;
            while end < text.len() && text[end] != b'\n' {
                if self.app_text_width(&text[start..end + 1], false, s) > rect.width as usize {
                    break;
                }
                if text[end] == b' ' {
                    space = Some(end);
                }
                end += 1;
            }
            if end < text.len() && text[end] != b'\n' {
                end = space.unwrap_or(end.max(start + 1));
            }
            self.app_text(
                rect.x.max(0) as usize,
                rect.y.max(0) as usize + row * 24 * s,
                &text[start..end],
                color,
                false,
                s,
            );
            start = end;
            if matches!(text.get(start), Some(b'\n' | b' ')) {
                start += 1;
            }
        }
        self.render_clip = clip;
    }
}
// ------------------------=
// FUNC: inset
// DESC: Applies a consistent safe gutter to an app-local card.
// ------------------=
fn inset(r: Rect, padding: usize) -> Rect {
    Rect {
        x: r.x + padding as i32,
        y: r.y + padding as i32,
        width: r.width.saturating_sub(2 * padding as u32),
        height: r.height.saturating_sub(2 * padding as u32),
    }
}
