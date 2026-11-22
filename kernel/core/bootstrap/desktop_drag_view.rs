//! Shared drag overlay, rendered unchanged by the guest and pixel-proof harness.
use super::*;
impl DisplayDevice {
    // ------------------------=
    // FUNC: drop_compass_overlay
    // DESC: Composites the reusable transparent glass ring and native destination cards.
    // ------------------=
    pub(super) fn drop_compass_overlay(&mut self) {
        let state=crate::ui::drop_compass::current();
        let Some(c)=state.center else{return;};
        let diameter=440.min(self.height.saturating_sub(140)).min(self.width.saturating_sub(160));
        self.paint_bitmap_alpha_fit_rect_opacity(include_bytes!("../../../assets/ui-design-kit/default/drop-compass-ring.bmp"),
            (c.x-diameter as i32/2).max(0)as usize,(c.y-diameter as i32/2).max(0)as usize,diameter,diameter,64);
        let radius=diameter as i32*46/100;
        for dy in -radius-2..=radius+2 {for dx in -radius-2..=radius+2 {
            let distance=(dx*dx+dy*dy-radius*radius).abs();
            let band=radius*3;
            if distance<band && c.x+dx>=0 && c.y+dy>=0 {
                self.blend_color(c.x+dx,c.y+dy,102,190,231,((band-distance)*155/band)as u8);
            }
        }}
        let cards=state.cards(self.width,self.height);
        let labels:[&[u8];4]=[b"Projects",b"Documents",b"Downloads",b"Trash"];
        let subtitles:[&[u8];4]=if state.file{[b"Move to Projects",b"Move to Documents",b"Move to Downloads",b"Move to Trash"]}else{[b"Open Projects",b"Open Documents",b"Open Downloads",b"Explore contents"]};
        for (i,r) in cards.into_iter().enumerate(){
            self.precision_surface(r,crate::ui::panel_recipe::Surface::Panel,crate::ui::panel_recipe::Material{
                fill:(7,29,48),edge:(100,181,222),opacity:237,edge_opacity:190,focus:if state.selected==Some(i){255}else{0},pressed:false,scale:1});
            self.themed_icon(r.x.max(0)as usize+36,r.y.max(0)as usize+r.height as usize/2,[9,4,5,10][i],40);
            self.ui_text_raster((r.x+65).max(0)as usize,(r.y+15).max(0)as usize,labels[i],232,244,255,FontSize::new(UI_FONT_CELL_HEIGHT,20),true,None);
            self.ui_text_raster((r.x+65).max(0)as usize,(r.y+43).max(0)as usize,subtitles[i],158,193,215,FontSize::new(UI_FONT_CELL_HEIGHT,14),false,None);
        }
        self.themed_icon(c.x as usize,c.y as usize-18,if state.file{49}else{4},92);
        let (name,length)=crate::ui::desktop_drag::name();
        let label=if state.file && length>0{&name[..length]}else{b"Desktop".as_slice()};
        let mut clipped=[0u8;256];
        let mut length=label.len().min(252);
        while length>0 && self.ui_text_width(&label[..length],1)>210 {length-=1;}
        clipped[..length].copy_from_slice(&label[..length]);
        if length<label.len(){clipped[length..length+3].copy_from_slice(b"...");length+=3;}
        let label=&clipped[..length];
        let width=self.ui_text_width(label,1)+28;
        self.fill_rounded_rect_alpha((c.x-width as i32/2).max(0)as usize,(c.y+42)as usize,width,34,12,47,154,225,235);
        self.ui_text_centered((c.x-width as i32/2).max(0)as usize,width,(c.y+50)as usize,label,230,245,255,1);
    }
    // ------------------------=
    // FUNC: desktop_trash_tile
    // DESC: Centers the themed wastebasket and caption in the drop target used by input.
    // ------------------=
    pub(super) fn desktop_trash_tile(&mut self, scale: usize) {
        let tile = crate::ui::desktop_widgets::current().position_trash(crate::ui::desktop_polish::trash_rect(self.width,self.height,scale),self.width,self.height);
        let size = (self.height / 21).max(44);
        self.themed_icon(tile.x as usize + tile.width as usize/2,tile.y as usize+size/2,10,size);
        self.ui_text_centered(tile.x as usize,tile.width as usize,tile.y as usize+size+8,
            b"Trash",221,235,243,1);
    }

    // ------------------------=
    // FUNC: desktop_drag_overlay
    // DESC: Reuses the themed icon with a finite cyan lift/drop halo above retained windows.
    // ------------------=
    pub(super) fn desktop_drag_overlay(&mut self) {
        let Some(view) = crate::ui::desktop_drag::current() else { return; };
        let x = self.width * view.position[0] as usize / 1000;
        let y = self.height * view.position[1] as usize / 1000;
        let size = (self.height / 21).max(44);
        for pad in (3..=9).rev().step_by(3) {
            self.fill_rounded_rect_alpha(x.saturating_sub(pad), y.saturating_sub(pad),
                size + 2*pad, size + 2*pad, 12, 24, 158, 215,
                (u16::from(view.emphasis) * 18 / 255) as u8);
        }
        // Redraw the foreground icon over the highlight; artwork never scales
        // or lags behind the pointer. The persistent desktop owns the drop.
        let (label,kind): (&[u8],usize) = match view.item {
            0=>(b"Documents",0),1=>(b"Downloads",1),2=>(b"Pictures",0),
            3=>(b"Music",0),4=>(b"Videos",0),5=>(b"Projects",2),_ =>(b"notes.txt",3),
        };
        let (captured,length) = crate::ui::desktop_drag::name();
        let label = if length > 0 { &captured[..length] } else { label };
        let role = if view.source.is_some() { [4,5,6,7,8,9,49][view.item.min(6)] } else {49};
        let _=kind;
        self.themed_icon(x+size/2,y+size/2,role,size);
        let label_width=self.ui_text_width(label,1);
        self.ui_text(x+size.saturating_sub(label_width)/2,y+size+8,label,221,235,243,1);
    }
}
