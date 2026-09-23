//! Native desktop widget header capture and chooser dispatch.
use super::*;
use crate::ui::{
    desktop_widgets as widgets,
    geometry::{Point, Rect},
};
impl ConsoleRuntime {
    // ------------------------=
    // FUNC: desktop_widgets_pointer
    // DESC: Moves exposed widgets without capturing covered application input; persists only completed edits.
    // ------------------=
    pub(super) fn desktop_widgets_pointer(&mut self, clicked: bool, secondary: bool) -> bool {
        if !matches!(self.mode, ConsoleMode::Desktop | ConsoleMode::Settings)
            || self.editor_dialog != EditorDialog::None
            || self.app_window_dragging
            || self.app_window_resizing.is_some()
            || self.home_window_dragging
            || self.home_window_resizing.is_some()
            || self.settings_window_dragging
            || self.settings_window_resizing.is_some()
        {
            return false;
        }
        let (width, height) = (
            self.system.framebuffer_width,
            self.system.framebuffer_height,
        );
        let layout = SystemLayout::new(width, height);
        let scale = layout.scale();
        let point = Point {
            x: self.pointer_x * width as i32 / 1000,
            y: self.pointer_y * height as i32 / 1000,
        };
        let mut state = widgets::current();
        let minimized = crate::runtime::ai::with_ai_runtime(|r| r.chat.minimized());
        if state.drag.is_some() {
            let old = state;
            state.move_pointer(point, self.pointer_pressed, width, height, scale, minimized);
            widgets::publish(state);
            if state.drag.is_none() {
                let _ = self.checkpoint_desktop_layout();
            }
            if state != old {
                self.redraw();
            } else {
                crate::bootstrap::system_ui_cursor(self.pointer_x, self.pointer_y);
            }
            return true;
        }
        if let Some(menu) = state.menu_rect(width, height, scale) {
            if clicked {
                if menu.contains(point) {
                    let row = (point.y - menu.y) as usize / (36 * scale);
                    if row == 1 || row == 2 {
                        state.visible ^= 1 << (row - 1);
                        if row == 2 && state.visible & 2 != 0 {
                            self.set_ai_chat_enabled(true);
                        }
                        self.ai_chat_focus = 0;
                    } else if row == 3 {
                        state.positions = [[0; 2]; 2];
                    }
                }
                state.menu = None;
                widgets::publish(state);
                let _ = self.checkpoint_desktop_layout();
                self.redraw();
            } else {
                crate::bootstrap::system_ui_cursor(self.pointer_x, self.pointer_y);
            }
            return true;
        }
        // Desktop widgets are below windows: never drag through an application.
        let (e, c, t) = self.desktop_app_windows();
        let app_rect = |s: DesktopAppWindowState| {
            layout
                .desktop_app_window_geometry(s.x, s.y, s.width, s.height, s.maximized)
                .window
        };
        let (x, y, w, h) = layout.home_window_geometry_sized(
            self.home_window_x,
            self.home_window_y,
            self.home_window_width,
            self.home_window_height,
            self.home_window_maximized,
        );
        let home = Rect {
            x: x as i32,
            y: y as i32,
            width: w as u32,
            height: h as u32,
        };
        if self.inactive_file_navigator_at_pointer().is_some()
            || crate::ui::desktop_stack::current()
                .hit(
                    [
                        home,
                        app_rect(c),
                        app_rect(e),
                        app_rect(t),
                        layout.settings_window_geometry(self.settings_window).window,
                    ],
                    point,
                )
                .is_some()
        {
            return false;
        }
        if point.y < (height * 7 / 100) as i32
            || layout.desktop_foreground_geometry().dock.contains(point)
        {
            return false;
        }
        if secondary {
            state.menu = Some(point);
            self.shell_menu = 0;
            widgets::publish(state);
            self.redraw();
            return true;
        }
        if clicked {
            for id in (0..2).rev() {
                if state.visible & (1 << id) == 0 {
                    continue;
                }
                let rect = state.rect(id, width, height, scale, minimized && id == 1);
                if rect.contains(point) {
                    if point.y < rect.y + (40 * scale) as i32
                        && point.x < rect.right() - (80 * scale) as i32
                    {
                        state.drag = Some((id, point.x - rect.x, point.y - rect.y));
                        self.ai_chat_focus = 0;
                        widgets::publish(state);
                        return true;
                    }
                    break;
                }
            }
        }
        false
    }
}
