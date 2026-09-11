//! Editor commands share the same state transitions for toolbar and keyboard input.
use super::*;
use crate::ui::editor_tools::{Field, Language, LANGUAGES};
impl ConsoleRuntime {
    // ------------------------=
    // FUNC: editor_tool_action
    // DESC: Opens inline editor tools or executes one undo/selection/syntax operation.
    // ------------------=
    pub(super) fn editor_tool_action(&mut self, index: usize) {
        match index {
            0 => {
                self.editor_document.undo();
            }
            1 => {
                self.editor_document.redo();
            }
            2 => {
                self.editor_tools.begin(Field::Find);
            }
            3 => {
                self.editor_tools.begin(Field::ReplaceFind);
            }
            4 => {
                self.editor_tools.begin(Field::GoTo);
            }
            5 => {
                self.editor_tools.selecting = !self.editor_tools.selecting;
                self.editor_tools.notice =
                    b"Selection mode: click start, then click end. Ctrl+C / X / V";
            }
            6 => {
                let i = LANGUAGES
                    .iter()
                    .position(|l| *l == self.editor_tools.language)
                    .unwrap_or(0);
                self.editor_tools.language = LANGUAGES[(i + 1) % LANGUAGES.len()];
            }
            _ => {}
        }
        self.reveal_editor_cursor();
    }
    // ------------------------=
    // FUNC: input_editor_tools
    // DESC: Handles editor shortcuts and inline find, replacement, line navigation, and command entry.
    // ------------------=
    pub(super) fn input_editor_tools(&mut self, key: ConsoleKey) -> bool {
        if let ConsoleKey::Shortcut(c) = key {
            match c.to_ascii_lowercase() {
                b'z' => self.editor_tool_action(0),
                b'y' => self.editor_tool_action(1),
                b'f' => self.editor_tool_action(2),
                b'h' => self.editor_tool_action(3),
                b'g' => self.editor_tool_action(4),
                b'a' => self
                    .editor_document
                    .select(0, self.editor_document.bytes().len()),
                b'c' | b'x' => {
                    if let Some((a, b)) = self.editor_document.selection() {
                        self.editor_clipboard[..b - a]
                            .copy_from_slice(&self.editor_document.bytes()[a..b]);
                        self.editor_clipboard_length = b - a;
                        if c.to_ascii_lowercase() == b'x' {
                            self.editor_document.replace_selection(b"");
                        }
                    }
                }
                b'v' => {
                    if !self
                        .editor_document
                        .replace_selection(&self.editor_clipboard[..self.editor_clipboard_length])
                    {
                        self.editor_tools.notice = b"Paste rejected: document limit is 16 KiB.";
                    }
                }
                b's' => self.save_editor_document(),
                b'o' => self.open_editor_document(),
                b'n' => {
                    if self.editor_document.is_saved() {
                        self.editor_document.clear();
                        self.editor_document_path_length = 0;
                        self.editor_document_name_length = 0;
                    } else {
                        self.editor_tools.notice =
                            b"Save your modified document before creating a new one.";
                    }
                }
                b'p' => {
                    self.editor_tools.begin(Field::Command);
                    self.editor_tools.notice=b"Commands: undo, redo, select all, duplicate line, copy, cut, paste, syntax, save";
                }
                b'd' => {
                    let cursor = self.editor_document.cursor();
                    self.editor_document.move_cursor_to_line_edge(false);
                    let a = self.editor_document.cursor();
                    self.editor_document.move_cursor_to_line_edge(true);
                    let b = self.editor_document.cursor();
                    let mut line = [0; crate::ui::text_editor::DOCUMENT_CAPACITY];
                    let n = b - a;
                    line[..n].copy_from_slice(&self.editor_document.bytes()[a..b]);
                    if n < line.len() {
                        line[n] = b'\n';
                        self.editor_document.set_cursor(a);
                        self.editor_document.replace_selection(&line[..n + 1]);
                    } else {
                        self.editor_document.set_cursor(cursor);
                    }
                }
                _ => {}
            }
            return true;
        }
        if let ConsoleKey::SelectMove(direction) = key {
            let anchor =
                self.editor_document
                    .selection()
                    .map_or(self.editor_document.cursor(), |(a, b)| {
                        if self.editor_document.cursor() == a {
                            b
                        } else {
                            a
                        }
                    });
            match direction {
                -1 | 1 => {
                    self.editor_document.move_cursor(direction);
                }
                -2 | 2 => {
                    self.editor_document.move_cursor_vertical(direction < 0);
                }
                -3 | 3 => {
                    self.editor_document.move_cursor_to_line_edge(direction > 0);
                }
                _ => {}
            }
            self.editor_document
                .select(anchor, self.editor_document.cursor());
            return true;
        }
        if self.editor_tools.field == Field::None {
            return false;
        }
        match key {
            ConsoleKey::Escape => self.editor_tools.field = Field::None,
            ConsoleKey::Tab(_) => {
                self.editor_tools.field = if self.editor_tools.field == Field::ReplaceWith {
                    Field::ReplaceFind
                } else if self.editor_tools.field == Field::ReplaceFind {
                    Field::ReplaceWith
                } else {
                    self.editor_tools.field
                };
            }
            ConsoleKey::Backspace => {
                if self.editor_tools.field == Field::ReplaceWith {
                    self.editor_tools.replacement_len =
                        self.editor_tools.replacement_len.saturating_sub(1);
                } else {
                    self.editor_tools.query_len = self.editor_tools.query_len.saturating_sub(1);
                }
            }
            ConsoleKey::Character(c) if (32..=126).contains(&c) => {
                let (buf, n) = if self.editor_tools.field == Field::ReplaceWith {
                    (
                        &mut self.editor_tools.replacement,
                        &mut self.editor_tools.replacement_len,
                    )
                } else {
                    (
                        &mut self.editor_tools.query,
                        &mut self.editor_tools.query_len,
                    )
                };
                if *n < buf.len() {
                    buf[*n] = c;
                    *n += 1;
                }
            }
            ConsoleKey::Enter => {
                let query = self.editor_tools.query;
                let query = &query[..self.editor_tools.query_len];
                match self.editor_tools.field {
                    Field::Find => {
                        self.editor_tools.notice = if self.editor_document.find(query, false) {
                            b"Match selected. Enter: next. Escape: edit."
                        } else {
                            b"No match in this document."
                        };
                    }
                    Field::ReplaceFind => self.editor_tools.field = Field::ReplaceWith,
                    Field::ReplaceWith => {
                        self.editor_tools.notice = match self.editor_document.replace_all(
                            query,
                            &self.editor_tools.replacement[..self.editor_tools.replacement_len],
                        ) {
                            Some(0) => b"No matches. Document unchanged.",
                            Some(_) => b"Replaced all matches. Ctrl+Z to undo.",
                            None => b"Replacement rejected: empty search or document capacity.",
                        };
                    }
                    Field::GoTo => {
                        let line = query.iter().try_fold(0usize, |n, c| {
                            if c.is_ascii_digit() {
                                n.checked_mul(10)?.checked_add((c - b'0') as usize)
                            } else {
                                None
                            }
                        });
                        if let Some(line) = line {
                            self.editor_document.goto_line(line.max(1));
                            self.editor_tools.field = Field::None;
                        }
                    }
                    Field::Command => {
                        self.editor_tools.field = Field::None;
                        let key = match query {
                            b"undo" => b'z',
                            b"redo" => b'y',
                            b"select all" => b'a',
                            b"copy" => b'c',
                            b"cut" => b'x',
                            b"paste" => b'v',
                            b"duplicate line" => b'd',
                            b"save" => b's',
                            b"find" => b'f',
                            b"replace" => b'h',
                            b"go to line" => b'g',
                            b"syntax" => {
                                self.editor_tool_action(6);
                                0
                            }
                            _ => {
                                self.editor_tools.notice =
                                    b"Unknown command. Ctrl+P to see supported commands.";
                                0
                            }
                        };
                        if key != 0 {
                            self.input_editor_tools(ConsoleKey::Shortcut(key));
                        }
                    }
                    Field::None => {}
                }
            }
            _ => {}
        }
        true
    }
    // ------------------------=
    // FUNC: reveal_editor_cursor
    // DESC: Keeps the caret visible without jumping edits to the end of the document.
    // ------------------=
    pub(super) fn reveal_editor_cursor(&mut self) {
        let layout = SystemLayout::new(
            self.system.framebuffer_width,
            self.system.framebuffer_height,
        );
        let g = layout.desktop_app_window_geometry(
            self.app_window_x,
            self.app_window_y,
            self.app_window_width,
            self.app_window_height,
            self.app_window_maximized,
        );
        let scale = layout.scale().max(1);
        let columns = (g.content.width as usize).saturating_sub(92 * scale)
            / (crate::ui::editor_tools::CELL_WIDTH * scale);
        let row = crate::ui::text_editor::visual_cursor_row(
            self.editor_document.bytes(),
            columns.max(1),
            self.editor_document.cursor(),
        );
        let visible = (g.content.height as usize).saturating_sub(130 * scale) / (24 * scale);
        if row < self.editor_scroll_row {
            self.editor_scroll_row = row;
        } else if row >= self.editor_scroll_row + visible.max(1) {
            self.editor_scroll_row = row + 1 - visible.max(1);
        }
    }
    // ------------------------=
    // FUNC: detect_editor_language
    // DESC: Updates syntax only when a file is opened or saved, preserving a manual mode between edits.
    // ------------------=
    pub(super) fn detect_editor_language(&mut self) {
        self.editor_tools.language =
            Language::detect(&self.editor_document_name[..self.editor_document_name_length]);
    }
    // ------------------------=
    // FUNC: editor_pointer_index
    // DESC: Maps drag selection to the same monospace buffer grid used for rendering.
    // ------------------=
    pub(super) fn editor_pointer_index(&self, l: SystemLayout) -> usize {
        let g = l.desktop_app_window_geometry(
            self.app_window_x,
            self.app_window_y,
            self.app_window_width,
            self.app_window_height,
            self.app_window_maximized,
        );
        let s = l.scale().max(1);
        let x = self.system.framebuffer_width as i32 * self.pointer_x / 1000;
        let y = self.system.framebuffer_height as i32 * self.pointer_y / 1000;
        let row =
            self.editor_scroll_row + (y - g.content.y - 94 * s as i32).max(0) as usize / (24 * s);
        let cols = (g.content.width as usize).saturating_sub(92 * s)
            / (crate::ui::editor_tools::CELL_WIDTH * s);
        let bytes = self.editor_document.bytes();
        let start = crate::ui::text_editor::visual_line_start(bytes, cols.max(1), row);
        let end = bytes[start..]
            .iter()
            .position(|b| *b == b'\n')
            .map_or(bytes.len(), |i| start + i)
            .min(start + cols.max(1));
        (start
            + (x - g.content.x - 64 * s as i32).max(0) as usize
                / (crate::ui::editor_tools::CELL_WIDTH * s))
            .min(end)
    }
}
