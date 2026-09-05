//! Resolution-aware geometry for the fatal incident screen.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CrashLayout {
    pub gutter: usize,
    pub content_width: usize,
    pub flag_size: usize,
    pub flag_left: usize,
    pub flag_top: usize,
    pub title_y: usize,
    pub panel_top: usize,
    pub panel_height: usize,
}

impl CrashLayout {
    // ------------------------=
    // FUNC: new
    // DESC: Separates emblem, heading, and incident panel with bounded resolution-aware gutters.
    // ------------------=
    pub fn new(width: usize, height: usize, title_scale: usize) -> Self {
        let gutter = (width / 18).max(28);
        let content_width = width.saturating_sub(gutter * 2);
        let flag_size = (height * 22 / 100)
            .min(width * 18 / 100)
            .max(112)
            .min(height.saturating_sub(240));
        let flag_left = width.saturating_sub(flag_size) / 2;
        let flag_top = (height * 3 / 100).max(18);
        let title_y = flag_top + flag_size + 14;
        let heading_clearance = if title_scale > 1 { 62 } else { 46 };
        let panel_top = title_y + heading_clearance;
        let bottom_gutter = (height / 18).max(24);
        let panel_height = height.saturating_sub(panel_top + bottom_gutter);
        Self {
            gutter,
            content_width,
            flag_size,
            flag_left,
            flag_top,
            title_y,
            panel_top,
            panel_height,
        }
    }
}
