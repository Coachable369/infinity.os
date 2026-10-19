//! Reusable Eclipse Access controls; no authentication state or service calls in paint.
use super::{access_style as palette, DisplayDevice};

impl DisplayDevice {
    // ------------------------=
    // FUNC: access_surface
    // DESC: Paints a palette-controlled glass surface with antialiased edges and a stable focus rim.
    // ------------------=
    pub(super) fn access_surface(&mut self, left: usize, top: usize, width: usize,
        height: usize, radius: usize, fill: palette::Rgb, border: palette::Rgb,
        opacity: u8, focused: bool) {
        self.access_surface_alpha(left, top, width, height, radius, fill, border,
            opacity, 255, focused);
    }

    // ------------------------=
    // FUNC: access_surface_alpha
    // DESC: Preserves independent authored fill and border opacity while retaining the shared antialiased access recipe.
    // ------------------=
    pub(super) fn access_surface_alpha(&mut self, left: usize, top: usize, width: usize,
        height: usize, radius: usize, fill: palette::Rgb, border: palette::Rgb,
        opacity: u8, border_opacity: u8, focused: bool) {
        if opacity == 0 && border_opacity == 0 { return; }
        let Some(region) = self.clipped_render_region(left, top, width, height) else { return; };
        let primary = fill == palette::PRIMARY;
        let focus = if focused { palette::focus_amount() } else { 0 };
        for y in region.top..region.bottom {
            let row = y - top;
            let lift = ((height.saturating_sub(row) * 20) / height.max(1)) as u8;
            let mut color = if primary {
                let t = (row * 255 / height.max(1)) as u8;
                palette::mix((49, 174, 255), (22, 62, 220), t)
            } else { palette::mix(fill, palette::BORDER, lift) };
            if primary && focused && palette::pressed() { color = palette::mix(color, palette::ELEVATED, 56); }
            for x in region.left..region.right {
                let col = x - left;
                let outer = palette::rounded_coverage(col, row, width, height, radius);
                let inner = if col > 0 && row > 0 {
                    palette::rounded_coverage(col-1, row-1, width.saturating_sub(2),
                        height.saturating_sub(2), radius.saturating_sub(1))
                } else { 0 };
                let edge = outer.saturating_sub(inner);
                let edge_color = palette::mix(border, palette::FOCUS, focus);
                let distance = col.min(row).min(width-1-col).min(height-1-row);
                let glow = if distance < 5 { (5-distance) as u16 * focus as u16 / 20 } else { 0 } as u8;
                let shaded = palette::mix(color, palette::BORDER, glow);
                self.blend_color_unchecked(x, y, shaded.0, shaded.1, shaded.2,
                    (inner as u16 * opacity as u16 / 255) as u8);
                self.blend_color_unchecked(x, y, edge_color.0, edge_color.1, edge_color.2,
                    (edge as u16 * border_opacity as u16 / 255) as u8);
            }
        }
        self.mark_dirty_rect(region.left, region.top, region.right-region.left, region.bottom-region.top);
    }

    // ------------------------=
    // FUNC: access_wordmark
    // DESC: Fits the original transparent header lettering with alpha-filtered edges, without its unused canvas margins.
    // ------------------=
    pub(super) fn access_wordmark(&mut self, left: usize, top: usize, width: usize, height: usize) {
        #[cfg(any(target_arch="aarch64", target_arch="x86_64"))]
        self.paint_bitmap_template_rect(super::bootstrap::SPLASH_BMP, left, top, width,
            height, [14, 1, 12, 80], 255, false);
    }

    // ------------------------=
    // FUNC: access_panel
    // DESC: Uses the same pre-session palette for login cards, utility trays and fallback setup panels.
    // ------------------=
    pub(super) fn access_panel(&mut self, left: usize, top: usize, width: usize, height: usize) {
        self.access_surface(left, top, width, height, 18, palette::SURFACE,
            palette::MUTED_BORDER, 240, false);
    }

    // ------------------------=
    // FUNC: access_button
    // DESC: Applies the same recipe to fallback setup actions without changing their native hit rectangles.
    // ------------------=
    pub(super) fn access_button(&mut self,left:usize,top:usize,width:usize,height:usize,label:&[u8],primary:bool,focused:bool) {
        self.access_surface(left,top,width,height,(height/4).min(14),
            if primary { palette::PRIMARY } else { palette::BACKGROUND },
            if primary { palette::BORDER } else { palette::MUTED_BORDER },255,focused);
        let padding = (height / 3).min(width / 4);
        self.access_label((left + padding, top, width.saturating_sub(padding * 2), height),
            label, (height * 5 / 12).max(8), palette::TEXT, true, true);
    }

    // ------------------------=
    // FUNC: access_icon
    // DESC: Samples the generated Eclipse glyph mask with area-filtered alpha and the shared silver/violet tokens.
    // ------------------=
    pub(super) fn access_icon(&mut self, cx: usize, cy: usize, kind: usize, size: usize, active: bool) {
        const ATLAS: &[u8; 16*64*64*4] = include_bytes!("../../../assets/ui-design-kit/default/eclipse-access-glyphs-v1.rgba");
        let Some(&glyph) = [0usize,4,2,3,14,4,14,7,9,9,0,11,13,14,5,8].get(kind.wrapping_sub(1)) else { return; };
        if size == 0 { return; }
        let left = cx.saturating_sub(size/2); let top = cy.saturating_sub(size/2);
        let Some(region) = self.clipped_render_region(left, top, size, size) else { return; };
        let color = if active { palette::mix(palette::TEXT, palette::FOCUS, palette::focus_amount()) } else { palette::TEXT };
        for y in region.top..region.bottom { for x in region.left..region.right {
            let x0=(x-left)*64/size; let y0=(y-top)*64/size;
            let x1=(((x-left+1)*64/size).max(x0+1)).min(64);
            let y1=(((y-top+1)*64/size).max(y0+1)).min(64);
            let mut alpha=0u32;
            for sy in y0..y1 { for sx in x0..x1 { alpha+=ATLAS[(glyph*64*64+sy*64+sx)*4+3] as u32; } }
            self.blend_color_unchecked(x,y,color.0,color.1,color.2,(alpha/((x1-x0)*(y1-y0)) as u32) as u8);
        } }
        self.mark_dirty_rect(region.left,region.top,region.right-region.left,region.bottom-region.top);
    }
}
