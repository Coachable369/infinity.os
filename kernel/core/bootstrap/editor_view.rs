//! Native editor window composed from shared menu and viewport geometry.
use super::app_style::{CYAN, MUTED, TEXT};
use crate::ui::{
    editor_chrome::{Layout, Menu, CODE_INSET, LINE_HEIGHT},
    editor_tools::{self, Field, Token},
    geometry::Rect,
};
// ------------------------=
// FUNC: decimal
// DESC: Formats a bounded status number without allocation.
// ------------------=
pub(super) fn decimal(mut value: usize, out: &mut [u8; 20]) -> &[u8] {
    let mut i = out.len();
    loop {
        i -= 1;
        out[i] = b'0' + (value % 10) as u8;
        value /= 10;
        if value == 0 {
            break;
        }
    }
    &out[i..]
}
impl super::DisplayDevice {
    // ------------------------=
    // FUNC: editor_window
    // DESC: Renders the kit-aligned editor with menus, document identity and a reflowed code viewport.
    // ------------------=
    pub(super) fn editor_window(
        &mut self,
        window: crate::ui::system_layout::DesktopAppWindowGeometry,
        input: &[u8],
        scroll: usize,
        saved: bool,
        _maximized: bool,
        s: usize,
    ) {
        let view = editor_tools::current();
        let g = Layout::new(
            window.window,
            s,
            crate::ui::app_assistant::read(2).expanded,
            view.field,
        );
        let w = g.window;
        let x = w.x.max(0) as usize;
        let y = w.y.max(0) as usize;
        self.app_card(w, (11, 18, 32), (33, 105, 139), s);
        self.fill_rect(x + s, y + 46 * s, w.width as usize - 2 * s, s, 33, 58, 85);
        self.app_symbol(
            Rect {
                x: w.x + 16 * s as i32,
                y: w.y + 12 * s as i32,
                width: 24 * s as u32,
                height: 24 * s as u32,
            },
            b'd',
            MUTED,
            s,
        );
        self.app_text(x + 48 * s, y + 15 * s, b"Text Editor", TEXT, true, s);
        if g.search.width > 70 * s as u32 {
            self.app_card(g.search, (15, 27, 46), (33, 58, 85), s);
            self.app_label(
                Rect {
                    x: g.search.x + 12 * s as i32,
                    width: g.search.width - 24 * s as u32,
                    ..g.search
                },
                b"Search in file...   Ctrl+F",
                MUTED,
                false,
                s,
            );
        }
        for (i, r) in [window.minimize, window.maximize, window.close]
            .iter()
            .enumerate()
        {
            self.app_symbol(*r, [b'-', b'm', b'x'][i], MUTED, s);
        }
        for (m, label) in [
            (Menu::File, b"File".as_slice()),
            (Menu::Edit, b"Edit"),
            (Menu::Selection, b"Selection"),
            (Menu::View, b"View"),
        ] {
            let r = g.menu_button(m);
            if view.menu == m {
                self.app_card(r, (20, 36, 56), (33, 58, 85), s);
            }
            self.app_label(
                Rect {
                    x: r.x + 8 * s as i32,
                    width: r.width.saturating_sub(16 * s as u32),
                    ..r
                },
                label,
                TEXT,
                false,
                s,
            );
        }
        if g.menu_bar.width > 540 * s as u32 && view.path_len > 0 {
            self.app_label(
                Rect {
                    x: g.menu_bar.x + 288 * s as i32,
                    y: g.menu_bar.y,
                    width: g.menu_bar.width.saturating_sub(304 * s as u32),
                    height: g.menu_bar.height,
                },
                &view.path[..view.path_len],
                MUTED,
                false,
                s,
            );
        }
        self.fill_rect(
            g.menu_bar.x as usize,
            g.menu_bar.bottom() as usize - 1,
            g.menu_bar.width as usize,
            1,
            33,
            58,
            85,
        );
        let name = if view.filename_len == 0 {
            b"Untitled".as_slice()
        } else {
            &view.filename[..view.filename_len]
        };
        let tab = g.document_tab();
        self.app_card(tab, (15, 27, 46), (33, 92, 124), s);
        self.app_symbol(
            Rect {
                x: tab.x + 6 * s as i32,
                width: 24 * s as u32,
                ..tab
            },
            b'd',
            MUTED,
            s,
        );
        self.app_symbol(g.tab_close(), b'x', MUTED, s);
        self.app_symbol(g.new_document(), b'+', MUTED, s);
        self.app_label(
            Rect {
                x: tab.x + 36 * s as i32,
                width: tab.width.saturating_sub(70 * s as u32),
                ..tab
            },
            name,
            TEXT,
            false,
            s,
        );
        if !saved {
            self.fill_rounded_rect_alpha(
                tab.x as usize + 7 * s,
                tab.y as usize + 27 * s,
                4 * s,
                4 * s,
                2 * s,
                34,
                211,
                238,
                255,
            );
        }
        self.app_label(
            Rect {
                x: tab.right() + 56 * s as i32,
                y: tab.y,
                width: g.document.width.saturating_sub(tab.width + 88 * s as u32),
                height: tab.height,
            },
            if !view.notice.is_empty() {
                view.notice
            } else if saved {
                b"Saved"
            } else {
                b"Unsaved changes"
            },
            MUTED,
            false,
            s,
        );
        if view.field != Field::None {
            self.app_card(g.field, (15, 27, 46), (34, 157, 187), s);
            let label = match view.field {
                Field::Find => b"Find / Enter: next match".as_slice(),
                Field::ReplaceFind => b"Find / Tab: replacement",
                Field::ReplaceWith => b"Replace all / Enter: apply",
                Field::GoTo => b"Go to line",
                Field::Command => b"Command palette",
                _ => b"",
            };
            self.app_label(
                Rect {
                    x: g.field.x + 12 * s as i32,
                    y: g.field.y,
                    width: g.field.width - 24 * s as u32,
                    height: 24 * s as u32,
                },
                label,
                MUTED,
                false,
                s,
            );
            let text = if view.field == Field::ReplaceWith {
                &view.replacement[..view.replacement_len]
            } else {
                &view.query[..view.query_len]
            };
            let mut start = 0;
            while start < text.len()
                && self.app_text_width(&text[start..], false, s) > g.field.width as usize - 32 * s
            {
                start += 1;
            }
            self.app_text(
                g.field.x as usize + 12 * s,
                g.field.y as usize + 23 * s,
                &text[start..],
                TEXT,
                false,
                s,
            );
            self.fill_rect(
                g.field.x as usize + 13 * s + self.app_text_width(&text[start..], false, s),
                g.field.y as usize + 24 * s,
                s,
                17 * s,
                34,
                211,
                238,
            );
            self.app_button(g.field_close, b"x", false, s);
        }
        self.code_editor(g, input, scroll, s);
        self.fill_rect(
            g.status.x as usize,
            g.status.y as usize,
            g.status.width as usize,
            1,
            33,
            58,
            85,
        );
        let mut n = [0; 20];
        let cursor = view.cursor.min(input.len());
        let line = input[..cursor].iter().filter(|b| **b == b'\n').count() + 1;
        let col = cursor
            - input[..cursor]
                .iter()
                .rposition(|b| *b == b'\n')
                .map_or(0, |i| i + 1)
            + 1;
        self.app_text(
            g.status.x as usize + 16 * s,
            g.status.y as usize + 7 * s,
            b"Ln",
            MUTED,
            false,
            s,
        );
        self.app_text(
            g.status.x as usize + 38 * s,
            g.status.y as usize + 7 * s,
            decimal(line, &mut n),
            TEXT,
            false,
            s,
        );
        if g.status.width > 360 * s as u32 {
            self.app_text(
                g.status.x as usize + 86 * s,
                g.status.y as usize + 7 * s,
                b"Col",
                MUTED,
                false,
                s,
            );
            self.app_text(
                g.status.x as usize + 115 * s,
                g.status.y as usize + 7 * s,
                decimal(col, &mut n),
                TEXT,
                false,
                s,
            );
        }
        if g.status.width > 540 * s as u32 {
            self.app_text(
                g.status.x as usize + 176 * s,
                g.status.y as usize + 7 * s,
                b"Spaces: 4    ASCII    LF",
                MUTED,
                false,
                s,
            );
        }
        self.app_label(
            Rect {
                x: g.syntax.x + 12 * s as i32,
                width: g.syntax.width.saturating_sub(36 * s as u32),
                ..g.syntax
            },
            view.language.name(),
            CYAN,
            false,
            s,
        );
        self.app_label(
            Rect {
                x: g.syntax.right() - 22 * s as i32,
                width: 16 * s as u32,
                ..g.syntax
            },
            if view.menu == Menu::Syntax {
                b"^"
            } else {
                b"v"
            },
            CYAN,
            false,
            s,
        );
        self.window_assistant(2, w, s);
        if view.menu != Menu::None {
            self.editor_popup(g, s);
        }
    }
    // ------------------------=
    // FUNC: code_editor
    // DESC: Paints code and selection using the same reflowed monospace grid as navigation and hit testing.
    // ------------------=
    fn code_editor(&mut self, g: Layout, input: &[u8], scroll: usize, s: usize) {
        let view = editor_tools::current();
        let body = g.body;
        let x = body.x as usize;
        let y = body.y as usize;
        let clip = self.render_clip;
        self.intersect_render_clip(x, y, body.width as usize, body.height as usize);
        self.fill_rect(x, y, body.width as usize, body.height as usize, 11, 18, 32);
        self.fill_rect(x, y, 76 * s, body.height as usize, 10, 22, 37);
        self.fill_rect(x + 76 * s, y, s, body.height as usize, 25, 43, 64);
        let columns = g.columns().max(1);
        let total = crate::ui::text_editor::visual_line_count(input, columns);
        let scroll = scroll.min(total.saturating_sub(g.rows().max(1)));
        let mut start = crate::ui::text_editor::visual_line_start(input, columns, scroll);
        let mut tokens = [Token::Text; crate::ui::text_editor::DOCUMENT_CAPACITY];
        editor_tools::highlight(input, view.language, &mut tokens);
        let mut logical = input[..start.min(input.len())]
            .iter()
            .filter(|b| **b == b'\n')
            .count()
            + 1;
        for row in 0..g.rows() {
            if start > input.len() {
                break;
            }
            let line_y = y + (8 + row * LINE_HEIGHT) * s;
            let end = input[start..]
                .iter()
                .position(|b| *b == b'\n')
                .map_or(input.len(), |i| start + i);
            let take = (end - start).min(columns);
            if view.cursor >= start && view.cursor <= start + take {
                self.fill_rect(
                    x + 77 * s,
                    line_y,
                    body.width as usize - 77 * s,
                    LINE_HEIGHT * s,
                    18,
                    31,
                    50,
                );
            }
            let mut n = [0; 20];
            let number = decimal(logical, &mut n);
            let width = self.app_text_width(number, false, s);
            self.app_text(x + 58 * s - width, line_y, number, MUTED, false, s);
            for col in 0..take {
                let i = start + col;
                let cx = x + (CODE_INSET + col * editor_tools::CELL_WIDTH) * s;
                if view.selection.is_some_and(|(a, b)| i >= a && i < b) {
                    self.fill_rect(
                        cx,
                        line_y,
                        editor_tools::CELL_WIDTH * s,
                        LINE_HEIGHT * s,
                        49,
                        31,
                        90,
                    );
                }
                let byte = if matches!(input[i], b'\t' | b'\r') {
                    b' '
                } else {
                    input[i]
                };
                self.editor_glyph(cx, line_y + 3 * s, byte, s, tokens[i].color());
            }
            if let Some((visible, cursor)) = crate::ui::text_input::caret(4) {
                if visible && view.field == Field::None && cursor >= start && cursor <= start + take
                {
                    self.fill_rect(
                        x + (CODE_INSET + (cursor - start) * editor_tools::CELL_WIDTH) * s,
                        line_y + 3 * s,
                        s,
                        19 * s,
                        34,
                        211,
                        238,
                    );
                }
            }
            if take == columns {
                start += take;
            } else if end == input.len() {
                break;
            } else {
                start = end + 1;
                logical += 1;
            }
        }
        let bar = g.scrollbar(total, scroll);
        if bar.maximum_scroll > 0 {
            self.fill_rounded_rect_alpha(
                bar.thumb.x as usize,
                bar.thumb.y as usize,
                bar.thumb.width as usize,
                bar.thumb.height as usize,
                3 * s,
                62,
                89,
                119,
                255,
            );
        }
        self.render_clip = clip;
    }
    // ------------------------=
    // FUNC: editor_popup
    // DESC: Paints executable menu rows, shortcut hints and the selected syntax mode.
    // ------------------=
    fn editor_popup(&mut self, g: Layout, s: usize) {
        let view = editor_tools::current();
        let p = g.popup(view.menu);
        self.app_card(p, (15, 27, 46), (49, 92, 123), s);
        for i in 0..view.menu.count() {
            let item = view.menu.entry(i).unwrap();
            let r = g.menu_row(view.menu, i);
            if i == view.menu_index {
                self.app_card(r, (20, 54, 76), (36, 137, 169), s);
            }
            let selected = matches!(item.command,crate::ui::editor_chrome::Command::Language(index) if editor_tools::LANGUAGES[index]==view.language);
            self.app_label(
                Rect {
                    x: r.x + 10 * s as i32,
                    width: 18 * s as u32,
                    ..r
                },
                if selected { b">" } else { b"" },
                CYAN,
                true,
                s,
            );
            let hint = self.app_text_width(item.shortcut, false, s);
            self.app_label(
                Rect {
                    x: r.x + 30 * s as i32,
                    width: r.width.saturating_sub((54 * s + hint) as u32),
                    ..r
                },
                item.label,
                TEXT,
                false,
                s,
            );
            self.app_label(
                Rect {
                    x: r.right() - 12 * s as i32 - hint as i32,
                    width: hint as u32,
                    ..r
                },
                item.shortcut,
                MUTED,
                false,
                s,
            );
        }
    }
    // ------------------------=
    // FUNC: editor_glyph
    // DESC: Rasterizes bundled JetBrains Mono within the fixed cell used by caret and pointer selection.
    // ------------------=
    pub(super) fn editor_glyph(
        &mut self,
        x: usize,
        y: usize,
        byte: u8,
        scale: usize,
        color: (u8, u8, u8),
    ) {
        if !(32..=126).contains(&byte) {
            return;
        }
        let glyph = (byte as usize - 32) * editor_tools::FONT_WIDTH;
        for row in 0..editor_tools::FONT_HEIGHT * scale {
            for col in 0..editor_tools::CELL_WIDTH * scale {
                let a = editor_tools::FONT_ATLAS
                    [row / scale * editor_tools::FONT_WIDTH * 95 + glyph + col / scale];
                if a != 0 {
                    self.blend_color(
                        (x + col) as i32,
                        (y + row) as i32,
                        color.0,
                        color.1,
                        color.2,
                        a,
                    );
                }
            }
        }
    }
}
