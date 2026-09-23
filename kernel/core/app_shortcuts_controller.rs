//! Native app copies, drag-through launcher activation, and durable catalog ordering.
use super::*;
use crate::ui::app_launcher::{self as launcher, shortcuts};
use crate::ui::geometry::Point;
impl ConsoleRuntime {
    // ------------------------=
    // FUNC: save_app_shortcuts
    // DESC: Commits desktop copies and launcher ordering after release, never during pointer motion.
    // ------------------=
    pub(super) fn save_app_shortcuts(&mut self) {
        if self.current_user.is_zero() {
            return;
        }
        let bytes = shortcuts::current().encode(launcher::launcher_presentation().order);
        let result = crate::storage::launcher_preferences::save(
            self.current_user.0,
            self.current_session.0,
            &bytes,
        );
        if result.is_err() {
            self.output
                .write_line(b"Unable to save launcher layout; changes remain in this session.");
        }
    }
    // ------------------------=
    // FUNC: restore_app_shortcuts
    // DESC: Restores validated per-user state and clears stale captures across login boundaries.
    // ------------------=
    pub(super) fn restore_app_shortcuts(&mut self) {
        shortcuts::publish(shortcuts::State::new());
        launcher::launcher_restore_default_order();
        if self.current_user.is_zero() {
            return;
        }
        let mut bytes = [0; shortcuts::STATE_BYTES];
        if let Ok(length) = crate::storage::launcher_preferences::load(
            self.current_user.0,
            self.current_session.0,
            &mut bytes,
        ) {
            if let Some((state, order)) = shortcuts::State::decode(&bytes[..length]) {
                shortcuts::publish(state);
                launcher::encode_launcher_order(&order);
            }
        }
    }
    // ------------------------=
    // FUNC: app_shortcuts_pointer
    // DESC: Captures app drags across surfaces, springs open the dock launcher and resolves release exactly once.
    // ------------------=
    pub(super) fn app_shortcuts_pointer(&mut self, clicked: bool, released: bool) -> bool {
        if !matches!(
            self.mode,
            ConsoleMode::Desktop | ConsoleMode::Settings | ConsoleMode::AppLauncher
        ) {
            return false;
        }
        let layout = SystemLayout::new(
            self.system.framebuffer_width,
            self.system.framebuffer_height,
        );
        let mut state = shortcuts::current();
        let point = Point {
            x: self.system.framebuffer_width as i32 * self.pointer_x / 1000,
            y: self.system.framebuffer_height as i32 * self.pointer_y / 1000,
        };
        if state.drag.is_none() && clicked && self.mode != ConsoleMode::AppLauncher {
            // Application windows own covered pixels, not desktop shortcuts beneath them.
            let (e, c, t) = self.desktop_app_windows();
            let rect = |w: DesktopAppWindowState| {
                layout
                    .desktop_app_window_geometry(w.x, w.y, w.width, w.height, w.maximized)
                    .window
            };
            let (x, y, w, h) = layout.home_window_geometry_sized(
                self.home_window_x,
                self.home_window_y,
                self.home_window_width,
                self.home_window_height,
                self.home_window_maximized,
            );
            let home = crate::ui::geometry::Rect {
                x: x as i32,
                y: y as i32,
                width: w as u32,
                height: h as u32,
            };
            if crate::ui::desktop_stack::current()
                .hit(
                    [
                        home,
                        rect(c),
                        rect(e),
                        rect(t),
                        layout.settings_window_geometry(self.settings_window).window,
                    ],
                    point,
                )
                .is_some()
                || self.inactive_file_navigator_at_pointer().is_some()
            {
                return false;
            }
            for id in (0..state.positions.len()).rev() {
                if shortcuts::icon_rect(
                    state.positions[id],
                    self.system.framebuffer_width,
                    self.system.framebuffer_height,
                )
                .is_some_and(|(x, y, w, h)| {
                    point.x >= x as i32
                        && point.y >= y as i32
                        && point.x < (x + w) as i32
                        && point.y < (y + h) as i32
                }) {
                    state.begin(id, self.pointer_x, self.pointer_y, false);
                    shortcuts::publish(state);
                    return true;
                }
            }
        }
        let Some((id, _, _, _)) = state.drag else {
            return false;
        };
        state.motion(self.pointer_x, self.pointer_y);
        let moved = state.drag.unwrap().3;
        let target = layout.app_launcher_target(
            self.pointer_x,
            self.pointer_y,
            launcher::LAUNCHER_APPS.len(),
        );
        if moved
            && self.pointer_pressed
            && target == AppLauncherTarget::DockToggle
            && self.mode != ConsoleMode::AppLauncher
        {
            self.open_app_launcher();
        }
        if self.mode == ConsoleMode::AppLauncher && moved {
            let geometry = layout.app_launcher_geometry();
            let scroll = layout.app_launcher_scroll_geometry(launcher::LAUNCHER_APPS.len());
            let edge = (28 * layout.scale()) as i32;
            if self.pointer_pressed
                && geometry.grid_viewport.contains(point)
                && point.y < geometry.grid_viewport.y + edge
            {
                launcher::launcher_scroll_by(-(12 * layout.scale() as i32), scroll.maximum_scroll);
            } else if self.pointer_pressed
                && geometry.grid_viewport.contains(point)
                && point.y > geometry.grid_viewport.bottom() - edge
            {
                launcher::launcher_scroll_by(12 * layout.scale() as i32, scroll.maximum_scroll);
            }
            let source = launcher::launcher_presentation()
                .order
                .iter()
                .position(|v| *v as usize == id)
                .unwrap();
            if launcher::launcher_presentation().drag_source.is_none() {
                launcher::launcher_begin_drag(source, state.drag.unwrap().1, state.drag.unwrap().2);
            }
            launcher::launcher_update_drag(
                match target {
                    AppLauncherTarget::App(i) => Some(i),
                    _ => None,
                },
                self.pointer_x,
                self.pointer_y,
            );
        }
        if released {
            if !moved {
                self.activate_launcher_action(launcher::LAUNCHER_APPS[id].action);
            } else if self.mode == ConsoleMode::AppLauncher
                && matches!(target, AppLauncherTarget::App(_))
            {
                let _ = launcher::launcher_finish_drag(b"");
            } else if target == AppLauncherTarget::Dismiss || self.mode != ConsoleMode::AppLauncher
            {
                state.place(id, self.pointer_x, self.pointer_y);
            }
            shortcuts::cancel_launcher_drag();
            state.drag = None;
            shortcuts::publish(state);
            if moved {
                self.save_app_shortcuts();
            }
        } else {
            shortcuts::publish(state);
        }
        self.present_continuous_motion(released);
        true
    }
}
