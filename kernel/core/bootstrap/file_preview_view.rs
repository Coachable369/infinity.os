//! Read-only quick look, using existing native glass, Inter and bitmap recipes.
use crate::ui::file_preview::{self as preview,Kind};
impl super::DisplayDevice {
    // ------------------------=
    // FUNC: file_preview_overlay
    // DESC: Paints the authenticated quick-look sheet with clipped text, aspect-fit images and a real scroll rail.
    // ------------------=
    pub(super) fn file_preview_overlay(&mut self) {
        let v=preview::current();if !v.open{return;}
        let x=v.rect.x.max(0) as usize;let y=v.rect.y.max(0) as usize;let w=v.rect.width as usize;let h=v.rect.height as usize;let s=v.scale;
        let primary=self.active_primary_rgb();let accent=self.active_accent_rgb();
        let (r,g,b)=((primary>>16) as u8,(primary>>8) as u8,primary as u8);
        let (ar,ag,ab)=((accent>>16) as u8,(accent>>8) as u8,accent as u8);
        self.fill_rounded_rect_alpha(x,y,w,h,8*s,r/2,g/2,b/2,255);
        self.outline_rounded_rect(x,y,w,h,8*s,ar,ag,ab);
        if v.menu {self.app_text(x+16*s,y+8*s,b"Preview",(222,235,245),false,s);
            if crate::ui::user_wallpaper::image_path(&v.path[..v.path_len]){self.app_text(x+16*s,y+40*s,b"Set as desktop wallpaper...",(222,235,245),false,s);}return;}
        let mut title=v.rect;title.x+=20*s as i32;title.y+=16*s as i32;title.width=title.width.saturating_sub(80*s as u32);title.height=24*s as u32;
        self.app_label(title,crate::runtime::object_navigation::namespace_basename(&v.path[..v.path_len]),(232,241,250),true,s);
        let c=v.close_rect();let cx=c.x+c.width as i32/2;let cy=c.y+c.height as i32/2;let d=5*s as i32;
        self.line(cx-d,cy-d,cx+d,cy+d,ar,ag,ab);self.line(cx-d,cy+d,cx+d,cy-d,ar,ag,ab);
        self.fill_rect(x+16*s,y+48*s,w.saturating_sub(32*s),1,ar/2,ag/2,ab/2);
        let body=v.body();
        match v.kind {
            Kind::Png=>preview::png(|p,raw|{
                let w=body.width as usize;let h=body.height as usize;
                let (dw,dh)=if w*p.height<=h*p.width{(w,(w*p.height/p.width).max(1))}else{((h*p.width/p.height).max(1),h)};
                let left=body.x as usize+(w-dw)/2;let top=body.y as usize+(h-dh)/2;
                for y in 0..dh{for x in 0..dw{if let Some([r,g,b,a])=p.rgba(raw,x*p.width/dw,y*p.height/dh){self.blend_color((left+x) as i32,(top+y) as i32,r,g,b,a);}}}
            }),
            Kind::Bitmap=>preview::content(|bytes|self.paint_bitmap_template_rect(bytes,body.x as usize,body.y as usize,body.width as usize,body.height as usize,[0,0,0,0],255,false)),
            Kind::Text|Kind::Directory=>{
                preview::content(|bytes|{preview::text_rows(bytes,v.columns(),|i,line|{
                    if i>=v.scroll && i<v.scroll+v.rows() {let row=crate::ui::geometry::Rect{x:body.x,y:body.y+((i-v.scroll)*24*s) as i32,width:body.width,height:(24*s) as u32};
                        self.app_label(row,line,(211,225,237),false,s);}
                });});
                if v.lines>v.rows() {let t=v.track();let th=(t.height as usize*v.rows()/v.lines).max(20*s).min(t.height as usize);
                    let top=t.y as usize+v.scroll*(t.height as usize-th)/v.lines.saturating_sub(v.rows()).max(1);
                    self.fill_rounded_rect_alpha(t.x as usize,t.y as usize,t.width as usize,t.height as usize,4*s,3,16,28,255);
                    self.fill_rounded_rect_alpha(t.x as usize,top,t.width as usize,th,4*s,ar,ag,ab,230);
                }
            },
            _=>{let message=match v.kind {Kind::TooLarge=>b"This file exceeds the 8 MiB preview limit.".as_slice(),Kind::Unavailable=>b"File unavailable or access denied.",_=>b"This document format has no native preview renderer yet."};
                self.app_label(body,message,(178,201,220),false,s);}
        }
        let footer=crate::ui::geometry::Rect{x:(x+20*s) as i32,y:(y+h.saturating_sub(30*s)) as i32,width:w.saturating_sub(40*s) as u32,height:(24*s) as u32};
        self.app_label(footer,b"Read-only preview   /   Esc to close   /   Scroll to explore",(147,180,201),false,s);
        self.fill_rect(x+w.saturating_sub(3*s),y+h/2-16*s,2*s,32*s,ar,ag,ab);
        self.fill_rect(x+s,y+h/2-16*s,2*s,32*s,ar,ag,ab);
        self.fill_rect(x+w/2-16*s,y+h.saturating_sub(3*s),32*s,2*s,ar,ag,ab);
    }
}
