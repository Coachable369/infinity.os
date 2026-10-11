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
                    width: p.width.saturating_sub(116 * s as u32),
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
            self.app_button(g.close, b"x", false, s);
            let body_clip=self.render_clip;
            self.intersect_render_clip(g.body.x.max(0) as usize,g.body.y.max(0) as usize,g.body.width as usize,g.body.height as usize);
            let intro = if panel.response_len>0 {&panel.response[..panel.response_len]}
                else {b"Ask this app for help, or type help to see its supported commands.".as_slice()};
            let preview=panel.pending!=Action::None && panel.argument_len>0;
            let parts=[(&panel.request[..panel.request_len],(20,36,56)),(intro,(15,27,46)),
                (if preview {&panel.argument[..panel.argument_len]}else{b""},(14,35,42))];
            let heights=parts.map(|(text,_)|if text.is_empty(){0}else{
                (self.assistant_lines(text,g.body.width.saturating_sub(24*s as u32) as usize,s)*24*s+24*s) as u32});
            let total=heights.iter().filter(|&&h|h>0).map(|h|h+12*s as u32).sum::<u32>();
            let maximum=total.saturating_sub(g.body.height);
            let offset=panel.scroll.min(maximum);
            let mut top=g.body.y-offset as i32;
            for ((text,color),height) in parts.into_iter().zip(heights) {
                if height==0 {continue;}
                let r=Rect{x:g.body.x,y:top,width:g.body.width,height};
                self.app_card(r,color,(33,58,85),s);
                self.assistant_wrapped(text,inset(r,12*s),s,TEXT);
                top+=height as i32+12*s as i32;
            }
            self.render_clip=body_clip;
            if panel.scroll_max!=maximum || panel.scroll!=offset {
                let mut updated=panel;updated.scroll_max=maximum;updated.scroll=offset;assistant::write(id,updated);
            }
            if maximum>0 {
                self.app_card(g.scrollbar,(11,18,32),(33,58,85),s);
                let thumb=assistant::scroll_thumb(g,offset,maximum);
                self.app_card(thumb,(75,152,192),(93,182,216),s);
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
            let caret=panel.caret.min(panel.length);
            while start < caret
                && self.app_text_width(&panel.input[start..caret], false, s)
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
            if panel.focused && crate::ui::text_input::caret(5).is_some_and(|(visible,_)|visible) {
                let x=c.x+12*s as i32+self.app_text_width(&panel.input[start..caret],false,s) as i32;
                self.fill_rect(x.max(0) as usize,(c.y+10*s as i32).max(0) as usize,s,20*s,160,218,245);
            }
        }
        let t = g.toggle;
        let pulse = if panel.hovered { assistant::glow_intensity(panel.glow_phase) } else { 48 };
        self.assistant_tab_glass(t, g.tab_left, pulse, s);
        self.render_clip = clip;
    }

    // ------------------------=
    // FUNC: assistant_tab_glass
    // DESC: Paints the shared UIKIT fin with an attached straight root, rounded shoulders and antialiased luminous rim.
    // ------------------=
    fn assistant_tab_glass(&mut self, tab: Rect, left: bool, pulse: u8, s: usize) {
        let halo=4*s as i32;
        for y in -halo..tab.height as i32+halo {
            for x in -halo..tab.width as i32+halo {
                let (r,g,b,a)=assistant::tab_style::pixel(tab.width,tab.height,x,y,left,pulse);
                if a!=0 {self.blend_color(tab.x+x,tab.y+y,r,g,b,a);}
            }
        }
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
            let y=rect.y+row as i32*24*s as i32;
            if y>=0 {self.app_text(
                rect.x.max(0) as usize,
                y as usize,
                &text[start..end],
                color,
                false,
                s,
            );}
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
