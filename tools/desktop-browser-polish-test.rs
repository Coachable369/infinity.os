//! Production material/glyph/geometry acceptance and bounded native visual proof.
#[path="../kernel/ui/dock_material.rs"] mod dock;
#[path="../kernel/ui/browser_glyphs.rs"] mod glyphs;
#[path="../kernel/ui/bitmap.rs"] mod bitmap;
use infinity_browser_core::{layout::Layout,skin,tab_style,welcome,Viewport};
use std::io::Write;

// ------------------------=
// FUNC: main
// DESC: Verifies production pixel behavior and emits reviewable browser and dock component images.
// ------------------=
fn main() {
    dock_behavior();glyph_geometry();welcome_geometry();button_pixels();artwork_alpha();
    dock_proof();browser_proof(800,620);browser_proof(1200,720);
    println!("Desktop/browser production material, geometry and pixel acceptance passed");
}

// ------------------------=
// FUNC: material
// DESC: Supplies the actual default-theme material parameters to the production dock renderer.
// ------------------=
fn material()->dock::Material {dock::Material{primary:0x101b3a,accent:0x78b7df,opacity:88,blur:4,scale:1,bgr:true}}

// ------------------------=
// FUNC: dock_behavior
// DESC: Tests backdrop transmission, real high-frequency attenuation, clip safety and active-state interior invariance.
// ------------------=
fn dock_behavior() {
    let (w,h)=(320,120);let bounds=(8,8,304,104);let clip=(0,0,w,h);
    let mut glass=Box::new(dock::Backdrop::new());
    let mut a=vec![0x102030;w*h];let mut b=vec![0x8090a0;w*h];
    glass.paint(&mut a,w,bounds,clip,material());glass.paint(&mut b,w,bounds,clip,material());
    assert_ne!(a[60*w+160],b[60*w+160]);assert_eq!(a[0],0x102030);assert_eq!(a[8*w+8],0x102030);
    let mut sharp=(0..w*h).map(|at|if (at%w+at/w)%2==0 {0xffffff}else{0}).collect::<Vec<_>>();
    let mut blurred=sharp.clone();let mut no_blur=material();no_blur.blur=0;
    glass.paint(&mut sharp,w,bounds,clip,no_blur);glass.paint(&mut blurred,w,bounds,clip,material());
    let contrast=|p:&[u32]|(p[60*w+160]&255).abs_diff(p[60*w+161]&255);
    assert!(contrast(&sharp)>50);assert!(contrast(&blurred)<3);
    let mut opaque=material();opaque.opacity=100;
    glass.paint(&mut a,w,bounds,clip,opaque);glass.paint(&mut b,w,bounds,clip,opaque);
    assert_eq!(a[60*w+160],b[60*w+160]);
    let mut transparent=material();transparent.opacity=0;
    let mut clear=vec![0x203040;w*h];let center=60*w+160;clear[center]=0xffffff;
    let before=clear.clone();glass.paint(&mut clear,w,bounds,clip,transparent);
    for y in 20..100 {for x in 24..296 {assert_eq!(clear[y*w+x],before[y*w+x]);}}
    let original=vec![0x223344;w*h];let mut clipped=original.clone();
    glass.paint(&mut clipped,w,bounds,(100,40,180,80),material());
    for y in 0..h {for x in 0..w {if !(100..180).contains(&x)||!(40..80).contains(&y) {assert_eq!(clipped[y*w+x],original[y*w+x]);}}}
    // Production expands damage before desktop restoration, not after old dock ink
    // is already in the source. This is pixel-identical to a full fresh composition.
    let source=(0..w*h).map(|at|(((at%w)*200/w)as u32)<<16|(((at/w)*150/h)as u32)<<8|(at%41)as u32).collect::<Vec<_>>();
    let mut full=source.clone();glass.paint(&mut full,w,bounds,clip,material());
    for damage in [(100,40,180,80),(0,0,20,20),(290,100,319,119)] {
        let expanded=dock::expand_damage(bounds,damage);let mut partial=vec![0xff00ff;w*h];
        for y in expanded.1..expanded.3 {for x in expanded.0..expanded.2 {partial[y*w+x]=source[y*w+x];}}
        glass.paint(&mut partial,w,bounds,expanded,material());
        for y in bounds.1..bounds.1+bounds.3 {for x in bounds.0..bounds.0+bounds.2 {assert_eq!(partial[y*w+x],full[y*w+x]);}}
    }
    assert_eq!(dock::expand_damage(bounds,(0,0,4,4)),(0,0,4,4));
    for size in [46,92,138,184] {
        for y in size/3..size*2/3 {for x in size/3..size*2/3 {
            assert_eq!(dock::launcher_pixel(size,x as i32,y as i32,0x78b7df,false),dock::launcher_pixel(size,x as i32,y as i32,0x78b7df,true));
        }}
        assert_eq!(dock::launcher_pixel(size,-1,size as i32/2,0x78b7df,false).3,0);
        assert!(dock::launcher_pixel(size,-1,size as i32/2,0x78b7df,true).3>0);
        assert!(dock::launcher_pixel(size,-50,-50,0x78b7df,true).3==0);
    }
    let mut fractional=0;
    for y in 0..14 {for x in 0..14 {let a=dock::rounded_coverage(x,y,304,104,14);if a>0 && a<255 {fractional+=1;}}}
    assert!(fractional>8);
    let mut timings=Vec::new();let template=vec![0x204060;1600*160];let mut frame=template.clone();
    for _ in 0..20 {
        frame.copy_from_slice(&template);let start=std::time::Instant::now();
        glass.paint(&mut frame,1600,(4,4,1592,152),(0,0,1600,160),material());
        timings.push(start.elapsed().as_micros());
    }
    timings.sort_unstable();println!("Frosted 1592x152 backdrop host median: {} us, fixed scratch: {} bytes",timings[timings.len()/2],std::mem::size_of::<dock::Backdrop>());
}

// ------------------------=
// FUNC: glyph_geometry
// DESC: Checks shared optical bounds, centered title controls and fractional alpha at all supported scales.
// ------------------=
fn glyph_geometry() {
    for scale in 1..=4 {
        let l=Layout::new(900*scale,620*scale,scale).unwrap().with_tab_count(1);
        let (tab,close)=l.tab(0,1).unwrap();let center=tab.y+tab.height as i32/2;
        for target in [close,l.new_tab,l.minimize,l.maximize,l.close] {assert_eq!(target.y+target.height as i32/2,center);}
        for (r,id) in [(l.close,9),(close,9),(l.minimize,7),(l.maximize,8),(l.new_tab,11),(l.back,0),(l.reload,2)] {
            let (x,y,size)=glyphs::bounds(r.x,r.y,r.width,r.height,id,scale as usize);
            assert!(x>=r.x && y>=r.y);assert_eq!(x+size as i32/2,r.x+r.width as i32/2);
            assert_eq!(y+size as i32/2,r.y+r.height as i32/2);
            assert_eq!(size,glyphs::logical_size(id)*scale as usize);
            let mut fractional=0;
            for gy in 0..size {for gx in 0..size {let a=glyphs::pixel(id,size,gx,gy,true).3;if a>0 && a<255 {fractional+=1;}}}
            assert!(fractional>0);
        }
    }
}

// ------------------------=
// FUNC: welcome_geometry
// DESC: Asserts readable compact line spacing and actual non-overlapping action/artwork bounds.
// ------------------=
fn welcome_geometry() {
    for s in 1..=4 {for width in [520,720,800,1000,1200] {
        let l=welcome::Layout::new(Viewport{x:0,y:160,width:width*s,height:600*s},s,0);
        let (first,size)=l.copy(1);let (second,_)=l.copy(2);let (detail,_)=l.copy(3);
        assert_eq!(second.y-first.y,(size as u32*5/4*s) as i32);
        assert!(second.y+second.height as i32<=detail.y);
        assert!(detail.y+detail.height as i32<=l.button(0).y);
        let a=l.button(1);let b=l.hero();
        assert!(a.x+a.width as i32<=b.x||a.y+a.height as i32<=b.y);
        assert!(l.footer().y+l.footer().height as i32<=l.area.y+l.height() as i32);
    }}
}

// ------------------------=
// FUNC: button_pixels
// DESC: Ensures the native rounded button gradient has no half-height color discontinuity.
// ------------------=
fn button_pixels() {
    for primary in [false,true] {for s in 1..=4 {
        let (w,h)=(164*s,36*s);let surface=skin::button(primary,skin::Interaction::Normal);
        assert_eq!(skin::surface_pixel(w,h,0,0,10*s,surface).3,0);
        let mut previous=None;
        for y in 2*s..h-2*s {
            let p=skin::surface_pixel(w,h,w/2,y,10*s,surface);assert_eq!(p.3,255);
            if let Some((r,g,b,_))=previous {assert!(p.0.abs_diff(r)<=2 && p.1.abs_diff(g)<=2 && p.2.abs_diff(b)<=2);}
            previous=Some(p);
        }
    }}
}

// ------------------------=
// FUNC: blend
// DESC: Applies production RGBA component pixels to an RGB review surface.
// ------------------=
fn blend(p:&mut[u32],w:usize,x:i32,y:i32,(r,g,b,a):(u8,u8,u8,u8)) {
    if x<0 || y<0 || x as usize>=w || y as usize>=p.len()/w {return;}
    let at=y as usize*w+x as usize;let old=p[at];let mut color=0;
    for (c,value) in [r,g,b].into_iter().enumerate() {let shift=16-c*8;let value=(((old>>shift)&255)*(255-a as u32)+value as u32*a as u32)/255;color|=value<<shift;}
    p[at]=color;
}

// ------------------------=
// FUNC: glyph
// DESC: Renders the exact generated runtime glyph atlas using the same optical bounds as the browser painter.
// ------------------=
fn glyph(p:&mut[u32],w:usize,r:Viewport,id:usize,enabled:bool) {
    let (left,top,size)=glyphs::bounds(r.x,r.y,r.width,r.height,id,1);
    for y in 0..size {for x in 0..size {blend(p,w,left+x as i32,top+y as i32,glyphs::pixel(id,size,x,y,enabled));}}
}

// ------------------------=
// FUNC: label
// DESC: Uses production Inter atlas, advances and kerning at an authored native size.
// ------------------=
fn label(p:&mut[u32],w:usize,mut x:usize,y:usize,text:&[u8],size:usize,color:u32) {
    let atlas=std::fs::read(format!("assets/fonts/InfinityBrowser-Regular-{size}.atlas")).unwrap();
    let metrics=std::fs::read(format!("assets/fonts/InfinityBrowser-Regular-{size}.metrics")).unwrap();
    let kern=std::fs::read(format!("assets/fonts/InfinityBrowser-Regular-{size}.kern")).unwrap();let mut previous=None;
    for &byte in text {let index=(byte-32)as usize;
        if let Some(left)=previous {x=x.saturating_add_signed(kern[left*95+index]as isize-128);}
        for gy in 0..size+6 {for gx in 0..size {let alpha=atlas[gy*size*95+index*size+gx];
            blend(p,w,(x+gx)as i32,(y+gy)as i32,((color>>16)as u8,(color>>8)as u8,color as u8,alpha));}}
        x+=metrics[index]as usize;previous=Some(index);
    }
}

// ------------------------=
// FUNC: bitmap
// DESC: Composes the existing approved artwork using its runtime alpha-aware resampler, never a new mockup asset.
// ------------------=
fn bitmap(p:&mut[u32],w:usize,r:Viewport,data:&[u8]) {
    let image=bitmap::RuntimeBitmap::parse_authored_bgra(data).or_else(||bitmap::RuntimeBitmap::parse(data)).unwrap();
    let placement=image.placement(r.width as usize,r.height as usize,[0;4],false).unwrap();
    for y in 0..placement.destination_height {for x in 0..placement.destination_width {
        let [red,green,blue,alpha]=image.filtered_rgba(placement,x,y);
        blend(p,w,r.x+(x+placement.destination_left)as i32,r.y+(y+placement.destination_top)as i32,(red,green,blue,alpha));
    }}
}

// ------------------------=
// FUNC: artwork_alpha
// DESC: Verifies native artwork preserves transparent padding and fractional edge coverage without changing generic RGB decoding.
// ------------------=
fn artwork_alpha() {
    let image=bitmap::RuntimeBitmap::parse_authored_bgra(welcome::HERO).unwrap();
    assert_eq!(image.rgba(0,0).unwrap()[3],0);
    assert_eq!(bitmap::RuntimeBitmap::parse(welcome::HERO).unwrap().rgba(0,0).unwrap()[3],255);
    let placement=image.placement(288,192,[0;4],false).unwrap();let mut partial=0;let mut transparent=0;
    for y in 0..placement.destination_height {for x in 0..placement.destination_width {
        let alpha=image.filtered_rgba(placement,x,y)[3];
        if alpha>0&&alpha<255 {partial+=1;}if alpha==0 {transparent+=1;}
    }}
    // The user's glass sculpture is intentionally translucent (most solid-looking
    // source pixels carry alpha 252/253), so full opacity is not a contract.
    assert!(partial>0&&transparent>0);
    let mut pixels=vec![0x182b43;288*192];
    bitmap(&mut pixels,288,Viewport{x:0,y:0,width:288,height:192},welcome::HERO);
    assert_eq!(pixels[0],0x182b43);assert_eq!(pixels[287],0x182b43);
}

// ------------------------=
// FUNC: browser_proof
// DESC: Renders compact and wide native geometry with production tab, glyph, gradient and artwork pixels for review.
// ------------------=
fn browser_proof(w:usize,h:usize) {
    let primary=0x101b3a;let accent=0x78b7df;
    let l=Layout::new(w as u32,h as u32,1).unwrap().with_tab_count(1).with_favorites(false);
    let color=tab_style::toolbar_color(primary);let toolbar=(color[0]as u32)<<16|(color[1]as u32)<<8|color[2]as u32;
    let mut p=vec![toolbar;w*h];
    for y in 0..48 {p[y*w..(y+1)*w].fill(0x101722);}
    let (tab,close)=l.tab(0,1).unwrap();
    for y in 0..tab.height {for x in 0..tab.width {blend(&mut p,w,tab.x+x as i32,tab.y+y as i32,tab_style::themed_pixel(tab.width,tab.height,x,y,true,false,primary,accent));}}
    glyph(&mut p,w,Viewport{x:tab.x+20,y:tab.y+10,width:16,height:16},10,true);
    label(&mut p,w,tab.x as usize+44,tab.y as usize+10,b"Welcome",14,0xdee4f6);
    for (r,id) in [(close,9),(l.new_tab,11),(l.minimize,7),(l.maximize,8),(l.close,9),(l.back,0),(l.forward,1),(l.reload,2),(l.downloads,4),(l.menu,13)] {glyph(&mut p,w,r,id,id>1);}
    for (r,text) in [(l.file_menu,b"File" as &[u8]),(l.settings_menu,b"Settings"),(l.edit_menu,b"Edit"),(l.view_menu,b"View")] {label(&mut p,w,r.x as usize+8,r.y as usize+9,text,14,0xdee4f6);}
    for y in 0..l.address.height {for x in 0..l.address.width {blend(&mut p,w,l.address.x+x as i32,l.address.y+y as i32,skin::pill_pixel(l.address.width,l.address.height,x,y,primary,accent,false));}}
    glyph(&mut p,w,Viewport{x:l.address.x+8,y:l.address.y+4,width:24,height:24},14,true);
    label(&mut p,w,l.address.x as usize+40,l.address.y as usize+9,b"Search or enter an address",14,0xdee4f6);
    let v=l.content;for y in 0..v.height {let shade=y.min(520)*10/520;let c=(18-shade/2)<<16|(31-shade/2)<<8|(51-shade);
        p[(v.y as usize+y as usize)*w..(v.y as usize+y as usize+1)*w].fill(c);}
    let page=welcome::Layout::new(v,1,0);
    for (i,text) in [b"InfinityOS" as &[u8],b"A more human",b"browsing experience.",b"Explore. Create. Stay in flow."].into_iter().enumerate() {
        let(r,size)=page.copy(i);label(&mut p,w,r.x as usize,r.y as usize,text,size,if i==0||i==3 {0xa6bade}else{0xf1f5ff});}
    bitmap(&mut p,w,page.hero(),welcome::HERO);
    for i in 0..2 {let r=page.button(i);let surface=skin::button(i==0,skin::Interaction::Normal);
        for y in 0..r.height {for x in 0..r.width {blend(&mut p,w,r.x+x as i32,r.y+y as i32,skin::surface_pixel(r.width,r.height,x,y,10,surface));}}
        label(&mut p,w,r.x as usize+if i==0 {34}else{26},r.y as usize+10,if i==0 {b"Start browsing"}else{b"Browser settings"},14,surface.text);
    }
    for (i,(title,detail)) in [(b"Continue browsing" as &[u8],b"Find open tabs in View." as &[u8]),(b"Your bookmarks",b"Save favorites with the star."),(b"Explore InfinityOS",b"Make it yours in Settings.")].into_iter().enumerate() {
        let r=page.card(i);if r.y+r.height as i32>l.status.y {continue;}
        let surface=skin::Surface{top:0x1c2a3e,bottom:0x1c2a3e,border:0x394d6a,text:0xf1f5ff};
        for y in 0..r.height {for x in 0..r.width {blend(&mut p,w,r.x+x as i32,r.y+y as i32,skin::surface_pixel(r.width,r.height,x,y,12,surface));}}
        glyph(&mut p,w,Viewport{x:r.x+20,y:r.y+18,width:24,height:24},[15,16,10][i],true);
        label(&mut p,w,r.x as usize+20,r.y as usize+66,title,14,0xf1f5ff);
        label(&mut p,w,r.x as usize+20,r.y as usize+90,detail,14,0xa6bade);
    }
    for y in l.status.y as usize..h {p[y*w..(y+1)*w].fill(0x061522);}
    label(&mut p,w,24,l.status.y as usize+5,b"Welcome - ready to browse",14,0xa6bade);
    write_ppm(&format!("build/behavior-tests/browser-polish-{w}.ppm"),w,h,&p);
}

// ------------------------=
// FUNC: dock_proof
// DESC: Shows real blurred wallpaper transmission and unchanged launcher interior across closed/open states.
// ------------------=
fn dock_proof() {
    let(w,h)=(1000,380);let mut p=vec![0x10243a;w*h];
    let wallpaper=std::fs::read("assets/desktop/infinity-default-dark-wallpaper-v2.bmp").unwrap();
    bitmap(&mut p,w,Viewport{x:0,y:0,width:w as u32,height:h as u32},&wallpaper);
    let mut frost=Box::new(dock::Backdrop::new());
    for (index,y) in [100,250].into_iter().enumerate() {
        frost.paint(&mut p,w,(30,y,940,76),(0,0,w,h),material());
        for dy in -5..51 {for dx in -5..51 {blend(&mut p,w,78+dx,y as i32+15+dy,dock::launcher_pixel(46,dx,dy,0x78b7df,index==1));}}
        bitmap(&mut p,w,Viewport{x:82,y:y as i32+22,width:38,height:28},include_bytes!("../assets/desktop/infinity-topbar-icon-v2.bmp"));
        let atlas=std::fs::read("assets/icons/runtime/crystal-blue-glass-base.bmp").unwrap();
        for (i,id) in [2,26,28,23,27,10].into_iter().enumerate() {
            native_atlas(&mut p,w,200+i*122,y+15,46,&atlas,id);
        }
        label(&mut p,w,32,y-30,if index==0 {b"Production dock material / existing icon atlas / closed"}else{b"Production dock material / existing icon atlas / open rim glow"},14,0xe7f2fa);
    }
    write_ppm("build/behavior-tests/dock-polish.ppm",w,h,&p);
}

// ------------------------=
// FUNC: native_atlas
// DESC: Samples the actual 5x9 desktop icon atlas with the same bounded premultiplied filter as the native painter.
// ------------------=
fn native_atlas(p:&mut[u32],w:usize,left:usize,top:usize,size:usize,data:&[u8],role:usize) {
    assert_eq!(u16::from_le_bytes(data[28..30].try_into().unwrap()),32);
    let offset=u32::from_le_bytes(data[10..14].try_into().unwrap())as usize;
    let sw=u32::from_le_bytes(data[18..22].try_into().unwrap())as usize;
    let signed=i32::from_le_bytes(data[22..26].try_into().unwrap());let sh=signed.unsigned_abs()as usize;
    let cw=sw/5;let ch=sh/9;let nx=(cw/size).clamp(1,4);let ny=(ch/size).clamp(1,4);
    for y in 0..size {for x in 0..size {
        let mut channels=[0u32;3];let mut coverage=0;
        for sy in 0..ny {for sx in 0..nx {
            let ay=role/5*ch+((y*ny*2+sy*2+1)*ch/(size*ny*2)).min(ch-1);
            let ax=role%5*cw+((x*nx*2+sx*2+1)*cw/(size*nx*2)).min(cw-1);
            let at=offset+((if signed<0 {ay}else{sh-1-ay})*sw+ax)*4;
            let a=data[at+3]as u32;coverage+=a;
            for c in 0..3 {channels[c]+=data[at+c]as u32*a;}
        }}
        if coverage>0 {blend(p,w,(left+x)as i32,(top+y)as i32,
            ((channels[2]/coverage)as u8,(channels[1]/coverage)as u8,(channels[0]/coverage)as u8,(coverage/(nx*ny)as u32)as u8));}
    }}
}

// ------------------------=
// FUNC: write_ppm
// DESC: Saves actual native component pixels under the build-kit-owned proof directory.
// ------------------=
fn write_ppm(path:&str,w:usize,h:usize,p:&[u32]) {
    let mut file=std::fs::File::create(path).unwrap();write!(file,"P6\n{w} {h}\n255\n").unwrap();
    for &pixel in p {file.write_all(&[(pixel>>16)as u8,(pixel>>8)as u8,pixel as u8]).unwrap();}
}
