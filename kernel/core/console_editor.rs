//! Editor commands share the same state transitions for toolbar and keyboard input.
use super::*;
use crate::ui::editor_tools::{Field, Language};
impl ConsoleRuntime {
    // ------------------------=
    // FUNC: editor_layout
    // DESC: Resolves the same current pixel viewport used by the editor renderer.
    // ------------------=
    pub(super) fn editor_layout(&self) -> crate::ui::editor_chrome::Layout {
        let layout = SystemLayout::new(
            self.system.framebuffer_width,
            self.system.framebuffer_height,
        );
        let window = layout
            .desktop_app_window_geometry(
                self.app_window_x,
                self.app_window_y,
                self.app_window_width,
                self.app_window_height,
                self.app_window_maximized,
            )
            .window;
        crate::ui::editor_chrome::Layout::new(
            window,
            layout.scale(),
            crate::ui::app_assistant::read(2).expanded,
            self.editor_tools.field,
        )
    }
    // ------------------------=
    // FUNC: editor_command
    // DESC: Executes menu items through the same guarded file and document operations as shortcuts.
    // ------------------=
    pub(super) fn editor_command(&mut self, command: crate::ui::editor_chrome::Command) {
        use crate::ui::editor_chrome::{Command as C, Menu};
        let shortcut = match command {
            C::New => b'n',
            C::Open => b'o',
            C::Save => b's',
            C::Undo => b'z',
            C::Redo => b'y',
            C::Cut => b'x',
            C::Copy => b'c',
            C::Paste => b'v',
            C::Find => b'f',
            C::Replace => b'h',
            C::SelectAll => b'a',
            C::Duplicate => b'd',
            C::GoTo => b'g',
            C::Palette => b'p',
            _ => 0,
        };
        if shortcut != 0 {
            self.input_editor_tools(ConsoleKey::Shortcut(shortcut));
        } else {
            match command {
                C::SaveAs => self.open_editor_save_as_dialog(),
                C::Delete => self.delete_editor_document(),
                C::Close => {
                    if self.editor_document.is_saved() {
                        self.close_desktop_app();
                    } else {
                        self.editor_tools.notice = b"Save your modified document before closing.";
                    }
                }
                C::Syntax => self.editor_tools.open_menu(Menu::Syntax),
                C::Assistant => {
                    self.input_window_assistant(ConsoleKey::Shortcut(b'i'));
                }
                C::Performance => self.open_settings(0),
                _ => {}
            }
        }
        self.reveal_editor_cursor();
    }
    // ------------------------=
    // FUNC: pointer_editor_chrome
    // DESC: Gives real menus and document controls exclusive hits without legacy invisible toolbar buttons.
    // ------------------=
    pub(super) fn pointer_editor_chrome(&mut self, clicked: bool) -> bool {
        use crate::ui::{editor_chrome::Menu, geometry::Point};
        if self.mode != ConsoleMode::Desktop
            || self.desktop_app != DesktopAppKind::TextEditor
            || self.editor_dialog != EditorDialog::None
        {
            return false;
        }
        let g = self.editor_layout();
        let p = Point {
            x: self.system.framebuffer_width as i32 * self.pointer_x / 1000,
            y: self.system.framebuffer_height as i32 * self.pointer_y / 1000,
        };
        if self.editor_tools.menu == Menu::None
            && (p.x < g.window.x + 6*g.scale as i32 || p.x >= g.window.right() - 6*g.scale as i32
                || p.y >= g.window.bottom() - 8*g.scale as i32) {return false;}
        if clicked {
            for menu in [Menu::File, Menu::Edit, Menu::Selection, Menu::View] {
                if g.menu_button(menu).contains(p) {
                    self.editor_tools
                        .open_menu(if self.editor_tools.menu == menu {
                            Menu::None
                        } else {
                            menu
                        });
                    return true;
                }
            }
            if g.syntax.contains(p) {
                self.editor_tools
                    .open_menu(if self.editor_tools.menu == Menu::Syntax {
                        Menu::None
                    } else {
                        Menu::Syntax
                    });
                return true;
            }
        }
        if self.editor_tools.menu != Menu::None {
            if let Some(row) = g.row_at(self.editor_tools.menu, p) {
                if self.editor_tools.menu_index != row {
                    self.editor_tools.menu_index = row;
                    self.redraw();
                }
                if clicked {
                    if let Some(command) = self.editor_tools.choose_menu() {
                        self.editor_command(command);
                    }
                }
            } else if clicked {
                self.editor_tools.menu = Menu::None;
            }
            return true;
        }
        if !clicked {
            return false;
        }
        if g.search.contains(p) {
            self.editor_tools.begin(Field::Find);
            return true;
        }
        if self.editor_tools.field != Field::None && g.field_close.contains(p) {
            self.editor_tools.field = Field::None;
            return true;
        }
        if g.body.contains(p) {
            if self.editor_scroll_geometry().maximum_scroll > 0
                && self.editor_scroll_geometry().track.contains(p)
            {
                return false;
            }
            let at = self.editor_pointer_index(SystemLayout::new(
                self.system.framebuffer_width,
                self.system.framebuffer_height,
            ));
            self.editor_document.set_cursor(at);
            self.editor_tools.field = Field::None;
            self.editor_selection_anchor = at;
            self.editor_selection_dragging = true;
            return true;
        }
        p.y >= g.menu_bar.y
            && p.y < g.status.bottom()
            && p.x >= g.menu_bar.x
            && p.x < g.menu_bar.right()
    }

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
                self.editor_tools
                    .open_menu(crate::ui::editor_chrome::Menu::Syntax);
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
        if self.editor_tools.menu != crate::ui::editor_chrome::Menu::None {
            match key {
                ConsoleKey::Up => self.editor_tools.menu_step(-1),
                ConsoleKey::Down => self.editor_tools.menu_step(1),
                ConsoleKey::Home => self.editor_tools.menu_index = 0,
                ConsoleKey::End => {
                    self.editor_tools.menu_index = self.editor_tools.menu.count().saturating_sub(1)
                }
                ConsoleKey::Escape => self.editor_tools.menu = crate::ui::editor_chrome::Menu::None,
                ConsoleKey::Enter => {
                    if let Some(command) = self.editor_tools.choose_menu() {
                        self.editor_command(command);
                    }
                }
                _ => {}
            }
            return true;
        }
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
        let g = self.editor_layout();
        let columns = g.columns();
        let row = crate::ui::text_editor::visual_cursor_row(
            self.editor_document.bytes(),
            columns.max(1),
            self.editor_document.cursor(),
        );
        let visible = g.rows();
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
        let g = self.editor_layout();
        let s = l.scale().max(1);
        let x = self.system.framebuffer_width as i32 * self.pointer_x / 1000;
        let y = self.system.framebuffer_height as i32 * self.pointer_y / 1000;
        let row = self.editor_scroll_row + (y - g.body.y - 8 * s as i32).max(0) as usize / (24 * s);
        let cols = g.columns();
        let bytes = self.editor_document.bytes();
        let start = crate::ui::text_editor::visual_line_start(bytes, cols.max(1), row);
        let end = bytes[start..]
            .iter()
            .position(|b| *b == b'\n')
            .map_or(bytes.len(), |i| start + i)
            .min(start + cols.max(1));
        (start
            + (x - g.body.x - 64 * s as i32).max(0) as usize
                / (crate::ui::editor_tools::CELL_WIDTH * s))
            .min(end)
    }
}
