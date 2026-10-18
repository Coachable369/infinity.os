//! Host component proof using production geometry, tab/pill pixels and native Inter.
use infinity_browser_core::{layout::Layout,skin,tab_style};
#[path="browser-tab-artwork-proof.rs"] mod text;
// ------------------------=
// FUNC: main
// DESC: Renders actual chrome component pixels for review without claiming installed-VM proof.
// ------------------=
fn main() {
    let(w,h)=(1280usize,240usize);let primary=0x082238;let accent=0x78b7df;
    let l=Layout::new(1200,480,1).unwrap().with_tab_count(3).with_favorites(false);
    let mut pixels=vec![0u8;w*h*3];
    let base=tab_style::toolbar_color(primary);
    for y in 0..h {for x in 0..w {
        let color=if x>=1200 {[8,17,30]}else if y<48 {[base[0]/2,base[1]/2,base[2]/2]}
            else if y<l.content.y as usize {base}else{[32,33,36]};
        pixels[(y*w+x)*3..(y*w+x)*3+3].copy_from_slice(&color);
    }}
    for i in 0..3 {
        let(t,c)=l.tab(i,3).unwrap();
        for y in 0..t.height {for x in 0..t.width {
            blend(&mut pixels,w,t.x as usize+x as usize,t.y as usize+y as usize,tab_style::themed_pixel(t.width,t.height,x,y,i==0,false,primary,accent));
        }}
        text::label(&mut pixels,w,t.x as usize+36,t.y as usize+11,[b"Welcome" as &[u8],b"Documentation",b"New tab"][i],1);
        cross(&mut pixels,w,c.x as usize+12,c.y as usize+12,4);
    }
    for (r,t) in [(l.file_menu,b"File" as &[u8]),(l.settings_menu,b"Settings"),(l.edit_menu,b"Edit"),(l.view_menu,b"View")] {
        text::label(&mut pixels,w,r.x as usize+8,r.y as usize+9,t,1);
    }
    for r in [l.address,infinity_browser_core::Viewport{x:1208,y:12,width:32,height:32}] {
        for y in 0..r.height {for x in 0..r.width {
            blend(&mut pixels,w,r.x as usize+x as usize,r.y as usize+y as usize,skin::pill_pixel(r.width,r.height,x,y,primary,accent,false));
        }}
    }
    text::label(&mut pixels,w,l.address.x as usize+40,l.address.y as usize+9,b"Search or enter an address",1);
    for r in [l.new_tab,l.minimize,l.maximize,l.close,l.back,l.forward,l.reload,l.downloads,l.menu] {
        let (x,y)=(r.x as usize+r.width as usize/2,r.y as usize+r.height as usize/2);
        let strokes:&[(i32,i32,i32,i32)]=if r==l.close {&[(-5,-5,5,5),(5,-5,-5,5)]}
            else if r==l.minimize {&[(-5,0,5,0)]}
            else if r==l.maximize {&[(-5,-5,5,-5),(5,-5,5,5),(5,5,-5,5),(-5,5,-5,-5)]}
            else if r==l.new_tab {&[(-7,0,7,0),(0,-7,0,7)]}
            else if r==l.back {&[(-7,0,7,0),(-7,0,-1,-6),(-7,0,-1,6)]}
            else if r==l.forward {&[(-7,0,7,0),(7,0,1,-6),(7,0,1,6)]}
            else if r==l.reload {&[(-5,-5,2,-7),(2,-7,7,-3),(7,-3,7,0),(-5,-5,-7,0),(-7,0,-5,5),(-5,5,2,7),(2,7,6,4),(7,-7,7,-1),(7,-1,1,-1)]}
            else if r==l.downloads {&[(0,-7,0,3),(-4,-1,0,3),(0,3,4,-1),(-6,4,-6,7),(-6,7,6,7),(6,7,6,4)]}
            else {&[(-3,-6,3,-6),(3,-6,6,-3),(6,-3,6,3),(6,3,3,6),(3,6,-3,6),(-3,6,-6,3),(-6,3,-6,-3),(-6,-3,-3,-6),(0,-9,0,-6),(0,6,0,9),(-9,0,-6,0),(6,0,9,0)]};
        for &(a,b,c,d) in strokes {line(&mut pixels,w,x as i32+a,y as i32+b,x as i32+c,y as i32+d);}
    }
    use std::io::Write;
    let mut f=std::fs::File::create("build/behavior-tests/browser-chrome-proof.ppm").unwrap();
    write!(f,"P6\n{w} {h}\n255\n").unwrap();f.write_all(&pixels).unwrap();
}
// ------------------------=
// FUNC: blend
// DESC: Composites component alpha over the proof framebuffer.
// ------------------=
fn blend(p:&mut [u8],w:usize,x:usize,y:usize,(r,g,b,a):(u8,u8,u8,u8)) {
    let at=(y*w+x)*3;for(c,v)in[r,g,b].into_iter().enumerate(){p[at+c]=((p[at+c]as u32*(255-a as u32)+v as u32*a as u32)/255)as u8;}
}
// ------------------------=
// FUNC: cross
// DESC: Renders the native close-target stroke on the component review board.
// ------------------=
fn cross(p:&mut [u8],w:usize,x:usize,y:usize,r:usize){for d in 0..=2*r{blend(p,w,x-r+d,y-r+d,(217,227,245,255));blend(p,w,x+r-d,y-r+d,(217,227,245,255));}}
// ------------------------=
// FUNC: line
// DESC: Draws the existing native glyph stroke geometry for the host review board.
// ------------------=
fn line(p:&mut [u8],w:usize,x:i32,y:i32,ex:i32,ey:i32){let steps=(ex-x).abs().max((ey-y).abs()).max(1);for i in 0..=steps {blend(p,w,(x+(ex-x)*i/steps)as usize,(y+(ey-y)*i/steps)as usize,(196,207,232,255));}}
