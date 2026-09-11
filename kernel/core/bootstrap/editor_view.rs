//! IDesign native code-editor rendering; interaction consumes the same geometry.
use crate::ui::{
    editor_tools::{self, Field, Token},
    geometry::Rect,
};
// ------------------------=
// FUNC: decimal
// DESC: Formats one bounded number without allocation for editor and assistant chrome.
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
    // FUNC: code_editor
    // DESC: Paints tools, syntax, selection, line gutter, caret, search field, and status using one monospace grid.
    // ------------------=
    pub(super) fn code_editor(
        &mut self,
        content: Rect,
        input: &[u8],
        scroll: usize,
        scale: usize,
    ) -> usize {
        let view = editor_tools::current();
        let left = content.x.max(0) as usize;
        let top = content.y.max(0) as usize;
        let width = content.width as usize;
        let height = content.height as usize;
        for i in 0..editor_tools::ACTIONS.len() {
            let r = editor_tools::action_rect(content, scale, i);
            self.fill_rounded_rect_alpha(
                r.x as usize,
                r.y as usize,
                r.width as usize,
                r.height as usize,
                6 * scale,
                17,
                40,
                57,
                245,
            );
            self.ui_text(
                r.x as usize + 6 * scale,
                r.y as usize + 6 * scale,
                editor_tools::ACTIONS[i],
                157,
                218,
                244,
                1,
            );
        }
        self.fill_rect(
            left + 8 * scale,
            top + 38 * scale,
            width.saturating_sub(16 * scale),
            scale,
            31,
            64,
            82,
        );
        if view.field != Field::None {
            let label: &[u8] = match view.field {
                Field::Find => b"FIND",
                Field::ReplaceFind => b"FIND (Tab: replacement)",
                Field::ReplaceWith => b"REPLACE ALL (Enter)",
                Field::GoTo => b"GO TO LINE",
                Field::Command => b"COMMAND",
                _ => b"",
            };
            self.ui_text(left + 12 * scale, top + 46 * scale, label, 73, 215, 249, 1);
            let text = if view.field == Field::ReplaceWith {
                &view.replacement[..view.replacement_len]
            } else {
                &view.query[..view.query_len]
            };
            let mut start = 0;
            while start < text.len()
                && self.ui_text_width(&text[start..], 1) > width.saturating_sub(40 * scale)
            {
                start += 1;
            }
            self.ui_text(
                left + 12 * scale,
                top + 67 * scale,
                &text[start..],
                225,
                240,
                250,
                1,
            );
            self.fill_rect(
                left + 12 * scale + self.ui_text_width(&text[start..], 1),
                top + 65 * scale,
                scale,
                18 * scale,
                73,
                215,
                249,
            );
        } else {
            let mut max = view.notice.len();
            while max > 0
                && self.ui_text_width(&view.notice[..max], 1) > width.saturating_sub(24 * scale)
            {
                max -= 1;
            }
            self.ui_text(
                left + 12 * scale,
                top + 52 * scale,
                &view.notice[..view.notice.len().min(max)],
                121,
                155,
                178,
                1,
            );
        }
        let cell = editor_tools::CELL_WIDTH;
        let columns = width.saturating_sub(92 * scale) / (cell * scale);
        let columns = columns.max(1);
        let total = crate::ui::text_editor::visual_line_count(input, columns);
        let rows = height.saturating_sub(130 * scale) / (24 * scale);
        let mut start = crate::ui::text_editor::visual_line_start(input, columns, scroll);
        let mut tokens = [Token::Text; crate::ui::text_editor::DOCUMENT_CAPACITY];
        editor_tools::highlight(input, view.language, &mut tokens);
        self.fill_rect(
            left + 8 * scale,
            top + 90 * scale,
            46 * scale,
            height.saturating_sub(130 * scale),
            9,
            23,
            36,
        );
        let mut logical = input[..start.min(input.len())]
            .iter()
            .filter(|b| **b == b'\n')
            .count()
            + 1;
        for row in 0..rows {
            if start > input.len() {
                break;
            }
            let y = top + (94 + row * 24) * scale;
            let end = input[start..]
                .iter()
                .position(|b| *b == b'\n')
                .map_or(input.len(), |n| start + n);
            let take = (end - start).min(columns);
            if view.cursor >= start && view.cursor <= start + take {
                self.fill_rect_alpha(
                    left + 56 * scale,
                    y - 2 * scale,
                    width.saturating_sub(78 * scale),
                    24 * scale,
                    25,
                    54,
                    77,
                    170,
                );
            }
            let mut num = [0; 20];
            let n = decimal(logical, &mut num);
            self.ui_text(left + 14 * scale, y, n, 109, 143, 166, 1);
            for col in 0..take {
                let index = start + col;
                let x = left + (64 + col * cell) * scale;
                if view
                    .selection
                    .map_or(false, |(a, b)| index >= a && index < b)
                {
                    self.fill_rect(x, y - 2 * scale, cell * scale, 24 * scale, 69, 47, 111);
                }
                let (r, g, b) = tokens.get(index).copied().unwrap_or(Token::Text).color();
                let byte = if matches!(input[index], b'\t' | b'\r') {
                    b' '
                } else {
                    input[index]
                };
                self.editor_glyph(x, y, byte, scale, (r, g, b));
            }
            if let Some((visible, cursor)) = crate::ui::text_input::caret(4) {
                if visible && view.field == Field::None && cursor >= start && cursor <= start + take
                {
                    self.fill_rect(
                        left + (64 + (cursor - start) * cell) * scale,
                        y,
                        2 * scale,
                        18 * scale,
                        108,
                        218,
                        255,
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
        let status_y = top + height.saturating_sub(28 * scale);
        self.fill_rect(
            left + 8 * scale,
            status_y - 6 * scale,
            width.saturating_sub(16 * scale),
            scale,
            31,
            64,
            82,
        );
        self.ui_text(
            left + 14 * scale,
            status_y,
            view.language.name(),
            105,
            210,
            239,
            1,
        );
        let cursor = view.cursor.min(input.len());
        let line = input[..cursor].iter().filter(|b| **b == b'\n').count() + 1;
        let column = cursor
            - input[..cursor]
                .iter()
                .rposition(|b| *b == b'\n')
                .map_or(0, |i| i + 1)
            + 1;
        let mut n = [0; 20];
        let x = left + width.saturating_sub(235 * scale);
        self.ui_text(x, status_y, b"Ln", 128, 161, 180, 1);
        self.ui_text(
            x + 27 * scale,
            status_y,
            decimal(line, &mut n),
            218,
            232,
            242,
            1,
        );
        self.ui_text(x + 78 * scale, status_y, b"Col", 128, 161, 180, 1);
        self.ui_text(
            x + 114 * scale,
            status_y,
            decimal(column, &mut n),
            218,
            232,
            242,
            1,
        );
        self.ui_text(x + 159 * scale, status_y, b"16 KiB", 128, 161, 180, 1);
        total
    }

    // ------------------------=
    // FUNC: editor_glyph
    // DESC: Rasterizes bundled antialiased JetBrains Mono in the same fixed-width cell used for caret and hit testing.
    // ------------------=
    fn editor_glyph(&mut self, x: usize, y: usize, byte: u8, scale: usize, color: (u8, u8, u8)) {
        if !(32..=126).contains(&byte) {
            return;
        }
        let glyph = (byte as usize - 32) * editor_tools::FONT_WIDTH;
        for row in 0..editor_tools::FONT_HEIGHT * scale {
            for col in 0..editor_tools::CELL_WIDTH * scale {
                let alpha = editor_tools::FONT_ATLAS
                    [(row / scale) * editor_tools::FONT_WIDTH * 95 + glyph + col / scale];
                if alpha != 0 {
                    self.blend_color(
                        (x + col) as i32,
                        (y + row) as i32,
                        color.0,
                        color.1,
                        color.2,
                        alpha,
                    );
                }
            }
        }
    }
}
