//! Native browser chrome and retained viewport. Servo never owns desktop drawing.
use super::DisplayDevice;
use infinity_browser_core::{layout::Layout,skin,Viewport,frames::Frame};
use crate::ui::system_layout::DesktopAppWindowState;
const ICON:&[u8]=include_bytes!("../../../assets/apps/infinity-browser-icon-v1.bmp");
const NAV:&[u8]=include_bytes!("../../../assets/apps/infinity-browser-navigation-v1.bmp");
static mut REVISION:Option<u64>=None;
static mut PAGE_KEY:Option<infinity_browser_core::damage::PageKey>=None;
static mut FRAME:Option<Frame<'static,16384000>>=None;

// ------------------------=
// FUNC: rgb
// DESC: Unpacks the shared sapphire design recipe without allocating.
// ------------------=
fn rgb(value:u32)->(u8,u8,u8) {((value>>16) as u8,(value>>8) as u8,value as u8)}

impl DisplayDevice {
    // ------------------------=
    // FUNC: browser_window
    // DESC: Composes native controls and an owned engine surface in a dedicated retained window slot.
    // ------------------=
    pub(super) fn browser_window(&mut self,state:DesktopAppWindowState) {
        let window=crate::ui::system_layout::SystemLayout::new(self.width,self.height)
            .desktop_app_window_geometry(state.x,state.y,state.width,state.height,state.maximized).window;
        let view=crate::runtime::browser::presentation();
        let scale=self.ui_scale().max(1).min((window.width as usize/760).max(1));
        let Some(layout)=Layout::new(window.width,window.height,scale as u32) else {return;};
        if !self.recording_surface {
            unsafe {
                if REVISION!=Some(view.revision) && infinity_browser_core::damage::chrome_only(PAGE_KEY,view.page_key()) {
                    super::retained_windows::invalidate_region(12,super::PresentRegion {
                        left:window.x.max(0) as usize,top:window.y.max(0) as usize,
                        right:window.right().max(0) as usize,bottom:(window.y+layout.content.y).max(0) as usize});
                    REVISION=Some(view.revision);
                } else {super::retained_windows::invalidate_revision(12,&mut *(&raw mut REVISION),Some(view.revision));}
                PAGE_KEY=Some(view.page_key());
            }
            self.retained_window(12,(window.x.max(0) as usize,window.y.max(0) as usize,
                window.width as usize,window.height as usize),|display|display.browser_window(state));
            return;
        }
        let left=window.x.max(0) as usize;let top=window.y.max(0) as usize;
        self.glass_panel(left,top,window.width as usize,window.height as usize,true);
        self.fill_rect_alpha(left+1,top+1,window.width as usize-2,layout.content.y as usize-1,8,17,30,238);
        let offset=|r:Viewport|Viewport{x:r.x+window.x,y:r.y+window.y,..r};
        self.paint_bitmap_alpha_fit_rect(ICON,left+12*scale,top+4*scale,32*scale,32*scale);
        self.browser_label(Viewport{x:(left+50*scale) as i32,y:(top+11*scale) as i32,width:200*scale as u32,height:24*scale as u32},b"Infinity Browser",18*scale,false);
        if window.width as usize>950*scale {
            self.browser_label(Viewport{x:(left+270*scale) as i32,y:(top+13*scale) as i32,
                width:window.width.saturating_sub(430*scale as u32),height:20*scale as u32},
                &view.title[..view.title_length],14*scale,false);
        }
        for (index,r) in [layout.minimize,layout.maximize,layout.close].into_iter().enumerate() {
            let r=offset(r);self.window_control(r.x as usize,r.y as usize,r.width as usize,index,state.maximized);
        }
        for index in 0..view.tab_count {
            let Some((tab,close))=layout.tab(index,view.tab_count) else {continue;};
            let tab=offset(tab);let close=offset(close);let entry=&view.tabs[index];
            self.browser_tab(tab,entry.id==view.active_tab,entry.id==view.hovered_tab);
            self.browser_label(Viewport{x:tab.x+16*scale as i32,y:tab.y+10*scale as i32,
                width:tab.width.saturating_sub(56*scale as u32),height:20*scale as u32},
                if entry.length==0 {b"New tab"}else{&entry.title[..entry.length]},14*scale,false);
            if view.hovered_tab==entry.id && view.hovered_close {
                self.fill_rounded_rect_alpha(close.x as usize,close.y as usize,close.width as usize,close.height as usize,5*scale,92,76,158,170);
            }
            let cx=close.x+close.width as i32/2;let cy=close.y+close.height as i32/2;
            let d=4*scale as i32;
            self.line(cx-d,cy-d,cx+d,cy+d,217,227,245);
            self.line(cx+d,cy-d,cx-d,cy+d,217,227,245);
        }
        let new_tab=offset(layout.new_tab);
        self.browser_surface(new_tab,skin::button(false,skin::Interaction::Normal));
        self.ui_text_elided_strong(new_tab.x as usize+10*scale,new_tab.y as usize+8*scale,16*scale,b"+",231,242,250);
        for (rect,glyph,enabled) in [(layout.back,0,view.history&1!=0),(layout.forward,1,view.history&2!=0),
            (layout.reload,2,true),(layout.downloads,4,true),(layout.menu,6,true)] {
            let r=offset(rect);
            self.browser_surface(r,skin::button(false,if enabled {skin::Interaction::Normal}else{skin::Interaction::Disabled}));
            self.paint_bitmap_alpha_atlas_cell(NAV,4,2,glyph,r.x as usize+6*scale,r.y as usize+6*scale,32*scale);
        }
        let address=offset(layout.address);
        self.browser_surface(address,skin::ADDRESS);
        self.paint_bitmap_alpha_atlas_cell(NAV,4,2,5,address.x as usize+8*scale,address.y as usize+10*scale,24*scale);
        let text=if view.address_focused {&view.edit[..view.edit_length]}else{&view.address[..view.address_length]};
        let available=(address.width as usize).saturating_sub(48*scale);
        let caret=view.caret.min(text.len());
        let mut start=0;
        if view.address_focused {
            let mut end=caret;
            while !view.address_selected && start<end {
                let middle=(start+end)/2;
                if self.template_text_width(&text[middle..caret],14*scale,false)>available.saturating_sub(3) {start=middle+1;}
                else {end=middle;}
            }
            self.outline_rounded_rect(address.x as usize,address.y as usize,address.width as usize,address.height as usize,10,0,215,255);
        }
        if view.address_focused && view.address_selected {
            let width=self.template_text_width(text,14*scale,false).min(available);
            self.fill_rect(address.x as usize+40*scale,address.y as usize+12*scale,width,22*scale,18,91,132);
        }
        self.browser_label(Viewport{x:address.x+40*scale as i32,y:address.y+14*scale as i32,
            width:available as u32,height:20*scale as u32},&text[start..],14*scale,false);
        if view.address_focused && view.caret_visible && !view.address_selected {
            let x=self.template_text_width(&text[start..caret],14*scale,false);
            self.fill_rect(address.x as usize+40*scale+x,address.y as usize+12*scale,2,20*scale,0,215,255);
        }
        let go=offset(layout.go);
        self.browser_surface(go,skin::button(false,skin::Interaction::Normal));
        self.browser_label(Viewport{y:go.y+13*scale as i32,..go},b"Go",16*scale,true);
        let content=offset(layout.content);
        if self.clipped_render_region(content.x.max(0) as usize,content.y.max(0) as usize,
            content.width as usize,(window.bottom()-(content.y)).max(0) as usize).is_none() {return;}
        self.fill_rect(content.x as usize,content.y as usize,content.width as usize,content.height as usize,247,248,250);
        if view.error!=0 {
            self.ui_text_elided_strong(content.x as usize+32*scale,content.y as usize+32*scale,
                content.width as usize-64*scale,b"This page could not be opened",22,45,66);
            self.ui_text_elided_strong(content.x as usize+32*scale,content.y as usize+68*scale,
                content.width as usize-64*scale,b"Check network permission and connection, then reload or enter another address.",53,79,101);
        } else { unsafe {
            let generation=crate::runtime::browser::frame_generation();
            let current=&mut *(&raw mut FRAME);
            if current.as_ref().is_some_and(|frame|frame.generation()!=generation) {*current=None;}
            if let Some(frame)=crate::runtime::browser::FRAMES.acquire(generation) {*current=Some(frame);}
            if let Some(frame)=current.as_ref() {
                let (width,height)=frame.size();
                if (width,height)==(content.width,content.height) {
                    self.browser_pixels(content,frame.bytes());
                }
            }
        }}
        if view.permission!=0 {
            let card=offset(layout.download_card);
            self.glass_panel(card.x as usize,card.y as usize,card.width as usize,card.height as usize,true);
            self.ui_text_elided_strong(card.x as usize+16*scale,card.y as usize+10*scale,
                card.width as usize-244*scale,if view.permission==2 {b"An authorized operator is required"}else{b"Allow browser network access?"},231,242,250);
            self.ui_text_elided_strong(card.x as usize+16*scale,card.y as usize+38*scale,
                card.width as usize-244*scale,b"10 minutes. Network policy still applies.",168,196,216);
            self.browser_download_button(offset(layout.download_save),b"Allow",true,scale);
            self.browser_download_button(offset(layout.download_discard),b"Cancel",false,scale);
        } else if view.download_state!=0 {
            let card=offset(layout.download_card);
            self.glass_panel(card.x as usize,card.y as usize,card.width as usize,card.height as usize,true);
            let title:&[u8]=match view.download_state {2=>b"Saved to Downloads",3=>b"Save failed - file retained for retry",_=>b"Save this download?"};
            self.ui_text_elided_strong(card.x as usize+16*scale,card.y as usize+10*scale,
                card.width as usize-244*scale,title,231,242,250);
            self.ui_text_elided_strong(card.x as usize+16*scale,card.y as usize+38*scale,
                card.width as usize-244*scale,&view.download_name[..view.download_length],168,196,216);
            if view.download_state!=2 {self.browser_download_button(offset(layout.download_save),b"Save",true,scale);}
            self.browser_download_button(offset(layout.download_discard),if view.download_state==2 {b"Done"}else{b"Discard"},false,scale);
        }
        let status=offset(layout.status);
        let message:&[u8]=if view.input_busy {b"Input queue busy. Please retry the last input."}
            else if view.error!=0 {b"Page could not be loaded. Check permissions and connection."}
            else if view.loading {b"Loading..."} else {b"Ready"};
        self.browser_label(Viewport{x:status.x+16*scale as i32,y:status.y+5*scale as i32,
            width:status.width-32*scale as u32,..status},message,12*scale,false);
    }
    // ------------------------=
    // FUNC: browser_label
    // DESC: Measures and elides authored-size chrome typography without inheriting oversized desktop labels.
    // ------------------=
    fn browser_label(&mut self,r:Viewport,text:&[u8],pixels:usize,center:bool) {
        let mut label=[0u8;2048];
        let length=text.len().min(label.len()-3);
        label[..length].copy_from_slice(&text[..length]);
        let mut end=length;
        if self.template_text_width(&label[..end],pixels,false)>r.width as usize {
            let dots=self.template_text_width(b"...",pixels,false);
            if dots>r.width as usize {return;}
            let mut low=0;let mut high=end;
            while low<high {
                let middle=(low+high+1)/2;
                if self.template_text_width(&label[..middle],pixels,false)+dots<=r.width as usize {low=middle;}else{high=middle-1;}
            }
            end=low;label[end..end+3].copy_from_slice(b"...");end+=3;
        }
        let width=self.template_text_width(&label[..end],pixels,false);
        let x=r.x.max(0) as usize+if center {(r.width as usize).saturating_sub(width)/2}else{0};
        self.template_text(x,r.y.max(0) as usize,&label[..end],231,242,250,255,pixels,false);
    }
    // ------------------------=
    // FUNC: browser_tab
    // DESC: Paints the horizontal AI-adornment glass silhouette with antialiased sapphire-violet shoulders.
    // ------------------=
    fn browser_tab(&mut self,r:Viewport,active:bool,hover:bool) {
        for y in 0..r.height {for x in 0..r.width {
            let (red,green,blue,alpha)=infinity_browser_core::tab_style::pixel(r.width,r.height,x,y,active,hover);
            if alpha!=0 {self.blend_color(r.x+x as i32,r.y+y as i32,red,green,blue,alpha);}
        }}
    }
    // ------------------------=
    // FUNC: browser_surface
    // DESC: Paints the shared gradient and soft border recipe at exact chrome geometry.
    // ------------------=
    fn browser_surface(&mut self,r:Viewport,surface:skin::Surface) {
        let (red,green,blue)=rgb(surface.bottom);
        self.fill_rounded_rect_alpha(r.x as usize,r.y as usize,r.width as usize,r.height as usize,10,red,green,blue,255);
        let (red,green,blue)=rgb(surface.top);
        self.fill_rounded_rect_alpha(r.x as usize+1,r.y as usize+1,r.width as usize-2,r.height as usize/2,8,red,green,blue,180);
        let (red,green,blue)=rgb(surface.border);
        self.outline_rounded_rect(r.x as usize,r.y as usize,r.width as usize,r.height as usize,10,red,green,blue);
    }
    // ------------------------=
    // FUNC: browser_download_button
    // DESC: Reuses the browser kit surface with centered native save controls and no decorative action glyph.
    // ------------------=
    fn browser_download_button(&mut self,r:Viewport,label:&[u8],primary:bool,scale:usize) {
        self.browser_surface(r,skin::button(primary,skin::Interaction::Normal));
        let width=self.ui_text_width_weighted(label,1,true);
        self.ui_text_elided_strong(r.x as usize+(r.width as usize).saturating_sub(width)/2,r.y as usize+9*scale,
            r.width as usize,label,231,242,250);
    }
}
