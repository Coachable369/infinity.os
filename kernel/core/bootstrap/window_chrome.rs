//! Shared native glass window controls; geometry remains owned by SystemLayout.
impl super::DisplayDevice {
    // ------------------------=
    // FUNC: window_control
    // DESC: Paints a high-contrast, centered native control with a restore glyph when maximized.
    // ------------------=
    pub(super) fn window_control(
        &mut self,
        x: usize,
        y: usize,
        size: usize,
        index: usize,
        maximized: bool,
    ) {
        let unit = (size / 20).max(1);
        self.fill_rounded_rect_alpha(x, y, size, size, size / 4, 11, 31, 48, 235);
        self.outline_rounded_rect(x, y, size, size, size / 4, 88, 139, 168);
        let cx = (x + size / 2) as i32;
        let cy = (y + size / 2) as i32;
        let r = (size / 4) as i32;
        let color = (226, 242, 252);
        match index {
            0 => self.icon_line(cx - r, cy, cx + r, cy, color, size),
            1 => {
                let shift = if maximized { 2 * unit } else { 0 };
                if maximized {
                    self.outline_rect(
                        (cx - r) as usize + shift,
                        (cy - r) as usize - shift,
                        (2 * r) as usize,
                        (2 * r) as usize,
                        156,
                        203,
                        230,
                    );
                }
                for inset in 0..unit {
                    self.outline_rect(
                        (cx - r) as usize + inset,
                        (cy - r) as usize + inset,
                        (2 * r) as usize - 2 * inset,
                        (2 * r) as usize - 2 * inset,
                        color.0,
                        color.1,
                        color.2,
                    );
                }
            }
            _ => {
                self.icon_line(cx - r, cy - r, cx + r, cy + r, color, size);
                self.icon_line(cx + r, cy - r, cx - r, cy + r, color, size);
            }
        }
    }
}
