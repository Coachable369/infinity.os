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
        let g = assistant::geometry(window, s, panel.expanded);
        let clip = self.render_clip;
        self.intersect_render_clip(
            window.x.max(0) as usize,
            window.y.max(0) as usize,
            window.width as usize,
            window.height as usize,
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
        if panel.expanded {
            self.app_symbol(t, b'x', TEXT, s);
        } else {
            self.app_card(t, (15, 39, 59), (34, 157, 187), s);
            self.app_ai_mark(Rect {
                x: t.x + 2 * s as i32,
                y: t.y + 8 * s as i32,
                width: 24 * s as u32,
                height: 28 * s as u32,
            });
            self.app_label(
                Rect {
                    y: t.y + 40 * s as i32,
                    height: 24 * s as u32,
                    ..t
                },
                b" AI",
                CYAN,
                true,
                s,
            );
        }
        self.render_clip = clip;
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
