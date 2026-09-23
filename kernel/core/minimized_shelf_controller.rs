//! Routes shelf actions to existing native lifecycle operations, including dirty-document guards.
use super::*;
use crate::ui::app_launcher::minimized_shelf::{self as shelf, Action, Geometry};
use crate::ui::geometry::Point;
const _: () =
    assert!(shelf::NAVIGATORS == crate::runtime::object_navigation::MAX_FILE_NAVIGATOR_INSTANCES);

impl ConsoleRuntime {
    // ------------------------=
    // FUNC: shelf_app_id
    // DESC: Maps the active singleton window to its stable minimized identity.
    // ------------------=
    pub(super) fn shelf_app_id(&self) -> Option<usize> {
        match self.desktop_app {
            DesktopAppKind::CommandWindow => Some(shelf::COMMAND),
            DesktopAppKind::TextEditor => Some(shelf::EDITOR),
            DesktopAppKind::TaskManager => Some(shelf::TASKS),
            _ => None,
        }
    }
    // ------------------------=
    // FUNC: publish_minimized_shelf
    // DESC: Publishes hidden navigator identities and removes visible singleton windows from the shelf.
    // ------------------=
    pub(super) fn publish_minimized_shelf(&self) {
        let mut state = shelf::current();
        let _ = crate::runtime::with_runtime(|runtime| {
            for id in 0..shelf::NAVIGATORS {
                state.set(
                    id,
                    runtime
                        .file_navigators
                        .window(id)
                        .map(|w| !w.visible)
                        .unwrap_or(false),
                );
            }
        });
        let (e, c, t) = self.desktop_app_windows();
        for (id, visible) in [
            (shelf::EDITOR, e.visible),
            (shelf::COMMAND, c.visible),
            (shelf::TASKS, t.visible),
            (shelf::SETTINGS, self.settings_open),
        ] {
            if visible {
                state.set(id, false);
            }
        }
        if !matches!(self.mode, ConsoleMode::Desktop | ConsoleMode::Settings) {
            state.menu = None;
            state.row = None;
            state.hover = None;
        }
        let geometry = Geometry::new(
            self.system.framebuffer_width,
            self.system.framebuffer_height,
            state,
        );
        state.offset = state
            .offset
            .min(state.count().saturating_sub(geometry.capacity));
        shelf::publish(state);
    }
    // ------------------------=
    // FUNC: shelf_available
    // DESC: Keeps modal documents, spatial overlays, and ongoing drag capture above the shelf.
    // ------------------=
    fn shelf_available(&self) -> bool {
        matches!(self.mode, ConsoleMode::Desktop | ConsoleMode::Settings)
            && self.editor_dialog == EditorDialog::None
            && !self.spatial.open
            && !self.home_window_dragging
            && self.home_window_resizing.is_none()
            && !self.app_window_dragging
            && self.app_window_resizing.is_none()
            && !self.settings_window_dragging
            && self.settings_window_resizing.is_none()
    }
    // ------------------------=
    // FUNC: minimized_shelf_pointer
    // DESC: Consumes shelf clicks without leaking them into covered windows; unchanged motion is cursor-only.
    // ------------------=
    pub(super) fn minimized_shelf_pointer(&mut self, clicked: bool, secondary: bool) -> bool {
        if !self.shelf_available() {
            return false;
        }
        let old = shelf::current();
        let mut state = old;
        let g = Geometry::new(
            self.system.framebuffer_width,
            self.system.framebuffer_height,
            state,
        );
        let point = Point {
            x: (self.pointer_x as i64 * self.system.framebuffer_width as i64 / 1000) as i32,
            y: (self.pointer_y as i64 * self.system.framebuffer_height as i64 / 1000) as i32,
        };
        let mut consumed = g.rail.contains(point);
        if state.drag.is_some() {
            let rail_width =
                g.rail.width as i32 * 1000 / self.system.framebuffer_width.max(1) as i32;
            state.move_drag(
                self.pointer_x,
                self.pointer_y,
                self.pointer_pressed,
                rail_width,
            );
            shelf::publish(state);
            if state.drag.is_none() {
                let _ = self.checkpoint_desktop_layout();
            }
            if state != old {
                self.redraw();
            }
            return true;
        }
        if clicked
            && state.menu.is_none()
            && consumed
            && point.y < g.rail.y + (g.rail.width / 2) as i32
        {
            let x = g.rail.x * 1000 / self.system.framebuffer_width.max(1) as i32;
            let y = g.rail.y * 1000 / self.system.framebuffer_height.max(1) as i32;
            state.drag = Some((x, y, self.pointer_x - x, self.pointer_y - y));
            state.hover = None;
            shelf::publish(state);
            self.redraw();
            return true;
        }
        if let Some(id) = state.menu {
            consumed = true;
            state.row = g.menu_row(point);
            if clicked {
                state.menu = None;
                shelf::publish(state);
                if let Some(row) = state.row {
                    self.shelf_action(id, [Action::Restore, Action::Maximize, Action::Close][row]);
                }
                self.redraw();
                return true;
            }
            if secondary {
                state.menu = None;
                state.row = None;
            }
        }
        state.hover = g.hit(state, point);
        if let Some(id) = state.hover {
            if secondary {
                state.menu = Some(id);
                state.row = Some(0);
            } else if clicked {
                shelf::publish(state);
                self.shelf_action(id, Action::Restore);
                self.redraw();
                return true;
            }
        } else if clicked && consumed && state.count() > g.capacity {
            state.scroll(
                if point.y < g.rail.y + (g.rail.width / 2) as i32 {
                    -1
                } else {
                    1
                },
                g.capacity,
            );
        }
        shelf::publish(state);
        if state != old {
            self.redraw();
        } else if consumed {
            crate::bootstrap::system_ui_cursor(self.pointer_x, self.pointer_y);
        }
        consumed
    }
    // ------------------------=
    // FUNC: minimized_shelf_scroll
    // DESC: Makes overflowing minimized windows reachable with the wheel and bounded repainting.
    // ------------------=
    pub(super) fn minimized_shelf_scroll(&mut self, vertical: i8) -> bool {
        if !self.shelf_available() || vertical == 0 {
            return false;
        }
        let mut state = shelf::current();
        let g = Geometry::new(
            self.system.framebuffer_width,
            self.system.framebuffer_height,
            state,
        );
        let point = Point {
            x: (self.pointer_x as i64 * self.system.framebuffer_width as i64 / 1000) as i32,
            y: (self.pointer_y as i64 * self.system.framebuffer_height as i64 / 1000) as i32,
        };
        if !g.rail.contains(point) {
            return false;
        }
        state.scroll(
            crate::ui::input_preferences::current()
                .wheel(vertical)
                .signum(),
            g.capacity,
        );
        shelf::publish(state);
        self.redraw();
        true
    }
    // ------------------------=
    // FUNC: minimized_shelf_key
    // DESC: Supports Escape, arrows, and Enter for an open native context menu.
    // ------------------=
    pub(super) fn minimized_shelf_key(&mut self, key: ConsoleKey) -> bool {
        if !self.shelf_available() {
            return false;
        }
        let mut state = shelf::current();
        let Some(id) = state.menu else {
            return false;
        };
        match key {
            ConsoleKey::Escape => {
                state.menu = None;
                state.row = None;
            }
            ConsoleKey::Up => state.row = Some((state.row.unwrap_or(0) + 2) % 3),
            ConsoleKey::Down => state.row = Some((state.row.unwrap_or(0) + 1) % 3),
            ConsoleKey::Enter => {
                let action =
                    [Action::Restore, Action::Maximize, Action::Close][state.row.unwrap_or(0)];
                state.menu = None;
                shelf::publish(state);
                self.shelf_action(id, action);
                self.redraw();
                return true;
            }
            _ => return true,
        }
        shelf::publish(state);
        self.redraw();
        true
    }
    // ------------------------=
    // FUNC: shelf_action
    // DESC: Restores the exact window before applying existing maximize or guarded close behavior.
    // ------------------=
    fn shelf_action(&mut self, id: usize, action: Action) {
        if shelf::current().mask & (1 << id) == 0 {
            return;
        }
        self.ai_chat_focus = 0;
        self.shell_menu = 0;
        self.store_active_app_window();
        if id < shelf::NAVIGATORS {
            self.checkpoint_active_file_navigator();
            let restored = crate::runtime::with_runtime(|r| r.file_navigators.restore(id))
                .flatten()
                .is_some();
            if !restored {
                return;
            }
            self.mode = ConsoleMode::Desktop;
            if !self.load_file_navigator_window(id) {
                return;
            }
            if action == Action::Close {
                let next = crate::runtime::with_runtime(|r| {
                    r.file_navigators.close_active();
                    r.file_navigators.active_index()
                })
                .flatten();
                if let Some(next) = next {
                    self.load_file_navigator_window(next);
                } else {
                    self.home_window_visible = false;
                }
            } else if action == Action::Maximize && !self.home_window_maximized {
                self.toggle_window_maximized(0);
            }
        } else if id == shelf::SETTINGS {
            self.mode = ConsoleMode::Settings;
            self.settings_open = true;
            self.system_focus = crate::ui::desktop_stack::current().settings_section;
            if action == Action::Maximize {
                self.settings_window.maximized = true;
            }
            if action == Action::Close {
                self.settings_open = false;
                self.enter_desktop();
            }
        } else {
            self.mode = ConsoleMode::Desktop;
            let app = match id {
                shelf::COMMAND => {
                    self.command_window.visible = true;
                    self.command_window_suspended = false;
                    DesktopAppKind::CommandWindow
                }
                shelf::EDITOR => {
                    self.editor_window.visible = true;
                    DesktopAppKind::TextEditor
                }
                _ => {
                    self.task_manager_window.visible = true;
                    DesktopAppKind::TaskManager
                }
            };
            self.focus_desktop_app(app);
            if action == Action::Maximize && !self.app_window_maximized {
                self.toggle_window_maximized(1);
            }
            if action == Action::Close {
                self.close_desktop_app();
            }
        }
        shelf::set(id, false);
        let mut state = shelf::current();
        state.menu = None;
        state.hover = None;
        state.row = None;
        shelf::publish(state);
        let _ = self.checkpoint_desktop_layout();
    }
}
