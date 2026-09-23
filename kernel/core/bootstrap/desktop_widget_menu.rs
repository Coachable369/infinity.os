//! Shared native glass chooser, also exercised by the host pixel harness.
use super::*;
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
impl DisplayDevice {
    // ------------------------=
    // FUNC: desktop_widget_menu
    // DESC: Draws the glass desktop chooser with checked visibility rows above application layers.
    // ------------------=
    pub(super) fn desktop_widget_menu(&mut self, scale: usize) {
        let state = crate::ui::desktop_widgets::current();
        let Some(rect) = state.menu_rect(self.width, self.height, scale) else {
            return;
        };
        let (x, y) = (rect.x.max(0) as usize, rect.y.max(0) as usize);
        self.glass_panel(x, y, rect.width as usize, rect.height as usize, true);
        self.ui_text_strong(x + 16 * scale, y + 10 * scale, b"WIDGETS", 120, 209, 245, 1);
        for (i, label) in [
            b"System Overview".as_slice(),
            b"AI Chat",
            b"Reset positions",
        ]
        .iter()
        .enumerate()
        {
            let top = y + (46 + i * 36) * scale;
            self.ui_text(x + 40 * scale, top, label, 222, 237, 249, 1);
            if i < 2 && state.visible & (1 << i) != 0 {
                let (a, b) = ((x + 15 * scale) as i32, (top + 7 * scale) as i32);
                self.icon_line(
                    a,
                    b,
                    a + 4 * scale as i32,
                    b + 4 * scale as i32,
                    (105, 219, 255),
                    12 * scale,
                );
                self.icon_line(
                    a + 4 * scale as i32,
                    b + 4 * scale as i32,
                    a + 12 * scale as i32,
                    b - 4 * scale as i32,
                    (105, 219, 255),
                    12 * scale,
                );
            }
        }
    }
}
