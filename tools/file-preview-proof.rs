//! Production preview painter with a host framebuffer adapter; not installed-VM proof.
#[path="../kernel/core/bootstrap/file_preview_view.rs"] mod painter;
#[path="browser-tab-artwork-proof.rs"] mod font;
use crate::geometry::Rect;
struct DisplayDevice {pixels:Vec<u8>,width:usize,height:usize}
impl DisplayDevice {
    // ------------------------=
    // FUNC: active_primary_rgb
    // DESC: Uses the documented default navy for the host preview fixture.
    // ------------------=
    fn active_primary_rgb(&self)->u32{0x082238}
    // ------------------------=
    // FUNC: active_accent_rgb
    // DESC: Uses the documented blue accent for the host preview fixture.
    // ------------------=
    fn active_accent_rgb(&self)->u32{0x78b7df}
    // ------------------------=
    // FUNC: blend_color
    // DESC: Composites a bounded production-painter pixel onto the review framebuffer.
    // ------------------=
    fn blend_color(&mut self,x:i32,y:i32,r:u8,g:u8,b:u8,a:u8){if x<0||y<0||x as usize>=self.width||y as usize>=self.height{return;}let at=(y as usize*self.width+x as usize)*3;
        for(i,v)in[r,g,b].into_iter().enumerate(){self.pixels[at+i]=((self.pixels[at+i] as u32*(255-a as u32)+v as u32*a as u32)/255)as u8;}}
    // ------------------------=
    // FUNC: fill_rect
    // DESC: Writes bounded solid rectangles requested by the production painter.
    // ------------------=
    fn fill_rect(&mut self,x:usize,y:usize,w:usize,h:usize,r:u8,g:u8,b:u8){for py in y..(y+h).min(self.height){for px in x..(x+w).min(self.width){self.blend_color(px as i32,py as i32,r,g,b,255);}}}
    // ------------------------=
    // FUNC: fill_rounded_rect_alpha
    // DESC: Matches the native rounded surface coverage for the host review adapter.
    // ------------------=
    fn fill_rounded_rect_alpha(&mut self,x:usize,y:usize,w:usize,h:usize,radius:usize,r:u8,g:u8,b:u8,a:u8){for py in 0..h{for px in 0..w{
        let dx=if px<radius{radius-px}else{px.saturating_sub(w.saturating_sub(radius+1))};let dy=if py<radius{radius-py}else{py.saturating_sub(h.saturating_sub(radius+1))};
        if dx==0||dy==0||dx*dx+dy*dy<=radius*radius{self.blend_color((x+px)as i32,(y+py)as i32,r,g,b,a);}}}}
    // ------------------------=
    // FUNC: outline_rounded_rect
    // DESC: Draws the review border using the native surface coverage with an inset.
    // ------------------=
    fn outline_rounded_rect(&mut self,x:usize,y:usize,w:usize,h:usize,radius:usize,r:u8,g:u8,b:u8){self.fill_rounded_rect_alpha(x,y,w,h,radius,r,g,b,255);self.fill_rounded_rect_alpha(x+1,y+1,w-2,h-2,radius.saturating_sub(1),4,17,28,255);}
    // ------------------------=
    // FUNC: line
    // DESC: Rasterizes the close glyph from the production painter's stroke coordinates.
    // ------------------=
    fn line(&mut self,x:i32,y:i32,xx:i32,yy:i32,r:u8,g:u8,b:u8){let n=(xx-x).abs().max((yy-y).abs()).max(1);for i in 0..=n{self.blend_color(x+(xx-x)*i/n,y+(yy-y)*i/n,r,g,b,255);}}
    // ------------------------=
    // FUNC: app_text
    // DESC: Draws the existing authored native Inter atlas without substituting a host font.
    // ------------------=
    fn app_text(&mut self,x:usize,y:usize,text:&[u8],_: (u8,u8,u8),_:bool,s:usize){let text:Vec<u8>=text.iter().map(|b|if (32..=126).contains(b){*b}else{b' '}).collect();font::label(&mut self.pixels,self.width,x,y,&text,s);}
    // ------------------------=
    // FUNC: app_label
    // DESC: Bounds host labels to their production viewport before native-atlas rendering.
    // ------------------=
    fn app_label(&mut self,r:Rect,text:&[u8],color:(u8,u8,u8),bold:bool,s:usize){let n=text.len().min(r.width as usize/(12*s));self.app_text(r.x as usize,r.y as usize,&text[..n],color,bold,s);}
    // ------------------------=
    // FUNC: paint_bitmap_template_rect
    // DESC: Uses production bitmap validation, placement and pixels in the review framebuffer.
    // ------------------=
    fn paint_bitmap_template_rect(&mut self,bytes:&[u8],x:usize,y:usize,w:usize,h:usize,crop:[u8;4],alpha:u8,fill:bool){let Some(b)=crate::bitmap::RuntimeBitmap::parse(bytes)else{return;};let Some(p)=b.placement(w,h,crop,fill)else{return;};
        for yy in 0..p.destination_height{for xx in 0..p.destination_width{if let Some([r,g,bb,a])=b.rgba(p.source_left+xx*p.source_width/p.destination_width,p.source_top+yy*p.source_height/p.destination_height){self.blend_color((x+p.destination_left+xx)as i32,(y+p.destination_top+yy)as i32,r,g,bb,(a as u16*alpha as u16/255)as u8);}}}}
}
// ------------------------=
// FUNC: render
// DESC: Captures the production text-preview composition for review without claiming a VM screenshot.
// ------------------=
pub fn render(){use crate::file_preview as p;let mut state=p::Preview::new();state.open=true;state.place(1024,768,1);state.kind=p::Kind::Text;
    let path=b"/home/default/documents/readme.md";state.path[..path.len()].copy_from_slice(path);state.path_len=path.len();
    let text=b"# InfinityOS project notes\n\nPreview documents without opening an editor.\nYour original file stays unchanged.\n\nFeatures\n- Right-click a file and choose Preview\n- Scroll long documents with the wheel or rail\n- Move the title bar and resize the side edges\n- Press Escape to return to your workspace\n\nCommand line\npreview /home/default/documents/readme.md\n";
    state.length=p::load(|b|{b[..text.len()].copy_from_slice(text);Some(text.len())}).unwrap();state.lines=p::text_rows(text,state.columns(),|_,_|{});p::publish(state);
    let mut d=DisplayDevice{pixels:vec![12;1024*768*3],width:1024,height:768};d.file_preview_overlay();
    use std::io::Write;let mut out=std::fs::File::create("builds/file-preview-proof.ppm").unwrap();write!(out,"P6\n1024 768\n255\n").unwrap();out.write_all(&d.pixels).unwrap();p::close();
    state=p::Preview::new();state.open=true;state.menu=true;state.rect=Rect{x:40,y:40,width:300,height:72};
    let path=b"/home/default/pictures/sky.png";state.path[..path.len()].copy_from_slice(path);state.path_len=path.len();p::publish(state);
    d.pixels.fill(12);d.file_preview_overlay();let mut out=std::fs::File::create("builds/wallpaper-menu-proof.ppm").unwrap();write!(out,"P6\n1024 768\n255\n").unwrap();out.write_all(&d.pixels).unwrap();p::close();
}
