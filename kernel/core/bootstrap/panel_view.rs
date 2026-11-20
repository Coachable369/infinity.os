//! OS-wide composition of the approved Eclipse Precision image kit.
use super::DisplayDevice;
use crate::ui::{geometry::Rect,panel_recipe::{self,Surface,Material,SurfaceStyle}};

impl DisplayDevice {
    // ------------------------=
    // FUNC: precision_toggle
    // DESC: Composes the Precision track and circular knob at opposite ends for the two boolean states.
    // ------------------=
    pub(super) fn precision_toggle(&mut self, rect: Rect, checked: bool, enabled: bool, mut material: Material) {
        if !enabled {
            material.opacity /= 2;
            material.edge_opacity /= 2;
            material.focus = 0;
        }
        let diameter=rect.height.min(rect.width);
        if diameter < 4 { return; }
        let edge=(54,87,116);
        let track=Material{fill:if checked {(36,90,212)}else{(23,43,72)},edge,..material};
        let style=SurfaceStyle{material:0,corners:2,radius:(diameter/(2*material.scale.max(1))as u32).min(40) as u8,fill:None,border:None};
        self.precision_surface_styled(rect,if checked{Surface::Primary}else{Surface::Secondary},track,style);
        let inset=(2*material.scale.max(1)) as u32;
        let size=diameter.saturating_sub(2*inset);
        let thumb=Rect{x:if checked {rect.right()-(size+inset)as i32}else{rect.x+inset as i32},
            y:rect.y+(rect.height-size)as i32/2,width:size,height:size};
        self.precision_surface(thumb,Surface::Current,Material{fill:if checked{(235,245,255)}else{(186,207,231)},edge,..material});

    }

    // ------------------------=
    // FUNC: precision_slider
    // DESC: Shares image-backed rail, filled progress and optical thumb artwork across native slider consumers.
    // ------------------=
    pub(super) fn precision_slider(&mut self, track:Rect, thumb:Rect, enabled:bool, mut material:Material) {
        if track.width==0 || track.height==0 {return;}
        if !enabled {material.opacity/=2;material.edge_opacity/=2;material.focus=0;}
        let style=SurfaceStyle{material:0,corners:2,radius:(track.height/(2*material.scale.max(1))as u32).min(40) as u8,fill:None,border:None};
        self.precision_surface_styled(track,Surface::Secondary,Material{fill:(35,61,92),edge:(54,87,116),..material},style);
        let width=(thumb.x+thumb.width as i32/2-track.x).clamp(0,track.width as i32) as u32;
        if width>0 {self.precision_surface_styled(Rect{width,..track},Surface::Primary,
            Material{fill:(61,120,234),edge:(54,87,116),..material},style);}
        self.precision_surface(thumb,Surface::Current,Material{fill:(235,245,255),edge:(54,87,116),..material});
    }

    // ------------------------=
    // FUNC: precision_radio
    // DESC: Uses the approved current/inactive circular artwork for a single selectable radio option, sharing focus and disabled opacity.
    // ------------------=
    pub(super) fn precision_radio(&mut self, rect: Rect, selected: bool, enabled: bool, mut material: Material) {
        if !enabled { material.opacity /= 2; material.edge_opacity /= 2; material.focus = 0; }
        self.precision_surface(rect, if selected {Surface::Current}else{Surface::Inactive}, material);
    }

    // ------------------------=
    // FUNC: precision_surface
    // DESC: Composes the authoritative nine-slice surface with signed clipping, independent background opacity and one damage submission.
    // ------------------=
    pub(super) fn precision_surface(&mut self,rect:Rect,kind:Surface,material:Material) {
        self.precision_surface_styled(rect,kind,material,panel_recipe::surface_style(kind));
    }

    // ------------------------=
    // FUNC: precision_surface_styled
    // DESC: Applies the same native surface sampler to global and per-element recipes with identical clipping and damage.
    // ------------------=
    pub(super) fn precision_surface_styled(&mut self,rect:Rect,kind:Surface,material:Material,style:SurfaceStyle) {
        if material.opacity == 0 && material.edge_opacity == 0 { return; }
        let left=rect.x.max(0) as usize; let top=rect.y.max(0) as usize;
        let right=rect.right().max(0).min(self.width as i32) as usize;
        let bottom=rect.bottom().max(0).min(self.height as i32) as usize;
        if right<=left || bottom<=top {return;}
        let Some(clip)=self.clipped_render_region(left,top,right-left,bottom-top) else{return;};
        let width=rect.width as usize; let height=rect.height as usize;
        // Retain foreground samples only. Every frame still blends against the
        // current wallpaper/windows, so glass remains live and transparent.
        let full = rect.x >= 0 && rect.y >= 0 && clip.left == left && clip.top == top
            && clip.right - left == width && clip.bottom - top == height;
        let key = crate::ui::surface_cache::Key { width, height, surface: kind, material, style };
        if kind != Surface::Frame && material.opacity > 0
            && crate::ui::surface_cache::with_surface(key, full, |pixels| {
                for y in clip.top..clip.bottom {
                    let row = (y as i32 - rect.y) as usize * width;
                    for x in clip.left..clip.right {
                        let (r,g,b,a) = pixels[row + (x as i32 - rect.x) as usize];
                        if a != 0 { self.blend_color_unchecked(x,y,r,g,b,a); }
                    }
                }
            }) {
            self.mark_dirty_rect(clip.left,clip.top,clip.right-clip.left,clip.bottom-clip.top);
            return;
        }
        let rim=if kind==Surface::Frame {panel_recipe::rim_width(kind,material.scale)}
            else if style.corners!=0 {width.max(height)}
            else if material.opacity==0 && matches!(kind,Surface::Panel|Surface::Field|Surface::Primary|Surface::Secondary) {panel_recipe::rim_width(kind,material.scale)}
            else {height.max(width)};
        for y in clip.top..clip.bottom {
            let row=(y as i32-rect.y) as usize;
            let inset=if row>=rim && row<height.saturating_sub(rim) {rim}else{width};
            let edge_end=(rect.x as i64+inset as i64).max(0) as usize;
            let edge_start=(rect.x as i64+width.saturating_sub(inset) as i64).max(0) as usize;
            let spans=if inset>=width || edge_end>=edge_start {[(clip.left,clip.right),(0,0)]}
                else {[(clip.left,clip.right.min(edge_end)),(clip.left.max(edge_start),clip.right)]};
            for (start,end) in spans {for x in start..end {
                    let col=(x as i32-rect.x) as usize;
                    let (r,g,b,a)=panel_recipe::pixel_styled(kind,width,height,col,row,material,style);
                    if a!=0 {self.blend_color_unchecked(x,y,r,g,b,a);}
            }}
        }
        self.mark_dirty_rect(clip.left,clip.top,clip.right-clip.left,clip.bottom-clip.top);
    }

    // ------------------------=
    // FUNC: precision_window_frame
    // DESC: Enforces the same rectangular image-backed corners on every OS-owned application window after client composition.
    // ------------------=
    pub(super) fn precision_window_frame(&mut self,rect:Rect,active:bool,scale:usize) {
        self.precision_window_frame_opacity(rect,active,scale,255);
    }

    // ------------------------=
    // FUNC: precision_window_frame_opacity
    // DESC: Composites the same frame during transient desktop fades without changing its shape or app-independent ownership.
    // ------------------=
    pub(super) fn precision_window_frame_opacity(&mut self,rect:Rect,active:bool,scale:usize,opacity:u8) {
        use crate::ui::skin::AccentSurface;
        let fill=self.active_accent_surface(AccentSurface::Widget);
        let edge=self.active_accent_surface(AccentSurface::WindowOutline);
        self.precision_surface(rect,Surface::Frame,Material{fill,edge,opacity:0,
            edge_opacity:opacity,focus:if active {255}else{0},pressed:false,scale});
    }
}
