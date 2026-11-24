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
        if count>g.capacity {
            let track=h.saturating_sub(56);
            let thumb=(track*g.capacity/count).max(4).min(track);
            let offset=(track-thumb)*state.offset.min(count-g.capacity)/(count-g.capacity);
            self.fill_rounded_rect_alpha(x+w-5,y+32+offset,3,thumb,1,106,197,235,200);
        }
        let expansion = shelf::presentation().0 as usize;
        let base = g.base as usize;
        // The compact rail yields to the selected application action menu.
        for index in 0..if expansion<=180 {g.capacity}else{0} {
            let Some(id)=state.item(state.offset+index) else {break;};
            let tile=g.tile_rect(index);
            let tx=tile.x.max(0) as usize;
            let ty=tile.y.max(0) as usize;
            let selected=state.menu==Some(id)||state.hover==Some(id);
            self.fill_rounded_rect_alpha(tx,ty,tile.width as usize,base,base/2,174,204,238,if selected {68}else{24});
            self.launcher_icon(tx+tile.width as usize/2,ty+base/2,shelf::icon(id),base*3/5);
            self.fill_rounded_rect_alpha(tx+tile.width as usize-8,ty+base-10,5,5,2,63,221,174,255);
        }
        if expansion>180 && state.hover.is_some() && state.menu.is_none() {
            let opacity=((expansion-180)*255/75) as u8;
            let spine=12;
            self.fill_rounded_rect_alpha(x+w-spine-5,y+12,spine,h.saturating_sub(24),spine/2,237,244,252,opacity);
            let first=g.action_rect(0);
            let pitch=g.action_rect(1).y-first.y;
            if state.row.is_some() {
                let sy=first.y+(shelf::presentation().1*pitch/256);
                self.fill_rounded_rect_alpha(first.x.max(0)as usize,sy.max(0)as usize,first.width as usize,first.height as usize,first.height as usize/2,237,244,252,opacity);
            }
            for (index,label) in shelf::ACTION_LABELS.iter().enumerate() {
                let a=g.action_rect(index);
                let selected=state.row==Some(index);
                if !selected {self.fill_rounded_rect_alpha(a.x.max(0)as usize,a.y.max(0)as usize,a.width as usize-12,a.height as usize,a.height as usize/2,174,204,238,24);}
                let icon_size=(a.height as usize*3/5).min(48);
                let ix=a.x.max(0)as usize+base/2;
                let iy=a.y.max(0)as usize+a.height as usize/2;
                if index<4 {self.window_control(ix-icon_size/2,iy-icon_size/2,icon_size,if index>=2 {2}else{1},index==0);}
                else {self.launcher_icon(ix,iy,19,icon_size);}
                self.ui_text(a.x.max(0)as usize+base,a.y.max(0)as usize+(a.height as usize).saturating_sub(if index==4 {40}else{20})/2,if index==4 {b"Show in"}else{label},
                    if selected {21}else{224},if selected {43}else{237},if selected {65}else{252},1);
                if index==4 {self.ui_text(a.x.max(0)as usize+base,a.y.max(0)as usize+(a.height as usize).saturating_sub(40)/2+20,b"Task Browser",if selected {21}else{224},if selected {43}else{237},if selected {65}else{252},1);}
            }
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
