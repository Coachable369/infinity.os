//! Theme-aware desktop copies and their pointer-following copy badge.
use super::*;
impl DisplayDevice {
    // ------------------------=
    // FUNC: desktop_app_shortcut
    // DESC: Uses installed high-resolution icon artwork for shortcuts and the copy-badged drag preview.
    // ------------------=
    pub(super) fn desktop_app_shortcut(&mut self, id: usize, x: usize, y: usize, copy: bool) {
        let entry = crate::ui::app_launcher::LAUNCHER_APPS[id];
        let size = (self.height / 21).max(44);
        self.launcher_icon(x + size / 2, y + size / 2, entry.icon_role, size);
        self.ui_text(x, y + size + 8, entry.label, 221, 235, 243, 1);
        if copy {
            self.fill_rounded_rect_alpha(
                x + size - 12,
                y + size - 12,
                20,
                20,
                10,
                23,
                124,
                187,
                240,
            );
            self.ui_text(x + size - 8, y + size - 13, b"+", 255, 255, 255, 1);
        }
    }
}
