//! Native launcher artwork shared with Studio's production preview.
use super::*;
impl DisplayDevice {

    #[cfg(all(
        not(feature = "installer"),
        any(target_arch = "x86_64", target_arch = "aarch64")
    ))]
    // ------------------------=
    // FUNC: launcher_icon
    // DESC: Renders a launcher role from its compact 256px-per-cell installed-theme atlas.
    // ------------------=
    pub(crate) fn launcher_icon(
        &mut self,
        center_x: usize,
        center_y: usize,
        role: usize,
        size: usize,
    ) -> bool {
        #[cfg(feature="native-browser")]
        if role==60 {
            self.paint_bitmap_alpha_fit_rect(include_bytes!("../../../assets/apps/infinity-browser-icon-v1.bmp"),
                center_x.saturating_sub(size/2),center_y.saturating_sub(size/2),size,size);
            return true;
        }
        if (57..=59).contains(&role) {
            self.spatial_identity_icon(center_x, center_y, size, role - 57);
            return true;
        }
        let roles = [0usize, 1, 4, 8, 9, 10, 12, 19, 23, 25, 26, 28, 32, 49];
        let Some(cell) = roles.iter().position(|candidate| *candidate == role) else {
            return self.themed_icon(center_x, center_y, role, size);
        };
        let bitmap = match self.active_icon_theme() {
            1 => LUMINOUS_OBSIDIAN_LAUNCHER_BMP,
            2 => FROSTED_QUARTZ_LAUNCHER_BMP,
            3 => AURORA_HARMONY_LAUNCHER_BMP,
            _ => CRYSTAL_BLUE_GLASS_LAUNCHER_BMP,
        };
        self.paint_bitmap_alpha_atlas_cell(
            bitmap,
            4,
            4,
            cell,
            center_x.saturating_sub(size / 2),
            center_y.saturating_sub(size / 2),
            size,
        )
    }

    #[cfg(any(feature = "installer", target_arch = "x86"))]
    // ------------------------=
    // FUNC: launcher_icon
    // DESC: Uses the existing vector fallback where installed 256px launcher atlases are unavailable.
    // ------------------=
    pub(crate) fn launcher_icon(
        &mut self,
        center_x: usize,
        center_y: usize,
        role: usize,
        size: usize,
    ) -> bool {
        if (57..=59).contains(&role) {
            self.spatial_identity_icon(center_x, center_y, size, role - 57);
            true
        } else {
            self.themed_icon(center_x, center_y, role, size)
        }
    }

    // ------------------------=
    // FUNC: spatial_identity_icon
    // DESC: Draws the spatial identity and AI-authored Holographic Desktop and World Shift artwork.
    // ------------------=
    fn spatial_identity_icon(&mut self, center_x: usize, center_y: usize, size: usize, kind: usize) {
        let left = center_x.saturating_sub(size / 2);
        let top = center_y.saturating_sub(size / 2);
        if kind == 1 {
            self.paint_bitmap_alpha_fit_rect(include_bytes!("../../../assets/apps/infinity-holographic-desktop-icon-v1.bmp"),
                left,top,size,size);
            return;
        }
        if kind == 2 {
            self.paint_bitmap_alpha_fit_rect(include_bytes!("../../../assets/apps/infinity-world-shift-icon-v1.bmp"),
                left,top,size,size);
            return;
        }
        let radius = (size / 5).max(5);
        let palette = [(12, 65, 105), (25, 52, 108), (34, 38, 104)][kind.min(2)];
        self.fill_rounded_rect_alpha(left, top, size, size, radius, palette.0, palette.1, palette.2, 248);
        self.outline_rounded_rect(left, top, size, size, radius, 112, 224, 255);
        let stroke: (u8, u8, u8) = (173, 240, 255);
        if kind == 0 {
            for ring in 0..3 {
                let inset = size * (18 + ring * 10) / 100;
                self.outline_rounded_rect(left + inset, top + inset, size - inset * 2, size - inset * 2,
                    size / 2, stroke.0, stroke.1.saturating_sub((ring * 28) as u8), stroke.2);
            }
            self.fill_rounded_rect_alpha(center_x.saturating_sub(size / 12), center_y.saturating_sub(size / 12), size / 6, size / 6, size / 12, 235, 252, 255, 255);
        }
    }
}
