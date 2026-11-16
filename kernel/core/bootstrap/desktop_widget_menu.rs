//! Shared native glass chooser, also exercised by the host pixel harness.
use super::*;
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
impl DisplayDevice {
    // ------------------------=
    // FUNC: desktop_widget_menu
    // DESC: Draws the glass desktop chooser with checked visibility rows above application layers.
    // ------------------=
    pub(crate) fn desktop_widget_menu(&mut self, scale: usize) {
        let state = crate::ui::desktop_widgets::current();
        let Some(rect) = state.menu_rect(self.width, self.height, scale) else {
            return;
        };
        let (x, y) = (rect.x.max(0) as usize, rect.y.max(0) as usize);
        use crate::ui::{geometry::Rect,panel_recipe::{Material,Surface}};
        let material=Material{fill:(16,27,58),edge:(117,165,206),opacity:245,edge_opacity:170,focus:0,pressed:false,scale};
        self.precision_surface(rect,Surface::Panel,material);
        let w=rect.width as usize;
        let heading=FontSize::new(UI_FONT_CELL_HEIGHT,18*scale);
        let body=FontSize::new(UI_FONT_CELL_HEIGHT,20*scale);
        let title:&[u8]=match(state.menu_widget,state.transparency_menu,state.visibility_menu){
            (_,true,_)=>b"Transparency",(Some(0),_,_)=>b"System Overview",(Some(1),_,_)=>b"Infinity AI",(Some(2),_,_)=>b"Weather",(_,_,true)=>b"Widgets",_=>b"Desktop"};
        self.ui_text_raster(x+16*scale,y+9*scale,title,157,197,230,heading,true,None);
        self.fill_rounded_rect_alpha(x+12*scale,y+35*scale,w-24*scale,scale,0,126,164,206,80);
        if let Some(id)=state.menu_widget {
            if state.transparency_menu{
                let percent=state.percent(id) as usize;let mut digits=[0;20];
                let n=super::editor_view::decimal(percent,&mut digits);
                let mut text=[0;4];text[..n.len()].copy_from_slice(n);text[n.len()]=b'%';
                self.ui_text_raster(x+w-60*scale,y+9*scale,&text[..n.len()+1],220,237,251,heading,true,None);
                let start=x+20*scale;let track=w-40*scale;let thumb=start+track*percent/100;
                self.fill_rounded_rect_alpha(start,y+67*scale,track,5*scale,2*scale,96,117,152,190);
                self.fill_rounded_rect_alpha(start,y+67*scale,thumb-start,5*scale,2*scale,102,192,249,230);
                self.precision_surface(Rect{x:(thumb-8*scale)as i32,y:(y+61*scale)as i32,width:(16*scale)as u32,height:(16*scale)as u32},Surface::Current,material);
                return;
            }
        }
        let labels:&[&[u8]]=match state.menu_widget{
            Some(0)=>&[b"System settings",b"Transparency",b"Hide widget"],
            Some(1)=>&[b"AI & Voice settings",b"Transparency",b"Hide widget"],
            Some(_)=>&[b"Set location",b"Transparency",b"Hide widget"],
            None if state.visibility_menu=>&[b"System Overview",b"Infinity AI",b"Weather",b"Reset positions"],
            None=>&[b"Widgets"],
        };
        for (i,label) in labels.iter().enumerate(){
            let top=y+(36+i*36)*scale;
            if state.menu_hover==Some(i+1){
                self.fill_rounded_rect_alpha(x+6*scale,top+2*scale,w-12*scale,32*scale,6*scale,74,133,205,85);
            }
            if state.visibility_menu && i==3{self.fill_rounded_rect_alpha(x+12*scale,top,w-24*scale,scale,0,126,164,206,70);}
            self.ui_text_raster(x+16*scale,top+7*scale,label,218,232,246,body,false,None);
            if state.visibility_menu && i<3 {
                self.precision_toggle(Rect{x:(x+w-51*scale)as i32,y:(top+8*scale)as i32,width:(34*scale)as u32,height:(19*scale)as u32},state.visible&(1<<i)!=0,true,material);
            }else if (!state.visibility_menu && state.menu_widget.is_none()) || (state.menu_widget.is_some()&&i==1){
                self.ui_text_raster(x+w-26*scale,top+6*scale,b">",130,183,228,body,false,None);
            }
        }
    }
}
