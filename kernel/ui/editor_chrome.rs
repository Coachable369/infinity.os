//! Shared editor menu, viewport and hit geometry. No renderer-only controls.
use super::{
    editor_tools::Field,
    geometry::{Point, Rect},
};
pub const ROW_HEIGHT: u32 = 32;
pub const LINE_HEIGHT: usize = 22;
pub const CODE_INSET: usize = 96;

impl Layout {
    // ------------------------=
    // FUNC: caret_damage
    // DESC: Returns only visible caret strips, including both cells at an exact soft-wrap boundary.
    // ------------------=
    pub fn caret_damage(
        self,
        input: &[u8],
        cursor: usize,
        scroll: usize,
    ) -> impl Iterator<Item = Rect> + '_ {
        let columns = self.columns().max(1);
        let total = super::text_editor::visual_line_count(input, columns);
        let scroll = scroll.min(total.saturating_sub(self.rows().max(1)));
        let mut start = super::text_editor::visual_line_start(input, columns, scroll);
        (0..self.rows()).filter_map(move |row| {
            if start > input.len() {
                return None;
            }
            let end = input[start..]
                .iter()
                .position(|b| *b == b'\n')
                .map_or(input.len(), |n| start + n);
            let take = (end - start).min(columns);
            let result = (cursor >= start && cursor <= start + take).then(|| Rect {
                x: self.body.x
                    + ((CODE_INSET + (cursor - start) * super::editor_tools::CELL_WIDTH)
                        * self.scale) as i32,
                y: self.body.y + ((11 + row * LINE_HEIGHT) * self.scale) as i32,
                width: self.scale as u32,
                height: (19 * self.scale) as u32,
            });
            start = if take == columns {
                start + take
            } else if end == input.len() {
                input.len() + 1
            } else {
                end + 1
            };
            result
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnsavedDialogGeometry {
    pub sheet: Rect,
    pub cancel: Rect,
    pub discard: Rect,
    pub save: Rect,
}

// ------------------------=
// FUNC: unsaved_dialog_geometry
// DESC: Returns one compact kit-aligned three-way decision sheet contained by the editor viewport.
// ------------------=
pub fn unsaved_dialog_geometry(content: Rect, scale: usize) -> UnsavedDialogGeometry {
    let scale = scale.max(1);
    let width = (560 * scale).min((content.width as usize).saturating_sub(24 * scale));
    let height = (220 * scale).min((content.height as usize).saturating_sub(24 * scale));
    let x = content.x + (content.width as i32 - width as i32) / 2;
    let y = content.y + (content.height as i32 - height as i32) / 2;
    let button_y = y + height as i32 - 62 * scale as i32;
    let button_width = (142 * scale).min(width.saturating_sub(64 * scale) / 3);
    let gap = 12 * scale;
    let group_width = button_width * 3 + gap * 2;
    let button_x = x + (width.saturating_sub(group_width) / 2) as i32;
    let button = |offset: usize| Rect {
        x: button_x + offset as i32,
        y: button_y,
        width: button_width as u32,
        height: (40 * scale) as u32,
    };
    UnsavedDialogGeometry {
        sheet: Rect {
            x,
            y,
            width: width as u32,
            height: height as u32,
        },
        cancel: button(0),
        discard: button(button_width + gap),
        save: button((button_width + gap) * 2),
    }
}

// ------------------------=
// FUNC: default_window
// DESC: Preserves the kit's landscape proportions even when firmware reports a square framebuffer.
// ------------------=
pub fn default_window(width: usize, height: usize) -> super::system_layout::DesktopAppWindowState {
    let scale = super::system_layout::SystemLayout::new(width, height)
        .scale()
        .max(1);
    let pixels = (1250 * scale)
        .min(width * 9 / 10)
        .min(height * 3 / 4 * 1250 / 622);
    let w = pixels * 1000 / width.max(1);
    let h = (pixels * 622 / 1250).min(height * 3 / 4) * 1000 / height.max(1);
    super::system_layout::DesktopAppWindowState::new(
        (1000 - w as i32) / 2,
        (1000 - h as i32) / 2,
        w as i32,
        h as i32,
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Menu {
    None,
    File,
    Edit,
    Selection,
    View,
    Syntax,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    New,
    Open,
    Save,
    SaveAs,
    Delete,
    Close,
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    Find,
    Replace,
    SelectAll,
    Duplicate,
    GoTo,
    Syntax,
    Assistant,
    Palette,
    Performance,
    Language(usize),
}
#[derive(Clone, Copy)]
pub struct Entry {
    pub label: &'static [u8],
    pub shortcut: &'static [u8],
    pub command: Command,
}
const FILE: [Entry; 6] = [
    Entry {
        label: b"New file",
        shortcut: b"Ctrl+N",
        command: Command::New,
    },
    Entry {
        label: b"Open file...",
        shortcut: b"Ctrl+O",
        command: Command::Open,
    },
    Entry {
        label: b"Save",
        shortcut: b"Ctrl+S",
        command: Command::Save,
    },
    Entry {
        label: b"Save as...",
        shortcut: b"",
        command: Command::SaveAs,
    },
    Entry {
        label: b"Delete file",
        shortcut: b"",
        command: Command::Delete,
    },
    Entry {
        label: b"Close window",
        shortcut: b"",
        command: Command::Close,
    },
];
const EDIT: [Entry; 7] = [
    Entry {
        label: b"Undo",
        shortcut: b"Ctrl+Z",
        command: Command::Undo,
    },
    Entry {
        label: b"Redo",
        shortcut: b"Ctrl+Y",
        command: Command::Redo,
    },
    Entry {
        label: b"Cut",
        shortcut: b"Ctrl+X",
        command: Command::Cut,
    },
    Entry {
        label: b"Copy",
        shortcut: b"Ctrl+C",
        command: Command::Copy,
    },
    Entry {
        label: b"Paste",
        shortcut: b"Ctrl+V",
        command: Command::Paste,
    },
    Entry {
        label: b"Find...",
        shortcut: b"Ctrl+F",
        command: Command::Find,
    },
    Entry {
        label: b"Replace...",
        shortcut: b"Ctrl+H",
        command: Command::Replace,
    },
];
const SELECTION: [Entry; 3] = [
    Entry {
        label: b"Select all",
        shortcut: b"Ctrl+A",
        command: Command::SelectAll,
    },
    Entry {
        label: b"Duplicate line",
        shortcut: b"Ctrl+D",
        command: Command::Duplicate,
    },
    Entry {
        label: b"Go to line...",
        shortcut: b"Ctrl+G",
        command: Command::GoTo,
    },
];
const VIEW: [Entry; 4] = [
    Entry {
        label: b"Syntax highlighting",
        shortcut: b">",
        command: Command::Syntax,
    },
    Entry {
        label: b"Toggle AI sidebar",
        shortcut: b"Ctrl+I",
        command: Command::Assistant,
    },
    Entry {
        label: b"Command palette...",
        shortcut: b"Ctrl+P",
        command: Command::Palette,
    },
    Entry {
        label: b"Performance settings",
        shortcut: b"",
        command: Command::Performance,
    },
];
impl Menu {
    // ------------------------=
    // FUNC: count
    // DESC: Returns only implemented menu items.
    // ------------------=
    pub fn count(self) -> usize {
        match self {
            Self::None => 0,
            Self::File => FILE.len(),
            Self::Edit => EDIT.len(),
            Self::Selection => SELECTION.len(),
            Self::View => VIEW.len(),
            Self::Syntax => super::editor_tools::LANGUAGES.len(),
        }
    }
    // ------------------------=
    // FUNC: entry
    // DESC: Maps each visible row to its executable command and keyboard hint.
    // ------------------=
    pub fn entry(self, index: usize) -> Option<Entry> {
        if self == Self::Syntax {
            return super::editor_tools::LANGUAGES.get(index).map(|l| Entry {
                label: l.name(),
                shortcut: b"",
                command: Command::Language(index),
            });
        }
        let entries: &[Entry] = match self {
            Self::File => &FILE,
            Self::Edit => &EDIT,
            Self::Selection => &SELECTION,
            Self::View => &VIEW,
            _ => &[],
        };
        entries.get(index).copied()
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Layout {
    pub window: Rect,
    pub menu_bar: Rect,
    pub document: Rect,
    pub body: Rect,
    pub status: Rect,
    pub syntax: Rect,
    pub search: Rect,
    pub field: Rect,
    pub field_close: Rect,
    pub scale: usize,
}
// ------------------------=
// FUNC: box_at
// DESC: Creates a bounded pixel rectangle for native controls.
// ------------------=
fn box_at(x: i32, y: i32, w: u32, h: u32) -> Rect {
    Rect {
        x,
        y,
        width: w,
        height: h,
    }
}
impl Layout {
    // ------------------------=
    // FUNC: new
    // DESC: Reflows the real editor viewport when inline tools or the shared AI sidebar are open.
    // ------------------=
    pub fn new(window: Rect, scale: usize, assistant: bool, field: Field) -> Self {
        let s = scale.max(1) as u32;
        let right = if assistant {
            super::app_assistant::geometry(window, scale, true).panel.x
        } else {
            window.right() - s as i32
        };
        let width = (right - window.x - 1).max(1) as u32;
        let menu_bar = box_at(window.x + 1, window.y + (48 * s) as i32, width, 36 * s);
        let document = box_at(menu_bar.x, menu_bar.bottom(), width, 38 * s);
        let status = box_at(menu_bar.x, window.bottom() - (32 * s) as i32, width, 31 * s);
        let field_height = if field == Field::None { 0 } else { 64 * s };
        let body_y = document.bottom() + field_height as i32;
        let body = box_at(menu_bar.x, body_y, width, (status.y - body_y).max(1) as u32);
        let syntax = box_at(right - (164 * s) as i32, status.y, 156 * s, status.height);
        let search_width = (230 * s).min(window.width.saturating_sub(340 * s));
        let search = box_at(
            window.right() - (134 * s + search_width) as i32,
            window.y + (9 * s) as i32,
            search_width,
            30 * s,
        );
        let field = box_at(
            menu_bar.x + (16 * s) as i32,
            document.bottom() + (8 * s) as i32,
            width.saturating_sub(68 * s),
            48 * s,
        );
        let field_close = box_at(
            right - (40 * s) as i32,
            document.bottom() + (16 * s) as i32,
            24 * s,
            24 * s,
        );
        Self {
            window,
            menu_bar,
            document,
            body,
            status,
            syntax,
            search,
            field,
            field_close,
            scale,
        }
    }
    // ------------------------=
    // FUNC: document_tab
    // DESC: Gives the single document tab a stable, shared icon and title gutter.
    // ------------------=
    pub fn document_tab(self) -> Rect {
        box_at(
            self.document.x + 16 * self.scale as i32,
            self.document.y + 5 * self.scale as i32,
            (184 * self.scale as u32).min(self.document.width / 2),
            33 * self.scale as u32,
        )
    }
    // ------------------------=
    // FUNC: tab_close
    // DESC: Provides the hit area for guarded closing of the current document.
    // ------------------=
    pub fn tab_close(self) -> Rect {
        let tab = self.document_tab();
        box_at(
            tab.right() - 28 * self.scale as i32,
            tab.y,
            28 * self.scale as u32,
            tab.height,
        )
    }
    // ------------------------=
    // FUNC: new_document
    // DESC: Provides the hit area for the existing guarded New workflow.
    // ------------------=
    pub fn new_document(self) -> Rect {
        let tab = self.document_tab();
        box_at(
            tab.right() + 8 * self.scale as i32,
            tab.y,
            28 * self.scale as u32,
            tab.height,
        )
    }
    // ------------------------=
    // FUNC: menu_button
    // DESC: Keeps File, Edit, Selection and View hit targets identical to their painted locations.
    // ------------------=
    pub fn menu_button(self, menu: Menu) -> Rect {
        let (x, w) = match menu {
            Menu::File => (8, 48),
            Menu::Edit => (60, 44),
            Menu::Selection => (108, 92),
            Menu::View => (208, 56),
            _ => (0, 0),
        };
        box_at(
            self.menu_bar.x + x * self.scale as i32,
            self.menu_bar.y + 2 * self.scale as i32,
            w * self.scale as u32,
            32 * self.scale as u32,
        )
    }
    // ------------------------=
    // FUNC: toolbar_action
    // DESC: Places the kit's New, Open, and Save actions on the right side of the breadcrumb toolbar.
    // ------------------=
    pub fn toolbar_action(self, index: usize) -> Rect {
        let s = self.scale as i32;
        let width = 72 * s;
        box_at(
            self.menu_bar.right() - ((3 - index.min(2)) as i32 * width) - 6 * s,
            self.menu_bar.y + 3 * s,
            width as u32,
            30 * self.scale as u32,
        )
    }
    // ------------------------=
    // FUNC: popup
    // DESC: Anchors syntax above the status selector and other menus below the menu bar, clamped inside the app.
    // ------------------=
    pub fn popup(self, menu: Menu) -> Rect {
        let s = self.scale as u32;
        let w = (300 * s).min(self.menu_bar.width.saturating_sub(16 * s));
        let h = menu.count() as u32 * ROW_HEIGHT * s + 16 * s;
        let x = if menu == Menu::Syntax {
            self.syntax.right() - w as i32
        } else {
            self.menu_button(menu).x
        };
        let x = x
            .max(self.window.x + 8 * self.scale as i32)
            .min(self.body.right() - w as i32 - 8 * self.scale as i32);
        let y = if menu == Menu::Syntax {
            self.status.y - h as i32 - 4 * self.scale as i32
        } else {
            self.menu_bar.bottom()
        };
        box_at(x, y.max(self.menu_bar.y), w, h)
    }
    // ------------------------=
    // FUNC: menu_row
    // DESC: Supplies padded popup row geometry for hover, click and keyboard selection rendering.
    // ------------------=
    pub fn menu_row(self, menu: Menu, index: usize) -> Rect {
        let p = self.popup(menu);
        let s = self.scale as u32;
        box_at(
            p.x + 4 * self.scale as i32,
            p.y + (8 + index as u32 * ROW_HEIGHT) as i32 * self.scale as i32,
            p.width.saturating_sub(8 * s),
            ROW_HEIGHT * s,
        )
    }
    // ------------------------=
    // FUNC: row_at
    // DESC: Resolves clicks only within real menu rows, never the underlying document.
    // ------------------=
    pub fn row_at(self, menu: Menu, p: Point) -> Option<usize> {
        (0..menu.count()).find(|i| self.menu_row(menu, *i).contains(p))
    }
    // ------------------------=
    // FUNC: columns
    // DESC: Shares the code font's cell width with renderer, scrolling and pointer selection.
    // ------------------=
    pub fn columns(self) -> usize {
        (self.body.width as usize).saturating_sub((CODE_INSET + 16) * self.scale)
            / (super::editor_tools::CELL_WIDTH * self.scale)
    }
    // ------------------------=
    // FUNC: rows
    // DESC: Excludes document and status chrome from the code viewport.
    // ------------------=
    pub fn rows(self) -> usize {
        (self.body.height as usize).saturating_sub(16 * self.scale) / (LINE_HEIGHT * self.scale)
    }
    // ------------------------=
    // FUNC: scrollbar
    // DESC: Returns the real reflowed viewport's scrollbar rather than the obsolete toolbar-based track.
    // ------------------=
    pub fn scrollbar(
        self,
        total: usize,
        scroll: usize,
    ) -> super::system_layout::EditorScrollGeometry {
        let maximum_scroll = total.saturating_sub(self.rows().max(1));
        let s = self.scale as u32;
        let track = box_at(
            self.body.right() - (10 * s) as i32,
            self.body.y + (8 * s) as i32,
            6 * s,
            self.body.height.saturating_sub(16 * s),
        );
        let thumb_height = if maximum_scroll == 0 {
            0
        } else {
            ((track.height as usize * self.rows().max(1) / total.max(1)) as u32)
                .max(28 * s)
                .min(track.height)
        };
        let y = track.y
            + ((track.height - thumb_height) as usize * scroll.min(maximum_scroll)
                / maximum_scroll.max(1)) as i32;
        super::system_layout::EditorScrollGeometry {
            track,
            thumb: box_at(track.x, y, track.width, thumb_height),
            maximum_scroll,
        }
    }
}
