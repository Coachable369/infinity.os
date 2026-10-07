//! Native browser chrome and retained viewport. Servo never owns desktop drawing.
use super::DisplayDevice;
use infinity_browser_core::{layout::Layout,skin,Viewport,frames::Frame};
use crate::ui::system_layout::DesktopAppWindowState;
static mut REVISION:Option<u64>=None;
static mut PAGE_KEY:Option<infinity_browser_core::damage::PageKey>=None;
static mut FRAME:Option<Frame<'static,16384000>>=None;

// ------------------------=
// FUNC: rgb
// DESC: Unpacks the shared sapphire design recipe without allocating.
// ------------------=
fn rgb(value:u32)->(u8,u8,u8) {((value>>16) as u8,(value>>8) as u8,value as u8)}

// ------------------------=
// FUNC: browser_font
// DESC: Selects an authored Inter atlas at native device size, avoiding rescaled installer glyphs.
// ------------------=
fn browser_font(pixels:usize)->(&'static [u8],&'static [u8],&'static [u8],usize) {
    macro_rules! face {($size:literal,$pixels:expr)=>{(
        include_bytes!(concat!("../../../assets/fonts/InfinityBrowser-Regular-",$size,".atlas")).as_slice(),
        include_bytes!(concat!("../../../assets/fonts/InfinityBrowser-Regular-",$size,".metrics")).as_slice(),
        include_bytes!(concat!("../../../assets/fonts/InfinityBrowser-Regular-",$size,".kern")).as_slice(),$pixels)}}
    match (pixels/14).clamp(1,4) {1=>face!("14",14),2=>face!("28",28),3=>face!("42",42),_=>face!("56",56)}
}

// ------------------------=
// FUNC: browser_text_width
// DESC: Measures the exact native Inter advances and kerning used by browser chrome painting.
// ------------------=
fn browser_text_width(text:&[u8],pixels:usize)->usize {
    let (_,metrics,kern,_)=browser_font(pixels);let mut width=0usize;let mut previous=None;
    for &byte in text {if !(32..=126).contains(&byte) {previous=None;continue;}
        let index=(byte-32) as usize;
        if let Some(left)=previous {width=width.saturating_add_signed(kern[left*95+index] as isize-128);}
        width+=metrics[index] as usize;previous=Some(index);
    }width
}

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
        let tab_count=if view.welcome_open {1}else{view.tab_count};
        let Some(layout)=Layout::new(window.width,window.height,scale as u32).map(|layout|layout.with_tab_count(tab_count).with_favorites(view.settings.favorites)) else {return;};
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
        self.fill_rounded_rect_alpha(left,top,window.width as usize,window.height as usize,8*scale,7,14,26,255);
        self.outline_rounded_rect(left,top,window.width as usize,window.height as usize,8*scale,65,80,112);
        self.fill_rect_alpha(left+1,top+1,window.width as usize-2,layout.content.y as usize-1,8,17,30,238);
        let offset=|r:Viewport|Viewport{x:r.x+window.x,y:r.y+window.y,..r};
        for (index,(red,green,blue)) in [(255,91,87),(255,194,48),(42,195,72)].into_iter().enumerate() {
            self.fill_rounded_rect_alpha(left+(18+index*22)*scale,top+18*scale,12*scale,12*scale,6*scale,red,green,blue,255);
        }
        for (index,r) in [layout.minimize,layout.maximize,layout.close].into_iter().enumerate() {
            let r=offset(r);self.browser_glyph(r,7+index,true,scale);
        }
        for index in 0..tab_count {
            let Some((tab,close))=layout.tab(index,tab_count) else {continue;};
            let tab=offset(tab);let close=offset(close);let entry=&view.tabs[index];
            self.browser_tab(tab,view.welcome_open || entry.id==view.active_tab,entry.id==view.hovered_tab);
            if tab.width>=112*scale as u32 {
                self.browser_glyph(Viewport{x:tab.x+28*scale as i32,y:tab.y+10*scale as i32,width:16*scale as u32,height:16*scale as u32},10,true,scale);
            }
            self.browser_label(Viewport{x:tab.x+52*scale as i32,y:tab.y+11*scale as i32,
                width:tab.width.saturating_sub(100*scale as u32),height:20*scale as u32},
                if view.welcome_open {b"Welcome"}else if entry.length==0 {b"New tab"}else{&entry.title[..entry.length]},14*scale,false);
            if view.hovered_tab==entry.id && view.hovered_close {
                self.fill_rounded_rect_alpha(close.x as usize,close.y as usize,close.width as usize,close.height as usize,5*scale,92,76,158,170);
            }
            let cx=close.x+close.width as i32/2;let cy=close.y+close.height as i32/2;
            let d=4*scale as i32;
            self.line(cx-d,cy-d,cx+d,cy+d,217,227,245);
            self.line(cx+d,cy-d,cx-d,cy+d,217,227,245);
        }
        let new_tab=offset(layout.new_tab);
        self.browser_glyph(new_tab,11,true,scale);
        for (r,name,id) in [(layout.file_menu,b"File" as &[u8],1),(layout.settings_menu,b"Settings" as &[u8],2),(layout.edit_menu,b"Edit" as &[u8],3),(layout.view_menu,b"View" as &[u8],4)] {
            let r=offset(r);
            if view.chrome_menu==id {self.browser_surface(r,skin::button(false,skin::Interaction::Hovered));}
            self.browser_label(Viewport{y:r.y+9*scale as i32,..r},name,14*scale,true);
        }
        for (rect,glyph,enabled) in [(layout.back,0,view.history&1!=0),(layout.forward,1,view.history&2!=0),
            (layout.reload,2,true),(layout.downloads,4,true),(layout.menu,13,true)] {
            let r=offset(rect);
            self.browser_glyph(r,glyph,enabled,scale);
        }
        let address=offset(layout.address);
        self.browser_surface(address,skin::ADDRESS);
        self.browser_glyph(Viewport{x:address.x+8*scale as i32,y:address.y+4*scale as i32,width:24*scale as u32,height:24*scale as u32},if view.welcome_open {14}else{5},true,scale);
        let text=if view.address_focused {&view.edit[..view.edit_length]}else{&view.address[..view.address_length]};
        let available=(address.width as usize).saturating_sub(48*scale);
        let caret=view.caret.min(text.len());
        let mut start=0;
        if view.address_focused {
            let mut end=caret;
            while !view.address_selected && start<end {
                let middle=(start+end)/2;
                if browser_text_width(&text[middle..caret],14*scale)>available.saturating_sub(3) {start=middle+1;}
                else {end=middle;}
            }
            self.outline_rounded_rect(address.x as usize,address.y as usize,address.width as usize,address.height as usize,10,0,215,255);
        }
        if view.address_focused && view.address_selected {
            let width=browser_text_width(text,14*scale).min(available);
            self.fill_rect(address.x as usize+40*scale,address.y as usize+7*scale,width,18*scale,18,91,132);
        }
        self.browser_label(Viewport{x:address.x+40*scale as i32,y:address.y+9*scale as i32,
            width:available as u32,height:20*scale as u32},if view.welcome_open && text.is_empty() && !view.address_focused {b"Search or enter an address"}else{&text[start..]},14*scale,false);
        if view.address_focused && view.caret_visible && !view.address_selected {
            let x=browser_text_width(&text[start..caret],14*scale);
            self.fill_rect(address.x as usize+40*scale+x,address.y as usize+7*scale,scale,18*scale,134,158,255);
        }
        let go=offset(layout.go);
        self.browser_glyph(go,1,true,scale);
        if view.settings.favorites {
        let star=offset(layout.favorite);
        if view.favorite_saved {self.browser_surface(star,skin::button(true,skin::Interaction::Normal));}
        self.browser_glyph(star,12,true,scale);
        if view.favorite_count==0 {
            self.browser_label(Viewport{x:star.x+36*scale as i32,y:star.y+7*scale as i32,
                width:layout.favorites.width-160*scale as u32,height:20*scale as u32},b"Save a favorite with the star or Ctrl+D",14*scale,false);
        }
        for slot in 0..layout.favorite_slots() {
            let index=view.favorite_offset+slot;if index>=view.favorite_count {break;}
            let r=offset(layout.favorite_item(slot).unwrap());let favorite=&view.favorites[index];
            if view.hovered_favorite==index {self.browser_surface(r,skin::button(false,skin::Interaction::Hovered));}
            self.browser_label(Viewport{x:r.x+8*scale as i32,y:r.y+7*scale as i32,width:r.width-16*scale as u32,..r},
                &favorite.title[..favorite.length],14*scale,false);
        }
        if view.favorite_count>layout.favorite_slots() || view.favorite_offset>0 {
            self.browser_glyph(offset(layout.favorites_previous),0,view.favorite_offset>0,scale);
            self.browser_glyph(offset(layout.favorites_next),1,view.favorite_offset+layout.favorite_slots()<view.favorite_count,scale);
        }
        }
        self.fill_rect(left,top+layout.content.y as usize-1,window.width as usize,1,33,58,82);
        let content=offset(layout.content);
        let chrome_clip=self.render_clip;
        if self.clipped_render_region(content.x.max(0) as usize,content.y.max(0) as usize,
            content.width as usize,(window.bottom()-(content.y)).max(0) as usize).is_none() {return;}
        self.fill_rect(content.x as usize,content.y as usize,content.width as usize,content.height as usize,247,248,250);
        if view.settings_open {
            self.fill_rect(content.x as usize,content.y as usize,content.width as usize,content.height as usize,9,20,33);
            let visible=content;
            let content=Viewport{y:content.y-view.settings_scroll as i32,..content};
            let label=|x:u32,y:u32,w:u32|Viewport{x:content.x+(x*scale as u32) as i32,y:content.y+(y*scale as u32) as i32,width:w*scale as u32,height:20*scale as u32};
            for (y,text) in [(20,b"Browser settings" as &[u8]),(56,b"Search engine for the address bar"),
                (152,b"Show favorites bar"),(208,b"Saved favorites")] {
                let r=label(24,y,if y<100 {440}else{288});
                if r.y>=visible.y && r.y+r.height as i32<=visible.y+visible.height as i32 {self.browser_label(r,text,14*scale,false);}
            }
            for index in 0..7 {
                let text:&[u8]=match index {0=>b"Fast Search",1=>b"DuckDuckGo",2=>b"Bing",
                    3=>if view.settings.favorites {b"On"}else{b"Off"},
                    4=>if view.settings_confirm {b"Confirm clear"}else{b"Clear favorites"},5=>b"Restore defaults",_=>b"Back to page"};
                let r=infinity_browser_core::settings::control(content,scale as u32,index);
                if r.y<visible.y || r.y+r.height as i32>visible.y+visible.height as i32 {continue;}
                self.browser_surface(r,skin::button(index==view.settings.search as usize || index==3&&view.settings.favorites,skin::Interaction::Normal));
                self.browser_label(Viewport{y:r.y+9*scale as i32,..r},text,14*scale,true);
                if index==view.settings_focus {self.outline_rounded_rect(r.x.max(0) as usize,r.y.max(0) as usize,r.width as usize,r.height as usize,8,34,211,238);}
            }
            let note:&[u8]=match view.settings_notice {1=>b"Saved to your profile.",2=>b"Could not load or save preferences. Check storage and retry.",
                _=>if view.settings_confirm {b"Click Confirm clear to permanently remove your favorites."}else{b"Changes save automatically. Downloads always require your approval."}};
            let r=label(24,240,690);
            if r.y>=visible.y && r.y+r.height as i32<=visible.y+visible.height as i32 {self.browser_label(r,note,14*scale,false);}
        } else if view.welcome_open {
            self.browser_welcome(content,scale,view.welcome_focus,view.welcome_scroll);
        } else if view.error!=0 {
            use infinity_browser_core::startup::Error;
            let detail:&[u8]=match view.error {
                value if value==Error::Entropy as u32=>b"Secure randomness unavailable. Enable firmware RNG or TPM 2.0, then restart InfinityOS.",
                value if value==Error::Clock as u32=>b"Trusted firmware time unavailable. Check the firmware clock, then retry.",
                value if value==Error::Viewport as u32=>b"This viewport is unsupported. Restore the browser window, then reload.",
                value if value==Error::Worker as u32=>b"Browser worker unavailable. Restart InfinityOS, then retry.",
                _=>b"Check network permission and connection, then reload or enter another address.",
            };
            self.ui_text_elided_strong(content.x as usize+32*scale,content.y as usize+32*scale,
                content.width as usize-64*scale,b"This page could not be opened",22,45,66);
            self.ui_text_elided_strong(content.x as usize+32*scale,content.y as usize+68*scale,
                content.width as usize-64*scale,detail,53,79,101);
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
        if !view.settings_open && view.permission!=0 {
            let card=offset(layout.download_card);
            self.glass_panel(card.x as usize,card.y as usize,card.width as usize,card.height as usize,true);
            self.ui_text_elided_strong(card.x as usize+16*scale,card.y as usize+10*scale,
                card.width as usize-244*scale,b"Browser network access unavailable",231,242,250);
            self.ui_text_elided_strong(card.x as usize+16*scale,card.y as usize+38*scale,
                card.width as usize-244*scale,b"Check your session and Network Settings.",168,196,216);
            self.browser_download_button(offset(layout.download_save),b"Retry",true,scale);
            self.browser_download_button(offset(layout.download_discard),b"Cancel",false,scale);
        } else if !view.settings_open && view.download_state!=0 {
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
        let mut message=[0u8;2304];
        let footer=view.footer_status();let prefix=if view.settings_open {b"Browser settings - private to your profile" as &[u8]}else if view.welcome_open {b"Welcome - ready to browse"}else{footer.label()};
        message[..prefix.len()].copy_from_slice(prefix);let mut length=prefix.len();
        if !view.settings_open && footer==infinity_browser_core::page_status::Status::Loading {
            message[length..length+view.address_length].copy_from_slice(&view.address[..view.address_length]);length+=view.address_length;
        }
        self.fill_rect(status.x as usize,status.y as usize,status.width as usize,status.height as usize,9,20,33);
        self.fill_rect(status.x as usize,status.y as usize,status.width as usize,1,33,58,82);
        let color=if view.error!=0 || view.favorite_error!=0 || view.permission!=0 {(245,165,95)}else if view.loading {(34,211,238)}else{(112,162,180)};
        self.fill_rounded_rect_alpha(status.x as usize+16*scale,status.y as usize+9*scale,6*scale,6*scale,3*scale,color.0,color.1,color.2,255);
        self.browser_label(Viewport{x:status.x+30*scale as i32,y:status.y+5*scale as i32,
            width:status.width-46*scale as u32,..status},&message[..length],14*scale,false);
        self.render_clip=chrome_clip;
        if view.find_open {
            let r=offset(layout.find_control(0));
            self.browser_surface(Viewport{x:content.x,y:content.y,width:content.width,height:40*scale as u32},skin::button(false,skin::Interaction::Normal));
            self.browser_surface(r,skin::button(false,skin::Interaction::Hovered));
            self.browser_label(Viewport{x:r.x+12*scale as i32,y:r.y+9*scale as i32,width:r.width.saturating_sub(150*scale as u32),..r},
                if view.find_length==0 {b"Find on page..."}else{&view.find_text[..view.find_length]},14*scale,true);
            let mut count=[0u8;32];let mut n=0;
            if view.find_result==u32::MAX {count[..12].copy_from_slice(b"Search error");n=12;}
            else {for (i,value) in [view.find_result>>16,view.find_result&65535].iter().enumerate() {
                if i==1 {count[n]=b'/';n+=1;}
                let mut digits=[0u8;10];let mut used=0;let mut v=*value;
                loop {digits[used]=b'0'+(v%10) as u8;used+=1;v/=10;if v==0 {break;}}
                for d in digits[..used].iter().rev() {count[n]=*d;n+=1;}
            }}
            self.browser_label(Viewport{x:r.x+r.width as i32-100*scale as i32,y:r.y+9*scale as i32,width:96*scale as u32,..r},&count[..n],14*scale,false);
            for (i,label) in [(1,b"<" as &[u8]),(2,b">"),(3,b"X")] {let r=offset(layout.find_control(i));
                self.browser_surface(r,skin::button(false,skin::Interaction::Normal));self.browser_label(Viewport{y:r.y+9*scale as i32,..r},label,14*scale,true);}
        }
        for index in 0..if view.chrome_menu==4 {tab_count}else{3} {
            let Some(r)=layout.menu_item(view.chrome_menu,index) else {continue;};let r=offset(r);
            self.browser_surface(r,skin::button(false,if index==view.menu_focus {skin::Interaction::Hovered}else{skin::Interaction::Normal}));
            let name:&[u8]=match view.chrome_menu {2=>b"Browser settings",3=>b"Find on page    Ctrl+F",4=>{
                let tab=&view.tabs[index];if view.welcome_open {b"Welcome"}else if tab.length==0 {b"New tab"}else{&tab.title[..tab.length]}
            },_=>match index {0=>b"New tab     Ctrl+T",1=>b"Close tab   Ctrl+W",_=>b"Close window"}};
            if view.chrome_menu==4 && view.tabs[index].id==view.active_tab {self.fill_rect(r.x as usize+3*scale,r.y as usize+10*scale,3*scale,12*scale,34,211,238);}
            self.browser_label(Viewport{x:r.x+12*scale as i32,y:r.y+(r.height.saturating_sub(14*scale as u32)/2) as i32,width:r.width-24*scale as u32,..r},name,14*scale,false);
        }
    }
    // ------------------------=
    // FUNC: browser_label
    // DESC: Measures and elides authored-size chrome typography without inheriting oversized desktop labels.
    // ------------------=
    fn browser_label(&mut self,r:Viewport,text:&[u8],pixels:usize,center:bool) {
        self.browser_label_color(r,text,pixels,center,(222,228,246));
    }
    // ------------------------=
    // FUNC: browser_label_color
    // DESC: Uses the same authored Inter metrics for semantic welcome typography colors.
    // ------------------=
    fn browser_label_color(&mut self,r:Viewport,text:&[u8],pixels:usize,center:bool,color:(u8,u8,u8)) {
        let mut label=[0u8;2048];
        let length=text.len().min(label.len()-3);
        label[..length].copy_from_slice(&text[..length]);
        let mut end=length;
        if browser_text_width(&label[..end],pixels)>r.width as usize {
            let dots=browser_text_width(b"...",pixels);
            if dots>r.width as usize {return;}
            let mut low=0;let mut high=end;
            while low<high {
                let middle=(low+high+1)/2;
                if browser_text_width(&label[..middle],pixels)+dots<=r.width as usize {low=middle;}else{high=middle-1;}
            }
            end=low;label[end..end+3].copy_from_slice(b"...");end+=3;
        }
        let width=browser_text_width(&label[..end],pixels);
        let mut x=r.x.max(0) as usize+if center {(r.width as usize).saturating_sub(width)/2}else{0};
        let (atlas,metrics,kern,size)=browser_font(pixels);let mut previous=None;
        for &byte in &label[..end] {if !(32..=126).contains(&byte) {previous=None;continue;}
            let index=(byte-32) as usize;
            if let Some(left)=previous {x=x.saturating_add_signed(kern[left*95+index] as isize-128);}
            for gy in 0..size+6 {for gx in 0..size {
                let alpha=atlas[gy*size*95+index*size+gx];
                if alpha!=0 {self.blend_color((x+gx) as i32,r.y+gy as i32,color.0,color.1,color.2,alpha);}
            }}
            x+=metrics[index] as usize;previous=Some(index);
        }
    }
    // ------------------------=
    // FUNC: browser_welcome
    // DESC: Renders the approved charcoal composition with real native controls and transparent hero artwork.
    // ------------------=
    fn browser_welcome(&mut self,content:Viewport,scale:usize,focus:usize,scroll:u32) {
        let old_clip=self.render_clip;
        self.fill_rect(content.x as usize,content.y as usize,content.width as usize,content.height as usize,32,33,36);
        let Some(clip)=self.clipped_render_region(content.x.max(0) as usize,content.y.max(0) as usize,content.width as usize,content.height as usize) else {return;};
        self.render_clip=Some(clip);
        let l=infinity_browser_core::welcome::Layout::new(content,scale as u32,scroll);
        let width=l.area.width/scale as u32;
        let copy_width=if l.narrow {width-64}else{width/2-24};
        for (y,text,color,size) in [(24,b"Welcome to" as &[u8],(248,249,250),28),
            (62,b"Infinity Browser",(34,211,238),28),(110,b"Your next discovery starts here.",(196,200,207),14),
            (132,b"Search, explore, and make the web your own.",(196,200,207),14)] {
            let r=l.rect(32,y,copy_width,36);
            if r.y>=content.y && r.y+r.height as i32<=content.y+content.height as i32 {self.browser_label_color(r,text,size*scale,false,color);}
        }
        let hero=l.hero();
        let bitmap=infinity_browser_core::welcome::HERO;
        let sw=u32::from_le_bytes(bitmap[18..22].try_into().unwrap()) as usize;
        let sh=i32::from_le_bytes(bitmap[22..26].try_into().unwrap()).unsigned_abs() as usize;
        let w=(hero.height as usize*sw/sh).min(hero.width as usize);let h=w*sh/sw;
        let y=hero.y+(hero.height as i32-h as i32)/2;
        if y>=0 {self.paint_bitmap_alpha_fit_rect(bitmap,hero.x as usize+(hero.width as usize-w)/2,y as usize,w,h);}
        for index in 0..2 {
            let r=l.button(index);if r.y<content.y || r.y+r.height as i32>content.y+content.height as i32 {continue;}
            self.browser_surface(r,skin::button(index==0,if index==focus {skin::Interaction::Hovered}else{skin::Interaction::Normal}));
            self.browser_label_color(Viewport{y:r.y+10*scale as i32,..r},if index==0 {b"Start browsing"}else{b"Browser settings"},14*scale,true,if index==0 {(3,27,44)}else{(231,242,250)});
            if index==focus {self.outline_rounded_rect(r.x as usize,r.y as usize,r.width as usize,r.height as usize,8*scale,34,211,238);}
        }
        for (index,(title,detail)) in [(b"Keep your place" as &[u8],b"Switch open tabs from View." as &[u8]),
            (b"Save what matters",b"Star a page to add it to Favorites."),(b"Find it faster",b"Search page text with Ctrl+F.")].into_iter().enumerate() {
            let r=l.card(index);if r.y<content.y || r.y+r.height as i32>content.y+content.height as i32 {continue;}
            self.fill_rounded_rect_alpha(r.x as usize,r.y as usize,r.width as usize,r.height as usize,10*scale,41,43,47,255);
            self.outline_rounded_rect(r.x as usize,r.y as usize,r.width as usize,r.height as usize,10*scale,68,71,77);
            let inset=if l.narrow {56}else{12}*scale as i32;
            let title_y=if l.narrow {10}else{44}*scale as i32;
            let text=Viewport{x:r.x+inset,y:r.y+title_y,width:r.width-inset as u32-12*scale as u32,height:20*scale as u32};
            self.browser_label_color(text,title,14*scale,!l.narrow,(248,249,250));
            self.browser_label_color(Viewport{y:text.y+24*scale as i32,..text},detail,14*scale,!l.narrow,(196,200,207));
            let icon=Viewport{x:if l.narrow {r.x+12*scale as i32}else{r.x+r.width as i32/2-12*scale as i32},y:r.y+10*scale as i32,width:24*scale as u32,height:24*scale as u32};
            self.browser_glyph(icon,if index==0 {15}else if index==1 {16}else{14},true,scale);
        }
        let r=l.rect(32,if l.narrow {622}else{350},width-64,20);
        if r.y>=content.y && r.y+r.height as i32<=content.y+content.height as i32 {
            self.browser_label_color(r,b"Infinity AI is one click away on the side of your window.",14*scale,false,(164,172,182));
        }
        self.render_clip=old_clip;
    }
    // ------------------------=
    // FUNC: browser_tab
    // DESC: Paints the horizontal AI-adornment glass silhouette with antialiased sapphire-violet shoulders.
    // ------------------=
    fn browser_tab(&mut self,r:Viewport,active:bool,hover:bool) {
        let radius=(r.height/36).max(1) as i32*6;
        for y in -radius..r.height as i32 {for x in -radius..r.width as i32+radius {
            let (red,green,blue,alpha)=infinity_browser_core::tab_style::halo(r.width,r.height,x,y,active,hover);
            if alpha!=0 {self.blend_color(r.x+x,r.y+y,red,green,blue,alpha);}
        }}
        for y in 0..r.height {for x in 0..r.width {
            let (red,green,blue,alpha)=infinity_browser_core::tab_style::pixel(r.width,r.height,x,y,active,hover);
            if alpha!=0 {self.blend_color(r.x+x as i32,r.y+y as i32,red,green,blue,alpha);}
        }}
    }
    // ------------------------=
    // FUNC: browser_glyph
    // DESC: Draws quiet kit-matched chrome strokes at native resolution without boxed bitmap buttons.
    // ------------------=
    fn browser_glyph(&mut self,r:Viewport,glyph:usize,enabled:bool,scale:usize) {
        let cx=r.x+r.width as i32/2;let cy=r.y+r.height as i32/2;let s=scale as i32;
        let color=if enabled {if glyph>=14 {(34,211,238)}else{(196,207,232)}}else{(91,108,139)};
        let mut stroke=|x1:i32,y1:i32,x2:i32,y2:i32| {
            for offset in 0..scale as i32 {self.line(cx+x1*s,cy+y1*s+offset,cx+x2*s,cy+y2*s+offset,color.0,color.1,color.2);}
        };
        match glyph {
            0|1=>{let sign=if glyph==0 {-1}else{1};stroke(-7,0,7,0);stroke(sign*7,0,sign, -6);stroke(sign*7,0,sign,6);}
            2=>{for (a,b,c,d) in [(-5,-5,2,-7),(2,-7,7,-3),(7,-3,7,0),(-5,-5,-7,0),(-7,0,-5,5),(-5,5,2,7),(2,7,6,4),(7,-7,7,-1),(7,-1,1,-1)] {stroke(a,b,c,d);}}
            4=>{stroke(0,-7,0,3);stroke(-4,-1,0,3);stroke(0,3,4,-1);stroke(-6,4,-6,7);stroke(-6,7,6,7);stroke(6,7,6,4);}
            5=>{for (a,b,c,d) in [(-4,-1,-4,6),(-4,6,4,6),(4,6,4,-1),(4,-1,-4,-1),(-3,-1,-3,-5),(-3,-5,0,-7),(0,-7,3,-5),(3,-5,3,-1)] {stroke(a,b,c,d);}}
            6=>{stroke(-7,0,-6,0);stroke(0,0,1,0);stroke(7,0,8,0);}
            7=>stroke(-5,0,5,0),
            8=>{stroke(-5,-5,5,-5);stroke(5,-5,5,5);stroke(5,5,-5,5);stroke(-5,5,-5,-5);}
            9=>{stroke(-5,-5,5,5);stroke(5,-5,-5,5);}
            10=>{for (a,b,c,d) in [(-5,-7,2,-7),(2,-7,5,-4),(5,-4,5,7),(5,7,-5,7),(-5,7,-5,-7),(2,-7,2,-3),(2,-3,5,-3),(-2,0,2,0),(-2,3,2,3)] {stroke(a,b,c,d);}}
            12|16=>{for (a,b,c,d) in [(0,-8,2,-3),(2,-3,8,-2),(8,-2,4,2),(4,2,5,8),(5,8,0,5),(0,5,-5,8),(-5,8,-4,2),(-4,2,-8,-2),(-8,-2,-2,-3),(-2,-3,0,-8)] {stroke(a,b,c,d);}}
            14=>{for (a,b,c,d) in [(-6,-7,0,-8),(0,-8,5,-4),(5,-4,5,1),(5,1,0,5),(0,5,-6,4),(-6,4,-9,-1),(-9,-1,-6,-7),(4,4,9,9)] {stroke(a,b,c,d);}}
            15=>{for (a,b,c,d) in [(-8,-3,3,-3),(3,-3,3,8),(3,8,-8,8),(-8,8,-8,-3),(-5,-6,6,-6),(6,-6,6,5),(-2,-9,9,-9),(9,-9,9,2)] {stroke(a,b,c,d);}}
            13=>{for (a,b,c,d) in [(-3,-6,3,-6),(3,-6,6,-3),(6,-3,6,3),(6,3,3,6),(3,6,-3,6),(-3,6,-6,3),(-6,3,-6,-3),(-6,-3,-3,-6),
                (0,-9,0,-6),(0,6,0,9),(-9,0,-6,0),(6,0,9,0),(-6,-6,-4,-4),(4,4,6,6),(-6,6,-4,4),(4,-4,6,-6),(-2,-2,2,-2),(2,-2,2,2),(2,2,-2,2),(-2,2,-2,-2)] {stroke(a,b,c,d);}}
            _=>{stroke(-7,0,7,0);stroke(0,-7,0,7);}
        }
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
