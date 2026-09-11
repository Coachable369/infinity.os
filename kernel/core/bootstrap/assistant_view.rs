//! Shared IDesign assistant rail for native app windows.
use crate::ui::{
    app_assistant::{self as assistant, Action},
    geometry::Rect,
};
impl super::DisplayDevice {
    // ------------------------=
    // FUNC: window_assistant
    // DESC: Paints the same expandable local assistant for every native app; caller owns window z-order.
    // ------------------=
    pub(super) fn window_assistant(&mut self, id: usize, window: Rect, scale: usize) {
        let panel = assistant::read(id);
        let g = assistant::geometry(window, scale, panel.expanded);
        let clip = self.render_clip;
        self.intersect_render_clip(
            window.x.max(0) as usize,
            window.y.max(0) as usize,
            window.width as usize,
            window.height as usize,
        );
        if panel.expanded {
            let p = g.panel;
            let x = p.x.max(0) as usize;
            let y = p.y.max(0) as usize;
            let w = p.width as usize;
            self.fill_rounded_rect_alpha(x, y, w, p.height as usize, 10 * scale, 11, 27, 43, 252);
            self.outline_rounded_rect(x, y, w, p.height as usize, 10 * scale, 52, 112, 146);
            self.ui_text_strong(
                x + 16 * scale,
                y + 16 * scale,
                b"Infinity AI",
                216,
                242,
                255,
                1,
            );
            self.ui_text(
                x + 16 * scale,
                y + 43 * scale,
                b"LOCAL / THIS APP ONLY",
                93,
                199,
                227,
                1,
            );
            self.fill_rect(
                x + 12 * scale,
                y + 73 * scale,
                w.saturating_sub(24 * scale),
                scale,
                32,
                71,
                94,
            );
            let intro: &[u8] = if panel.response_len == 0 {
                if id == 2 {
                    b"Ask for help, describe document, find TEXT, or insert TEXT. Review each proposed app action before applying it."
                } else {
                    b"Ask for help, maximize, restore, minimize, or refresh. App actions require your confirmation."
                }
            } else {
                &panel.response[..panel.response_len]
            };
            let reserve = if panel.pending != Action::None && panel.argument_len > 0 {
                84
            } else {
                0
            };
            self.assistant_wrapped(
                intro,
                Rect {
                    x: p.x + 16 * scale as i32,
                    y: p.y + 88 * scale as i32,
                    width: p.width.saturating_sub(32 * scale as u32),
                    height: g
                        .apply
                        .y
                        .saturating_sub(p.y + (100 + reserve) * scale as i32)
                        .max(0) as u32,
                },
                scale,
                (194, 217, 233),
            );
            if panel.pending != Action::None {
                for (rect, label, primary) in [
                    (g.apply, b"Apply".as_slice(), true),
                    (g.dismiss, b"Dismiss".as_slice(), false),
                ] {
                    self.fill_rounded_rect_alpha(
                        rect.x as usize,
                        rect.y as usize,
                        rect.width as usize,
                        rect.height as usize,
                        6 * scale,
                        if primary { 5 } else { 18 },
                        if primary { 96 } else { 40 },
                        if primary { 130 } else { 57 },
                        250,
                    );
                    self.outline_rounded_rect(
                        rect.x as usize,
                        rect.y as usize,
                        rect.width as usize,
                        rect.height as usize,
                        6 * scale,
                        66,
                        155,
                        190,
                    );
                    self.ui_text(
                        rect.x as usize + 12 * scale,
                        rect.y as usize + 7 * scale,
                        label,
                        219,
                        241,
                        250,
                        1,
                    );
                }
                if panel.argument_len > 0 {
                    self.assistant_wrapped(
                        &panel.argument[..panel.argument_len],
                        Rect {
                            x: p.x + 16 * scale as i32,
                            y: g.apply.y - 76 * scale as i32,
                            width: p.width.saturating_sub(32 * scale as u32),
                            height: 64 * scale as u32,
                        },
                        scale,
                        (124, 225, 195),
                    );
                }
            }
            let c = g.composer;
            self.fill_rounded_rect_alpha(
                c.x as usize,
                c.y as usize,
                c.width as usize,
                c.height as usize,
                6 * scale,
                5,
                17,
                29,
                255,
            );
            self.outline_rounded_rect(
                c.x as usize,
                c.y as usize,
                c.width as usize,
                c.height as usize,
                6 * scale,
                if panel.focused { 61 } else { 36 },
                if panel.focused { 208 } else { 92 },
                if panel.focused { 246 } else { 123 },
            );
            let mut start = 0;
            while start < panel.length
                && self.ui_text_width(&panel.input[start..panel.length], 1)
                    > (c.width as usize).saturating_sub(16 * scale)
            {
                start += 1;
            }
            self.assistant_wrapped(
                if panel.length == 0 {
                    b"Ask this app..."
                } else {
                    &panel.input[start..panel.length]
                },
                Rect {
                    x: c.x + 8 * scale as i32,
                    y: c.y + 8 * scale as i32,
                    width: c.width.saturating_sub(16 * scale as u32),
                    height: c.height.saturating_sub(12 * scale as u32),
                },
                scale,
                (193, 218, 238),
            );
            self.fill_rounded_rect_alpha(
                g.send.x as usize,
                g.send.y as usize,
                g.send.width as usize,
                g.send.height as usize,
                6 * scale,
                6,
                88,
                122,
                255,
            );
            self.ui_text(
                g.send.x as usize + 10 * scale,
                g.send.y as usize + 18 * scale,
                b">",
                223,
                247,
                255,
                1,
            );
        }
        let t = g.toggle;
        self.fill_rounded_rect_alpha(
            t.x as usize,
            t.y as usize,
            t.width as usize,
            t.height as usize,
            7 * scale,
            7,
            57,
            83,
            255,
        );
        self.outline_rounded_rect(
            t.x as usize,
            t.y as usize,
            t.width as usize,
            t.height as usize,
            7 * scale,
            51,
            186,
            231,
        );
        self.ui_text(
            t.x as usize + 5 * scale,
            t.y as usize + 7 * scale,
            if panel.expanded { b">" } else { b"AI" },
            192,
            243,
            255,
            1,
        );
        self.ui_text(
            t.x as usize + 9 * scale,
            t.y as usize + 27 * scale,
            b"*",
            85,
            220,
            251,
            1,
        );
        self.render_clip = clip;
    }
    // ------------------------=
    // FUNC: assistant_wrapped
    // DESC: Wraps bounded assistant text into its card without painting across controls or window edges.
    // ------------------=
    fn assistant_wrapped(&mut self, text: &[u8], rect: Rect, scale: usize, color: (u8, u8, u8)) {
        let rows = rect.height as usize / (32 * scale);
        let old_clip = self.render_clip;
        self.intersect_render_clip(
            rect.x.max(0) as usize,
            rect.y.max(0) as usize,
            rect.width as usize,
            rect.height as usize,
        );
        let mut start = 0;
        for row in 0..rows {
            if start >= text.len() {
                break;
            }
            let mut end = start;
            let mut space = None;
            while end < text.len() && text[end] != b'\n' {
                if self.ui_text_width(&text[start..end + 1], 1) > rect.width as usize {
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
            self.ui_text(
                rect.x.max(0) as usize,
                rect.y.max(0) as usize + row * 32 * scale,
                &text[start..end],
                color.0,
                color.1,
                color.2,
                1,
            );
            start = end;
            if matches!(text.get(start), Some(b'\n' | b' ')) {
                start += 1;
            }
        }
        self.render_clip = old_clip;
    }
}
