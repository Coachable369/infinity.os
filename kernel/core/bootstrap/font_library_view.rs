use super::DisplayDevice;
use super::primitives::FontSize;
use crate::ui::{geometry::Rect,font_library::{self,Layout,ROWS}};
impl DisplayDevice {
    // ------------------------=
    // FUNC: font_library_label
    // DESC: Clips authentic font previews and kit labels to their own columns and restores the caller's clip.
    // ------------------=
    fn font_library_label(&mut self,r:Rect,text:&[u8],face:u8,bright:bool,scale:usize) {
        if r.y<0 || r.x<0 {return;}
        let old=self.render_clip;
        self.intersect_render_clip(r.x as usize,r.y as usize,r.width as usize,r.height as usize);
        let color=if bright {(225,240,250)}else{(145,178,201)};
        self.ui_text_raster_face(r.x as usize,r.y as usize,text,color.0,color.1,color.2,FontSize::new(24,16*scale),false,None,face);
        self.render_clip=old;
    }
    // ------------------------=
    // FUNC: font_library_button
    // DESC: Reuses rounded navy controls with cyan active state and visibly muted unavailable actions.
    // ------------------=
    fn font_library_button(&mut self,r:Rect,text:&[u8],enabled:bool,selected:bool,s:usize) {
        if r.y<0 || r.x<0 {return;}
        let (x,y,w,h)=(r.x as usize,r.y as usize,r.width as usize,r.height as usize);
        self.fill_rounded_rect_alpha(x,y,w,h,8*s,15,31,48,244);
        let color=if selected {(34,211,238)}else{(45,72,96)};
        self.outline_rounded_rect(x,y,w,h,8*s,color.0,color.1,color.2);
        self.font_library_label(Rect{x:r.x+8*s as i32,y:r.y+6*s as i32,width:r.width.saturating_sub(16*s as u32),height:r.height.saturating_sub(8*s as u32)},text,crate::runtime::font_faces::selected(),enabled,s);
    }
    // ------------------------=
    // FUNC: settings_font_library
    // DESC: Presents the actual bundled family catalog with native previews and persisted availability controls.
    // ------------------=
    pub(super) fn settings_font_library(&mut self,detail:Rect,s:usize) {
        let layout=Layout::new(detail,s);
        let state=crate::runtime::font_preferences::current();
        let catalog=crate::runtime::font::FontCatalog::new();
        let face=crate::runtime::font_faces::selected();
        for i in 0..ROWS {
            let Some(font)=catalog.nth(font_library::page()*ROWS+i) else {break;};
            let id=font.id as u8;let r=layout.rows[i];
            if r.y<0 {continue;}
            self.fill_rounded_rect_alpha(r.x.max(0) as usize,r.y as usize,r.width as usize,r.height as usize,8*s,11,25,39,230);
            if i==font_library::focus() {self.outline_rounded_rect(r.x.max(0) as usize,r.y as usize,r.width as usize,r.height as usize,8*s,34,211,238);}
            let width=(layout.select[i].x-r.x-24*s as i32).max(0) as u32;
            self.font_library_label(Rect{x:r.x+8*s as i32,y:r.y+5*s as i32,width,height:22*s as u32},font.family,face,true,s);
            self.font_library_label(Rect{x:r.x+8*s as i32,y:r.y+29*s as i32,width,height:22*s as u32},b"Aa Bb Cc 012345",id,state.contains(id),s);
            self.font_library_button(layout.select[i],if state.selected==id {b"Active"}else{b"Use"},state.contains(id),state.selected==id,s);
            self.font_library_button(layout.toggle[i],if id==2 {b"Required"}else if state.contains(id) {b"Remove"}else{b"Install"},id!=2,false,s);
        }
        self.font_library_button(layout.previous,b"Previous",font_library::page()>0,false,s);
        self.font_library_button(layout.next,b"Next",font_library::page()<7,false,s);
        let r=layout.footer;
        let label=match font_library::status() {1=>b"Saved" as &[u8],2=>b"Save failed; unchanged",_=>b"Open Font License"};
        self.font_library_label(Rect{x:r.x+184*s as i32,width:r.width.saturating_sub(184*s as u32),..r},label,face,font_library::status()!=2,s);
        self.font_library_label(Rect{x:r.x,y:r.y+36*s as i32,width:r.width,height:20*s as u32},b"Arrows: browse   Enter: use   Delete: install/remove",face,false,s);
    }
}
