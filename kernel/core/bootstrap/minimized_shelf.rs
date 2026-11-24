//! Native theme-aware glass shelf; all controls and glyphs remain live framebuffer UI.
use super::*;
use crate::ui::app_launcher::minimized_shelf::{self as shelf, Geometry};

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
impl DisplayDevice {
    // ------------------------=
    // FUNC: minimized_app_shelf
    // DESC: Composes minimized windows above app surfaces, using selected-theme icons and a left-opening menu.
    // ------------------=
    pub(super) fn minimized_app_shelf(&mut self) {
        let state = shelf::current();
        if !state.visible() { return; }
        let g = Geometry::new(self.width, self.height, state);
        let r = g.rail;
        let x = r.x.max(0) as usize;
        let y = r.y.max(0) as usize;
        let w = r.width as usize;
        let h = r.height as usize;
        // Reuse the kit's frosted surface recipe with the Eclipse blue/silver
        // tokens; do not introduce an independently rasterized glass layer.
        let edge = self.active_accent_surface(crate::ui::skin::AccentSurface::WindowOutline);
        self.precision_surface(r, crate::ui::panel_recipe::Surface::Panel,
            crate::ui::panel_recipe::Material {
                fill: self.active_accent_surface(crate::ui::skin::AccentSurface::Widget),
                edge, opacity: 208, edge_opacity: 175, focus: 0,
                pressed: false, scale: self.ui_scale(),
            });
        let snap = state.drag.and_then(|(px, _, _, _)| {
            if px <= 35 {
                Some(true)
            } else if px + (w * 1000 / self.width) as i32 >= 965 {
                Some(false)
            } else {
                None
            }
        });
        if let Some(left) = snap {
            let edge = if left { x + 3 } else { x + w - 6 };
            self.fill_rounded_rect_alpha(
                edge,
                y + 12,
                3,
                h.saturating_sub(24),
                1,
                117,
                223,
                255,
                220,
            );
        }
        self.fill_rounded_rect_alpha(
            x + 2,
            y + 2,
            w.saturating_sub(4),
            (g.base as usize / 3).max(8),
            8,
            135,
            212,
            255,
            14,
        );
        if snap.is_some() || state.drag.is_some() {
            self.ui_text_centered(
                x,
                w,
                y + w / 10,
                if snap.is_some() { b"SNAP" } else { b"MOVE" },
                137,
                199,
                228,
                1,
            );
        }
        // Grip, window count and scroll position remain visible independently of selection.
        let grip_width=(w/4).max(1);
        self.fill_rounded_rect_alpha(x+(w-grip_width)/2,y+6,grip_width,3,1,151,192,220,160);
        let count=state.count();
        if count>0 && state.drag.is_none() && snap.is_none() {
            let digits=[b'0'+(count/10) as u8,b'0'+(count%10) as u8];
            let label=if shelf::presentation().0>180 {state.hover.map(shelf::label).unwrap_or(b"Tasks")}else if count<10 {&digits[1..]}else{&digits};
            self.ui_text_centered(x,w,y+12,label,154,211,238,1);
        }
        // Reuse the selected application's UIKit icon, centered under its name.
        if shelf::presentation().0 > 180 && state.menu.is_none() {
            if let Some(id) = state.hover {
                let size = (g.base as usize * 2 / 5).max(24);
                self.launcher_icon(x + w / 2, y + g.base as usize * 2 / 3,
                    shelf::icon(id), size);
            }
        }
        if count>g.capacity {
            let track=h.saturating_sub(56);
            let thumb=(track*g.capacity/count).max(4).min(track);
            let offset=(track-thumb)*state.offset.min(count-g.capacity)/(count-g.capacity);
            self.fill_rounded_rect_alpha(x+w-5,y+32+offset,3,thumb,1,106,197,235,200);
        }
        let expansion = shelf::presentation().0 as usize;
        let base = g.base as usize;
        // The compact rail yields to the selected application action menu.
        for index in 0..if expansion<=40 {g.capacity}else{0} {
            let Some(id)=state.item(state.offset+index) else {break;};
            let tile=g.tile_rect(index);
            let tx=tile.x.max(0) as usize;
            let ty=tile.y.max(0) as usize;
            let selected=state.menu==Some(id)||state.hover==Some(id);
            self.fill_rounded_rect_alpha(tx,ty,tile.width as usize,base,base/2,174,204,238,if selected {68}else{24});
            self.launcher_icon(tx+tile.width as usize/2,ty+base/2,shelf::icon(id),base*3/5);
            self.fill_rounded_rect_alpha(tx+tile.width as usize-8,ty+base-10,5,5,2,63,221,174,255);
        }
        if expansion>40 && state.hover.is_some() && state.menu.is_none() {
            let previous_clip=self.render_clip;
            let mut clip=PresentRegion {left:x,top:y,right:(x+w).min(self.width),bottom:(y+h).min(self.height)};
            if let Some(old)=previous_clip {clip.left=clip.left.max(old.left);clip.top=clip.top.max(old.top);clip.right=clip.right.min(old.right);clip.bottom=clip.bottom.min(old.bottom);}
            self.render_clip=Some(clip);
            let opacity=((expansion-40)*255/215) as u8;
            let glass_opacity=(opacity as u16*190/255) as u8;
            let spine=(base*62/100).max(18);
            let spine_x=x+w-spine-5;
            self.fill_rounded_rect_alpha(spine_x,y+5,spine,h.saturating_sub(10),spine/2,248,250,255,glass_opacity);
            let (hover,tail)=shelf::bubble_state();
            let first=g.action_rect(0);
            let pitch=g.action_rect(1).y-first.y;
            let lead=shelf::presentation().1;
            let top=first.y+lead.min(tail)*pitch/256;
            let bottom=first.y+lead.max(tail)*pitch/256+first.height as i32;
            // Paint the glass lenses below the moving white selection surface.
            for index in 0..shelf::ACTION_LABELS.len() {
                let a=g.action_rect(index);
                let ax=a.x.max(0)as usize;
                let ay=a.y.max(0)as usize;
                let aw=a.width as usize;
                let ah=a.height as usize;
                // The clip has a raised, softly shaded glass row, not a flat pill.
                for line in 0..ah {
                    let radius=ah/2;
                    let dy=if line<radius {radius-line}else{line-(ah-radius)};
                    let inset=radius-shelf::circle_extent(radius as i32,dy as i32)as usize;
                    let alpha=(44usize.saturating_sub(line*24/ah))*opacity as usize/255;
                    self.fill_rect_alpha(ax+inset,ay+line,aw.saturating_sub(inset*2),1,174,204,238,alpha as u8);
                }
                let growth=hover[index].max(0)as usize;
                let lens=(ah+(aw.saturating_sub(ah))*growth/255).min(aw);
                self.fill_rounded_rect_alpha(ax,ay,lens,ah,ah/2,214,233,255,(growth*opacity as usize*64/(255*255))as u8);
            }
            if state.hover.is_some() {
                let shoulder=(base*18/100).max(6) as i32;
                for py in top-shoulder..bottom+shoulder {
                    if let Some((left,_))=shelf::bubble_span(py*4+2,top*4,bottom*4,first.x*4,spine_x as i32*4,shoulder*4) {
                        let start=(left+3)/4;
                        self.fill_rect_alpha(start.max(0)as usize,py.max(0)as usize,(spine_x as i32-start).max(0)as usize,1,248,250,255,glass_opacity);
                        let coverage=(start*4-left).clamp(0,4);
                        if coverage>0 {self.fill_rect_alpha((start-1).max(0)as usize,py.max(0)as usize,1,1,248,250,255,(glass_opacity as i32*coverage/4)as u8);}
                    }
                }
            }
            for (index,label) in shelf::ACTION_LABELS.iter().enumerate() {
                let a=g.action_rect(index);
                let ax=a.x.max(0)as usize;
                let ay=a.y.max(0)as usize;
                let ah=a.height as usize;
                let icon_size=(ah*3/5).min(32);
                let ix=ax+ah/2;
                let iy=ay+ah/2;
                if index<4 {self.window_control(ix-icon_size/2,iy-icon_size/2,icon_size,if index>=2 {2}else{1},index==0);}
                else {self.launcher_icon(ix,iy,19,icon_size);}
                let text=if index==4 {b"Task Browser" as &[u8]}else{label};
                let font=(base*18/100).max(14);
                let text_y=ay+ah.saturating_sub(font)/2;
                self.template_text(ax+ah,text_y,text,224,237,252,opacity,font,false);
                // Recolor only pixels covered by the moving selection, rather than
                // switching the whole label when its center crosses the bubble.
                if state.hover.is_some() {
                    let text_clip=PresentRegion {left:clip.left,top:clip.top.max(top.max(0)as usize),right:clip.right,bottom:clip.bottom.min(bottom.max(0)as usize)};
                    if text_clip.top<text_clip.bottom {
                        self.render_clip=Some(text_clip);
                        self.template_text(ax+ah,text_y,text,21,43,65,opacity,font,false);
                        self.render_clip=Some(clip);
                    }
                }
            }
            self.render_clip=previous_clip;
        }

        if state.count() > g.capacity {
            self.ui_text_centered(x, w, y + h - 25, b"v", 123, 209, 251, 1);
        }
        if let Some(id) = state.menu {
            let m = g.menu;
            let mx = m.x.max(0) as usize;
            let my = m.y.max(0) as usize;
            let mw = m.width as usize;
            let row = g.row_height as usize;
            self.glass_panel(
                mx,
                my,
                mw,
                if state.menu.is_some() {
                    m.height as usize
                } else {
                    row
                },
                true,
            );
            self.ui_text(
                mx + 14,
                my + (row.saturating_sub(20)) / 2,
                shelf::label(id),
                130,
                191,
                223,
                1,
            );
            if id < shelf::NAVIGATORS {
                self.ui_text(
                    mx + mw - 28,
                    my + row.saturating_sub(20) / 2,
                    &[b'1' + id as u8],
                    130,
                    191,
                    223,
                    1,
                );
            }
            if state.menu.is_none() {
                return;
            }
            self.fill_rect(mx + 12, my + row - 1, mw.saturating_sub(24), 1, 37, 82, 110);
            for (index, label) in shelf::ACTION_LABELS.iter().enumerate()
            {
                let ry = my + (index + 1) * row;
                if state.row == Some(index) {
                    self.fill_rounded_rect_alpha(
                        mx + 5,
                        ry + 2,
                        mw - 10,
                        row - 4,
                        6,
                        48,
                        134,
                        187,
                        145,
                    );
                }
                self.ui_text(
                    mx + 16,
                    ry + row.saturating_sub(20) / 2,
                    label,
                    if index == 2 { 255 } else { 223 },
                    if index == 2 { 158 } else { 236 },
                    if index == 2 { 157 } else { 245 },
                    1,
                );
            }
        }
    }
}
