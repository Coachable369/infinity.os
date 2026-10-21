//! Reusable Eclipse Access controls; no authentication state or service calls in paint.
use super::{access_style as palette, DisplayDevice};
use crate::ui::panel_recipe::Surface;
#[path = "../../ui/dock_material.rs"]
mod drawer_material;
static mut ACCESS_BACKDROP: drawer_material::Backdrop = drawer_material::Backdrop::new();

impl DisplayDevice {
    // ------------------------=
    // FUNC: access_frost_panel
    // DESC: Paints an outer access panel over its restored wallpaper with the same bounded blur, navy tint and sheen as the desktop drawer.
    // ------------------=
    pub(super) fn access_frost_panel(&mut self, left:usize, top:usize, width:usize,
        height:usize, fill:palette::Rgb, border:palette::Rgb, opacity:u8, edge_opacity:u8) {
        let Some(region)=self.clipped_render_region(left,top,width,height) else{return;};
        let pack=|c:palette::Rgb| (u32::from(c.0)<<16)|(u32::from(c.1)<<8)|u32::from(c.2);
        let material=drawer_material::Material{primary:pack(fill),accent:pack(border),
            opacity:((u16::from(opacity)*100+127)/255) as u8,blur:12,
            scale:self.ui_scale(),bgr:self.format!=0};
        // Full scenes seed clean parent samples after restoring the wallpaper.
        // Field/caret/progress damage reuses that grid and never samples controls
        // or changes the blur kernel just because the output clip is smaller.
        unsafe {(&mut *(&raw mut ACCESS_BACKDROP)).paint_retained_with_edge_opacity(
            core::slice::from_raw_parts_mut(self.buffer,self.stride*self.height),self.stride,
            (left,top,width,height),(region.left,region.top,region.right,region.bottom),material,edge_opacity,
            (left,top,left.saturating_add(width).min(self.width),top.saturating_add(height).min(self.height)));}
        self.mark_dirty_rect(region.left,region.top,region.right-region.left,region.bottom-region.top);
    }
    // ------------------------=
    // FUNC: access_panel_glow
    // DESC: Adds a clipped inward edge light without affecting panel content or surrounding wallpaper.
    // ------------------=
    pub(super) fn access_panel_glow(&mut self, left: usize, top: usize, width: usize, height: usize) {
        self.precision_surface(crate::ui::geometry::Rect{x:left as i32,y:top as i32,
            width:width as u32,height:height as u32},crate::ui::panel_recipe::Surface::Panel,
            crate::ui::panel_recipe::Material{fill:palette::SURFACE,edge:palette::BORDER,
                opacity:0,edge_opacity:palette::panel_motion().glow,
                focus:255,pressed:false,scale:self.ui_scale()});
    }
    // ------------------------=
    // FUNC: access_surface
    // DESC: Paints a palette-controlled glass surface with antialiased edges and a stable focus rim.
    // ------------------=
    pub(super) fn access_surface(&mut self, left: usize, top: usize, width: usize,
        height: usize, kind: Surface, fill: palette::Rgb, border: palette::Rgb,
        opacity: u8, focused: bool) {
        self.access_surface_alpha(left, top, width, height, kind, fill, border,
            opacity, 255, focused);
    }

    // ------------------------=
    // FUNC: access_surface_alpha
    // DESC: Preserves independent authored fill and border opacity while retaining the shared antialiased access recipe.
    // ------------------=
    pub(super) fn access_surface_alpha(&mut self, left: usize, top: usize, width: usize,
        height: usize, kind: Surface, fill: palette::Rgb, border: palette::Rgb,
        opacity: u8, border_opacity: u8, focused: bool) {
        use crate::ui::panel_recipe::Material;
        self.precision_surface(crate::ui::geometry::Rect{x:left as i32,y:top as i32,
            width:width as u32,height:height as u32},kind,Material{fill,edge:border,opacity,
                edge_opacity:border_opacity,focus:if focused {palette::focus_amount()}else{0},
                pressed:focused && palette::pressed(),scale:self.ui_scale()});
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
        self.precision_surface(crate::ui::geometry::Rect{x:left as i32,y:top as i32,
            width:width as u32,height:height as u32},crate::ui::panel_recipe::Surface::Panel,
            crate::ui::panel_recipe::Material{fill:palette::SURFACE,edge:palette::BORDER,
                opacity:240,edge_opacity:255,focus:0,pressed:false,scale:self.ui_scale()});
    }

    // ------------------------=
    // FUNC: access_button
    // DESC: Applies the same recipe to fallback setup actions without changing their native hit rectangles.
    // ------------------=
    pub(super) fn access_button(&mut self,left:usize,top:usize,width:usize,height:usize,label:&[u8],primary:bool,focused:bool) {
        self.access_surface(left,top,width,height,if primary {Surface::Primary}else{Surface::Secondary},
            if primary { palette::PRIMARY } else { palette::BACKGROUND },
            palette::BORDER,255,focused);
        let padding = (height / 3).min(width / 4);
        self.access_label((left + padding, top, width.saturating_sub(padding * 2), height),
            label, (height * 5 / 12).max(8), palette::TEXT, true, true);
    }

    // ------------------------=
    // FUNC: access_icon
    // DESC: Samples the generated Eclipse glyph mask with area-filtered alpha and the shared silver/violet tokens.
    // ------------------=
    pub(super) fn access_icon(&mut self, cx: usize, cy: usize, kind: usize, size: usize, active: bool) {
        let Some(&glyph) = [0usize,4,2,3,14,4,14,7,9,9,0,11,13,14,5,8].get(kind.wrapping_sub(1)) else { return; };
        self.access_glyph(cx,cy,glyph,size,active);
    }

    // ------------------------=
    // FUNC: access_glyph
    // DESC: Reuses the authored monochrome family for semantic controls and completed progress markers.
    // ------------------=
    pub(super) fn access_glyph(&mut self,cx:usize,cy:usize,glyph:usize,size:usize,active:bool) {
        const ATLAS: &[u8; 16*64*64*4] = include_bytes!("../../../assets/ui-design-kit/default/eclipse-access-glyphs-v1.rgba");
        if glyph>=16 {return;}
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
