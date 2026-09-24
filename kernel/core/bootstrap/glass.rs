//! Shared native glass recipe, used directly by the desktop and pixel regression harness.
use super::*;
impl DisplayDevice {
    // ------------------------=
    // FUNC: gravity_collection
    // DESC: Paints a circular kit collection with concentric glass rims, accent lighting and centered live labels.
    // ------------------=
    pub(in super::super) fn gravity_collection(
        &mut self, x: usize, y: usize, width: usize, height: usize,
        label: &[u8], count: usize, index: usize, selected: bool,
    ) {
        let size = width.min(height);
        let left = x + (width - size) / 2;
        let top = y + (height - size) / 2;
        let (r,g,b) = [(0,209,255),(167,139,250),(251,191,36),(163,215,246)][index.min(3)];
        self.fill_rounded_rect_alpha(left,top,size,size,size/2,r,g,b,if selected {70}else{28});
        self.fill_rounded_rect_alpha(left+3,top+3,size.saturating_sub(6),size.saturating_sub(6),size/2-3,5,17,32,235);
        self.outline_rounded_rect(left,top,size,size,size/2,r,g,b);
        if size > 32 {
            self.outline_rounded_rect(left+8,top+8,size-16,size-16,(size-16)/2,r/2,g/2,b/2);
            let radius = size as i32 / 2 - 10;
            for dy in -radius..radius {
                for dx in -radius..radius {
                    if dx*dx + dy*dy < radius*radius {
                        let alpha = ((radius-dy) * 22 / (radius*2)) as u8;
                        self.blend_color((left+size/2) as i32+dx,(top+size/2) as i32+dy,r,g,b,alpha);
                    }
                }
            }
        }
        let cy = top + size/3;
        let cx = left + size/2;
        let points: &[(i32,i32)] = match index {
            0 => &[(-10,-9),(-3,-9),(0,-5),(10,-5),(10,10),(-10,10),(-10,-9)],
            1 => &[(8,-10),(3,4),(-8,10),(-3,-4),(8,-10)],
            2 => &[(0,-12),(4,-4),(12,-3),(6,3),(8,12),(0,7),(-8,12),(-6,3),(-12,-3),(-4,-4),(0,-12)],
            _ => &[],
        };
        let scale = (size/100).max(1) as i32;
        for segment in points.windows(2) {
            self.icon_line(cx as i32+segment[0].0*scale,cy as i32+segment[0].1*scale,
                cx as i32+segment[1].0*scale,cy as i32+segment[1].1*scale,(r,g,b),18);
        }
        if index==1 { self.icon_circle(cx as i32,cy as i32,12*scale,(r,g,b),18); }
        if index==3 {
            for offset in [-8,0,8] {
                self.fill_rounded_rect_alpha((cx as i32+offset*scale-2) as usize,cy-2,5,5,2,227,238,247,255);
            }
        }
        self.ui_text_centered_strong(left,size,top+size/2,label,227,238,247,1);
        let count = count.min(99);
        let mut caption = *b"00 ideas";
        caption[0] = b'0' + (count/10) as u8;
        caption[1] = b'0' + (count%10) as u8;
        self.ui_text_centered(left,size,top+size/2+UI_FONT_CELL_HEIGHT+6,
            if count<10 {&caption[1..]} else {&caption},143,185,208,1);
    }

    // ------------------------=
    // FUNC: glass_label_pill
    // DESC: Paints a compact kit capsule with matched semibold metrics and stable selection geometry.
    // ------------------=
    pub(in super::super) fn glass_label_pill(
        &mut self,
        left: usize,
        top: usize,
        width: usize,
        height: usize,
        label: &[u8],
        selected: bool,
    ) {
        if width < 8 || height < 8 {
            return;
        }
        let radius = (height / 2).min(16);
        self.fill_rounded_rect_alpha(left, top, width, height, radius, 80, 132, 167, 125);
        self.fill_rounded_rect_alpha(
            left + 1,
            top + 1,
            width - 2,
            height - 2,
            radius - 1,
            10,
            29,
            47,
            245,
        );
        // Layer shallow, fading rounded reflections rather than a hard top cap.
        for inset in (2..=8).rev() {
            self.fill_rounded_rect_alpha(
                left + inset,
                top + 2,
                width.saturating_sub(inset * 2),
                height.saturating_sub(4) / 2,
                radius.saturating_sub(inset),
                130,
                190,
                229,
                if selected { 7 } else { 4 },
            );
        }
        if selected {
            self.outline_rounded_rect(left, top, width, height, radius, 100, 172, 219);
        }
        let mut shortened = [0u8; 96];
        let mut length = label.len().min(shortened.len() - 3);
        shortened[..length].copy_from_slice(&label[..length]);
        let available = width.saturating_sub(32);
        let mut shown = length;
        if self.ui_text_width_weighted(&shortened[..length], 1, true) > available {
            loop {
                shortened[length..length + 3].copy_from_slice(b"...");
                shown = length + 3;
                if self.ui_text_width_weighted(&shortened[..shown], 1, true) <= available {
                    break;
                }
                if length == 0 {
                    return;
                }
                length -= 1;
            }
        }
        self.ui_text_centered_strong(
            left,
            width,
            top + height.saturating_sub(UI_FONT_CELL_HEIGHT) / 2,
            &shortened[..shown],
            227,
            238,
            247,
            1,
        );
    }

    // ------------------------=
    // FUNC: glass_panel
    // DESC: Builds a layered translucent panel with restrained shadow, highlight, and cyan edge treatment.
    // ------------------=
    pub(in super::super) fn glass_panel(
        &mut self,
        left: usize,
        top: usize,
        width: usize,
        height: usize,
        strong: bool,
    ) {
        let (panel_r, panel_g, panel_b) =
            self.active_accent_surface(crate::ui::skin::AccentSurface::Widget);
        let (outline_r, outline_g, outline_b) =
            self.active_accent_surface(crate::ui::skin::AccentSurface::WindowOutline);
        self.glass_panel_with_palette(
            left,
            top,
            width,
            height,
            strong,
            (panel_r, panel_g, panel_b),
            (outline_r, outline_g, outline_b),
        );
    }

    // ------------------------=
    // FUNC: glass_panel_with_palette
    // DESC: Draws the shared layered glass recipe using one caller-selected surface and edge palette.
    // ------------------=
    pub(in super::super) fn glass_panel_with_palette(
        &mut self,
        left: usize,
        top: usize,
        width: usize,
        height: usize,
        strong: bool,
        panel: (u8, u8, u8),
        outline: (u8, u8, u8),
    ) {
        let radius = (width.min(height) / 12).clamp(8, 18);
        let (opacity, blur) = self.active_background_effects();
        if blur >= 2 && opacity < 100 && !self.fast_motion_frame {
            self.blur_framebuffer_region(left, top, width, height, blur as usize);
        }
        if self.skin_visual_mode() == 1 {
            self.fill_rounded_rect_alpha(
                left,
                top,
                width,
                height,
                radius,
                248,
                251,
                255,
                ((if strong { 244u16 } else { 226u16 }) * u16::from(opacity) / 100) as u8,
            );
            self.outline_rounded_rect(left, top, width, height, radius, 122, 145, 166);
            return;
        }
        if self.skin_visual_mode() == 2 {
            self.fill_rounded_rect_alpha(
                left,
                top,
                width,
                height,
                radius,
                8,
                8,
                8,
                (255u16 * u16::from(opacity) / 100) as u8,
            );
            self.outline_rounded_rect(left, top, width, height, radius, 255, 255, 255);
            return;
        }
        for inset in (1..=5usize).rev() {
            let (shadow_red, shadow_green, shadow_blue, shadow_alpha) = if strong {
                (5, 29, 42, 12)
            } else {
                (0, 4, 10, 18)
            };
            self.fill_rounded_rect_alpha(
                left.saturating_add(inset * 2),
                top.saturating_add(inset * 2),
                width,
                height,
                radius,
                shadow_red,
                shadow_green,
                shadow_blue,
                shadow_alpha,
            );
        }
        self.fill_rounded_rect_alpha(
            left,
            top,
            width,
            height,
            radius,
            panel.0,
            panel.1,
            panel.2,
            ((if strong { 232u16 } else { 204u16 }) * u16::from(opacity) / 100) as u8,
        );
        self.outline_rounded_rect(
            left, top, width, height, radius, outline.0, outline.1, outline.2,
        );
        if width > 4 && height > 4 {
            let divisor = if strong { 3 } else { 5 };
            let inner_edge = (
                outline.0 / divisor,
                outline.1 / divisor,
                outline.2 / divisor,
            );
            self.outline_rounded_rect(
                left + 2,
                top + 2,
                width - 4,
                height - 4,
                radius.saturating_sub(2),
                inner_edge.0,
                inner_edge.1,
                inner_edge.2,
            );
        }
    }
}
