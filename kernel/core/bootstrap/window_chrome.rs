//! Shared native glass window controls; geometry remains owned by SystemLayout.
impl super::DisplayDevice {
    // ------------------------=
    // FUNC: kit_close_control
    // DESC: Centers the shared circular Close artwork inside the control's existing pointer target.
    // ------------------=
    pub(super) fn kit_close_control(&mut self, rect:crate::ui::geometry::Rect) {
        let size=rect.width.min(rect.height).min((26*self.ui_scale().max(1)) as u32);
        if size==0 {return;}
        self.window_control_state((rect.x+(rect.width-size) as i32/2).max(0) as usize,
            (rect.y+(rect.height-size) as i32/2).max(0) as usize,size as usize,2,false,
            crate::ui::app_launcher::window_controls::state(rect));
    }
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
        use crate::ui::app_launcher::window_controls as controls;
        let rect=crate::ui::geometry::Rect{x:x as i32,y:y as i32,width:size as u32,height:size as u32};
        let state=controls::state(rect);
        self.window_control_state(x,y,size,index,maximized,state);
    }

    // ------------------------=
    // FUNC: window_control_state
    // DESC: Shares Studio's authored control pixels when an application retains a larger accessible hit rectangle.
    // ------------------=
    pub(super) fn window_control_state(&mut self,x:usize,y:usize,size:usize,index:usize,maximized:bool,state:usize) {
        use crate::ui::app_launcher::window_controls as controls;
        let primary=self.active_primary_rgb();
        let accent=self.active_accent_rgb();
        let unit = (size / 20).max(1);
        let Some(clip)=self.clipped_render_region(x,y,size,size) else {return;};
        for py in clip.top-y..clip.bottom-y {for px in clip.left-x..clip.right-x {
            let (r,g,b,a)=controls::pixel(size,px,py,state,primary,accent,index==2);
            if a!=0 {self.blend_color_unchecked(x+px,y+py,r,g,b,a);}
        }}
        self.mark_dirty_rect(clip.left,clip.top,clip.right-clip.left,clip.bottom-clip.top);
        let cx = (x + size / 2) as i32;
        let cy = (y + size / 2) as i32;
        let r = (size / 5).max(3) as i32;
        let center=controls::pixel(size,size/2,size/2,state,primary,accent,index==2);
        let luminance=(u32::from(center.0)*54+u32::from(center.1)*183+u32::from(center.2)*19)/256;
        let color = if luminance>150 {(15,25,36)}else{(226,242,252)};
        match index {
            0 => {
                for offset in 0..unit as i32 {
                    self.line(cx-r,cy+offset,cx+r,cy+offset,color.0,color.1,color.2);
                }
            }
            1 => {
                let shift = if maximized { 2 * unit } else { 0 };
                if maximized {
                    self.outline_rect(
                        (cx - r) as usize + shift,
                        (cy - r) as usize - shift,
                        (2 * r) as usize,
                        (2 * r) as usize,
                        color.0,
                        color.1,
                        color.2,
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
                for offset in 0..unit as i32 {
                    self.line(cx-r+offset,cy-r,cx+r+offset,cy+r,color.0,color.1,color.2);
                    self.line(cx+r+offset,cy-r,cx-r+offset,cy+r,color.0,color.1,color.2);
                }
            }
        }
    }
}
