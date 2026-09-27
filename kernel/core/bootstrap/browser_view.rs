//! Native browser chrome and retained viewport. Servo never owns desktop drawing.
use super::DisplayDevice;
use infinity_browser_core::{layout::Layout,skin,Viewport,frames::Frame};
use crate::ui::system_layout::DesktopAppWindowState;
const ICON:&[u8]=include_bytes!("../../../assets/apps/infinity-browser-icon-v1.bmp");
const NAV:&[u8]=include_bytes!("../../../assets/apps/infinity-browser-navigation-v1.bmp");
static mut REVISION:Option<u64>=None;
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
        if !self.recording_surface {
            unsafe {super::retained_windows::invalidate_revision(12,&mut *(&raw mut REVISION),Some(view.revision));}
            self.retained_window(12,(window.x.max(0) as usize,window.y.max(0) as usize,
                window.width as usize,window.height as usize),|display|display.browser_window(state));
            return;
        }
        let scale=self.ui_scale().max(1).min((window.width as usize/760).max(1));
        let Some(layout)=Layout::new(window.width,window.height,scale as u32) else {return;};
        let left=window.x.max(0) as usize;let top=window.y.max(0) as usize;
        self.glass_panel(left,top,window.width as usize,window.height as usize,true);
        let offset=|r:Viewport|Viewport{x:r.x+window.x,y:r.y+window.y,..r};
        self.paint_bitmap_alpha_fit_rect(ICON,left+12*scale,top+4*scale,32*scale,32*scale);
        self.ui_text_elided_strong(left+50*scale,top+12*scale,200*scale,b"Infinity Browser",231,242,250);
        if window.width as usize>950*scale {
            self.ui_text_elided_strong(left+270*scale,top+12*scale,
                (window.width as usize).saturating_sub(430*scale),&view.title[..view.title_length],168,196,216);
        }
        for (index,r) in [layout.minimize,layout.maximize,layout.close].into_iter().enumerate() {
            let r=offset(r);self.window_control(r.x as usize,r.y as usize,r.width as usize,index,state.maximized);
        }
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
            while start<end {
                let middle=(start+end)/2;
                if self.ui_text_width_weighted(&text[middle..caret],1,true)>available.saturating_sub(3) {start=middle+1;}
                else {end=middle;}
            }
            self.outline_rounded_rect(address.x as usize,address.y as usize,address.width as usize,address.height as usize,10,0,215,255);
        }
        self.ui_text_elided_strong(address.x as usize+40*scale,address.y as usize+14*scale,
            available,&text[start..],231,242,250);
        if view.address_focused && view.caret_visible {
            let x=self.ui_text_width_weighted(&text[start..caret],1,true);
            self.fill_rect(address.x as usize+40*scale+x,address.y as usize+12*scale,2,20*scale,0,215,255);
        }
        let go=offset(layout.go);
        self.polished_button(go.x as usize,go.y as usize,go.width as usize,go.height as usize,b"Go",true,false);
        let content=offset(layout.content);
        self.fill_rect(content.x as usize,content.y as usize,content.width as usize,content.height as usize,247,248,250);
        if view.error!=0 {
            self.ui_text_elided_strong(content.x as usize+32*scale,content.y as usize+32*scale,
                content.width as usize-64*scale,b"This page could not be opened",22,45,66);
            self.ui_text_elided_strong(content.x as usize+32*scale,content.y as usize+68*scale,
                content.width as usize-64*scale,b"Check network permission and connection, then reload or enter another address.",53,79,101);
        } else { unsafe {
            let generation=crate::runtime::browser::status().2;
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
        if view.download_state!=0 {
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
        self.ui_text_elided_strong(status.x as usize+16*scale,status.y as usize+4*scale,
            status.width as usize-32*scale,message,168,196,216);
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
