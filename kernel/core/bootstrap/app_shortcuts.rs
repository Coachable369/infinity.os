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
        let width = size + 120;
        let center = x + width / 2;
        self.launcher_icon(center, y + size / 2, entry.icon_role, size);
        // Measure the final elided label, not the unbounded original string.
        let mut label = [0u8; 64];
        let mut length = entry.label.len().min(label.len());
        label[..length].copy_from_slice(&entry.label[..length]);
        if self.ui_text_width(&label[..length], 1) > width - 8 {
            while length > 0
                && self.ui_text_width(&label[..length], 1) + self.ui_text_width(b"...", 1)
                    > width - 8
            {
                length -= 1;
            }
            label[length..length + 3].copy_from_slice(b"...");
            length += 3;
        }
        self.ui_text_centered(x, width, y + size + 8, &label[..length], 221, 235, 243, 1);
        if copy {
            self.fill_rounded_rect_alpha(
                center + size / 2 - 12,
                y + size - 12,
                20,
                20,
                10,
                23,
                124,
                187,
                240,
            );
            self.ui_text(center + size / 2 - 8, y + size - 13, b"+", 255, 255, 255, 1);
        }
    }
}
