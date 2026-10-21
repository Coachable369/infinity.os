//! Eclipse Precision is the OS-owned image-backed surface contract, not an application style.
pub type Rgba = (u8, u8, u8, u8);
pub type Rgb = (u8, u8, u8);
const SIZE: usize = 256;
const CELLS: usize = 14;
const ART: &[u8; 2 * CELLS * SIZE * SIZE * 4] = include_bytes!("../../assets/ui-design-kit/default/eclipse-precision-v1.rgba");

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Surface { Frame, WindowFill, Panel, Primary, Secondary, Field, Current, Inactive }

pub const STYLE_COUNT: usize = 6;
pub const STYLE_BYTES: usize = 12 + STYLE_COUNT * 12;
#[used]
static STYLE_PAYLOAD: [u8; STYLE_BYTES] = *include_bytes!("../../assets/ui-design-kit/default/uikit-styles.bin");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SurfaceStyle {
    pub material: u8,
    pub corners: u8,
    pub radius: u8,
    pub fill: Option<u32>,
    pub border: Option<u32>,
}

impl SurfaceStyle {
    pub const AUTHORED: Self = Self { material: 0, corners: 0, radius: 0, fill: None, border: None };
}

// ------------------------=
// FUNC: decode_styles
// DESC: Rejects malformed surface records atomically before exposing geometry, material or optional tint overrides.
// ------------------=
pub const fn decode_styles(bytes: &[u8]) -> Option<[SurfaceStyle; STYLE_COUNT]> {
    if bytes.len() != STYLE_BYTES { return None; }
    let magic = b"IUISTY01";
    let mut i = 0;
    while i < 8 { if bytes[i] != magic[i] { return None; } i += 1; }
    if bytes[8] != 1 || bytes[9] != 0 || bytes[10] != 6 || bytes[11] != 0 { return None; }
    let mut result = [SurfaceStyle::AUTHORED; STYLE_COUNT];
    i = 0;
    while i < STYLE_COUNT {
        let at = 12 + i * 12;
        if bytes[at] > 2 || bytes[at+1] > 2 || bytes[at+2] > 40 || bytes[at+3] & !3 != 0
            || (i < 2 && bytes[at+1] == 2) { return None; }
        let fill = u32::from_le_bytes([bytes[at+4], bytes[at+5], bytes[at+6], bytes[at+7]]);
        let border = u32::from_le_bytes([bytes[at+8], bytes[at+9], bytes[at+10], bytes[at+11]]);
        result[i] = SurfaceStyle { material: bytes[at], corners: bytes[at+1], radius: bytes[at+2],
            fill: if bytes[at+3] & 1 != 0 { Some(fill) } else { None },
            border: if bytes[at+3] & 2 != 0 { Some(border) } else { None } };
        i += 1;
    }
    Some(result)
}

// ------------------------=
// FUNC: surface_style
// DESC: Resolves the six shared component defaults, with the compositor frame controlling the window substrate silhouette.
// ------------------=
pub fn surface_style(kind: Surface) -> SurfaceStyle {
    #[cfg(studio_native_preview)]
    let bytes=crate::studio_native_render::asset_override(b"uikit-styles.bin").unwrap_or(&STYLE_PAYLOAD);
    #[cfg(not(studio_native_preview))]
    let bytes=&STYLE_PAYLOAD;
    let styles = match decode_styles(bytes) { Some(styles) => styles, None => [SurfaceStyle::AUTHORED; STYLE_COUNT] };
    let mut style = match kind { Surface::Frame => styles[0], Surface::WindowFill => styles[1],
        Surface::Panel => styles[2], Surface::Primary => styles[3], Surface::Secondary => styles[4],
        Surface::Field => styles[5], _ => SurfaceStyle::AUTHORED };
    if matches!(kind, Surface::WindowFill) && styles[0].corners != 0 {
        style.corners = styles[0].corners;
        style.radius = styles[0].radius;
    }
    style
}

// ------------------------=
// FUNC: template_style
// DESC: Inherits shared material and geometry while retaining the element's explicitly authored fill and border colors.
// ------------------=
pub fn template_style(kind: Surface, material: u8, corners: u8, radius: u8) -> SurfaceStyle {
    let mut style = surface_style(kind);
    style.fill = None;
    style.border = None;
    if material != 0 { style.material = material; }
    if corners != 0 { style.corners = corners; style.radius = if radius > 40 { 40 } else { radius }; }
    style
}

#[derive(Clone, Copy)]
pub struct Material {
    pub fill: Rgb,
    pub edge: Rgb,
    pub opacity: u8,
    pub edge_opacity: u8,
    pub focus: u8,
    pub pressed: bool,
    pub scale: usize,
}

// ------------------------=
// FUNC: rim_width
// DESC: Shares the exact authored corner/glow extent with the clipped painter so transparent fills never discard focus lighting.
// ------------------=
pub fn rim_width(kind: Surface, scale: usize) -> usize {
    (match kind { Surface::Frame => 18, Surface::Panel => 12,
        Surface::Primary | Surface::Secondary | Surface::Field => 8, _ => 0 }) * scale.max(1)
}

// ------------------------=
// FUNC: axis
// DESC: Preserves native corner pixels at each logical scale and stretches only the interior nine-slice spans.
// ------------------=
fn axis(at: usize, length: usize, cut: usize, scale: usize) -> usize {
    let cap=(cut*scale).min(length/2).max(1);
    if at<cap { return at*cut*256/cap; }
    if at>=length.saturating_sub(cap) {
        return (SIZE-cut)*256+(at-(length-cap))*cut*256/cap;
    }
    cut*256+(at-cap)*(SIZE-cut*2)*256/length.saturating_sub(cap*2).max(1)
}

// ------------------------=
// FUNC: texture
// DESC: Bilinearly filters premultiplied authored coverage without introducing dark fringes around transparent light.
// ------------------=
fn texture(index: usize, fx: usize, fy: usize) -> Rgba {
    #[cfg(studio_native_preview)]
    let art=crate::studio_native_render::precision_art().unwrap_or(ART);
    #[cfg(not(studio_native_preview))]
    let art=ART;
    let (sx,sy)=(fx/256,fy/256); let (wx,wy)=(fx%256,fy%256);
    let address=|x:usize,y:usize| (index*SIZE*SIZE+y.min(SIZE-1)*SIZE+x.min(SIZE-1))*4;
    let offsets=[address(sx,sy),address(sx+1,sy),address(sx,sy+1),address(sx+1,sy+1)];
    if wx==0 && wy==0 {
        let at=offsets[0];return (art[at],art[at+1],art[at+2],art[at+3]);
    }
    if offsets.iter().all(|at|art[at+3]==255) {
        let weights=[(256-wx)*(256-wy),wx*(256-wy),(256-wx)*wy,wx*wy];
        let channel=|c:usize| ((0..4).map(|i|usize::from(art[offsets[i]+c])*weights[i]).sum::<usize>()>>16) as u8;
        return (channel(0),channel(1),channel(2),255);
    }
    let mut channels=[0u64;4];
    for (dy,vy) in [(0,256-wy),(1,wy)] {for (dx,vx) in [(0,256-wx),(1,wx)] {
        let at=(index*SIZE*SIZE+(sy+dy).min(SIZE-1)*SIZE+(sx+dx).min(SIZE-1))*4;
        let weight=(vx*vy) as u64; let alpha=art[at+3] as u64;
        for c in 0..3 {channels[c]+=art[at+c] as u64*alpha*weight;}
        channels[3]+=alpha*weight;
    }}
    let alpha=channels[3];
    if alpha==0 {return (0,0,0,0);}
    ((channels[0]/alpha) as u8,(channels[1]/alpha) as u8,(channels[2]/alpha) as u8,
        ((alpha+32768)/65536) as u8)
}

// ------------------------=
// FUNC: transition
// DESC: Eases between authored state pixels in premultiplied space rather than lifting the whole fill uniformly.
// ------------------=
fn transition(a:Rgba,b:Rgba,t:u8)->Rgba {
    if t==0 {return a;} if t==255 {return b;}
    let alpha=u32::from(a.3)*(255-u32::from(t))+u32::from(b.3)*u32::from(t);
    if alpha==0 {return (0,0,0,0);}
    let channel=|x:u8,y:u8| ((u32::from(x)*u32::from(a.3)*(255-u32::from(t))
        +u32::from(y)*u32::from(b.3)*u32::from(t))/alpha) as u8;
    (channel(a.0,b.0),channel(a.1,b.1),channel(a.2,b.2),((alpha+127)/255) as u8)
}

// ------------------------=
// FUNC: themed
// DESC: Applies only the user's deviation from approved tokens, preserving authored contrast and pressed colors in the default theme.
// ------------------=
fn themed(pixel:Rgba,base:Rgb,target:Rgb,opacity:u8)->Rgba {
    if pixel.3==0 || opacity==0 {return (0,0,0,0);}
    if base==target {return (pixel.0,pixel.1,pixel.2,(u16::from(pixel.3)*u16::from(opacity)/255) as u8);}
    let shift=|value:u8,reference:u8,tint:u8| (i16::from(value)+i16::from(tint)-i16::from(reference)).clamp(0,255) as u8;
    (shift(pixel.0,base.0,target.0),shift(pixel.1,base.1,target.1),shift(pixel.2,base.2,target.2),
        (u16::from(pixel.3)*u16::from(opacity)/255) as u8)
}

// ------------------------=
// FUNC: over
// DESC: Combines independent edge light and background into one sample without fading the outline with surface opacity.
// ------------------=
fn over(background:Rgba,edge:Rgba)->Rgba {
    if edge.3==0 {return background;}
    if edge.3==255 || background.3==0 {return edge;}
    let retained=u32::from(background.3)*(255-u32::from(edge.3));
    let alpha=u32::from(edge.3)*255+retained;
    if alpha==0 {return (0,0,0,0);}
    let channel=|b:u8,e:u8| ((u32::from(e)*u32::from(edge.3)*255+u32::from(b)*retained)/alpha) as u8;
    (channel(background.0,edge.0),channel(background.1,edge.1),channel(background.2,edge.2),((alpha+127)/255) as u8)
}

// ------------------------=
// FUNC: pixel
// DESC: Samples reference states and bounded light while keeping hit geometry, theme and background opacity independent.
// ------------------=
pub fn pixel(kind:Surface,width:usize,height:usize,x:usize,y:usize,m:Material)->Rgba {
    pixel_styled(kind,width,height,x,y,m,surface_style(kind))
}

// ------------------------=
// FUNC: pixel_authored
// DESC: Retains the approved no-override atlas sampler exactly for backward-compatible pixels.
// ------------------=
pub fn pixel_authored(kind:Surface,width:usize,height:usize,x:usize,y:usize,m:Material)->Rgba {
    pixel_authored_material(kind,width,height,x,y,m,false)
}

// ------------------------=
// FUNC: pixel_authored_material
// DESC: Retains authored image geometry and alpha while optionally flattening marker reflection light for the matte material.
// ------------------=
fn pixel_authored_material(kind:Surface,width:usize,height:usize,x:usize,y:usize,m:Material,matte:bool)->Rgba {
    if width==0 || height==0 || x>=width || y>=height {return (0,0,0,0);}
    let scale=m.scale.max(1);
    let circular=matches!(kind,Surface::Current|Surface::Inactive);
    let rim=rim_width(kind,scale);
    let distance=x.min(y).min(width-1-x).min(height-1-y);
    if kind==Surface::Frame && distance>=rim {return (0,0,0,0);}
    let (normal,focused)=match kind {
        Surface::Frame=>(0,1),Surface::Panel|Surface::WindowFill=>(2,3),
        Surface::Primary=>if m.pressed {(6,6)}else{(4,5)},
        Surface::Secondary=>if m.pressed {(13,13)}else{(11,12)},
        Surface::Field=>(7,8),Surface::Current=>(9,9),Surface::Inactive=>(10,10),
    };
    let (fx,fy)=if circular {(x*255*256/width,y*255*256/height)}
        else {let cut=if kind==Surface::WindowFill {12}else{rim/scale};
            (axis(x,width,cut,scale),axis(y,height,cut,scale))};
    let state=|layer:usize| {
        if normal==focused || m.focus==0 {texture(normal+layer*CELLS,fx,fy)}
        else if m.focus==255 {texture(focused+layer*CELLS,fx,fy)}
        else {transition(texture(normal+layer*CELLS,fx,fy),texture(focused+layer*CELLS,fx,fy),m.focus)}
    };
    let base=if matches!(kind,Surface::Primary|Surface::Current) {(62,111,255)}else{(16,27,58)};
    let background=if m.opacity!=0 && kind!=Surface::Frame && (kind==Surface::WindowFill || circular || distance>=2*scale) {
        // WindowFill is an OS rectangular substrate, not a rounded panel mask.
        // Sample interior fill at its boundary; the compositor owns every edge.
        if kind==Surface::WindowFill {
            themed(texture(2,fx.clamp(12*256,243*256),fy.clamp(12*256,243*256)),base,m.fill,m.opacity)
        } else {
        let mut sample=state(0);
        if matte {
            let flatten=|value:u8,base:u8| (i16::from(base)+(i16::from(value)-i16::from(base))/5).clamp(0,255) as u8;
            sample=(flatten(sample.0,base.0),flatten(sample.1,base.1),flatten(sample.2,base.2),sample.3);
        }
        themed(sample,base,m.fill,m.opacity)
        }
    }else{(0,0,0,0)};
    let edge=if m.edge_opacity!=0 && !circular && distance<rim {
        themed(state(1),(134,199,255),m.edge,m.edge_opacity)
    }else{(0,0,0,0)};
    over(background,edge)
}

// ------------------------=
// FUNC: styled_axis
// DESC: Resizes only the authored corner extent while preserving native antialiased image coverage.
// ------------------=
fn styled_axis(at: usize, length: usize, cut: usize, cap: usize) -> usize {
    let cap = cap.min(length/2).max(1);
    if at < cap { return at * cut * 256 / cap; }
    if at >= length-cap { return (SIZE-cut)*256+(at-(length-cap))*cut*256/cap; }
    cut*256+(at-cap)*(SIZE-cut*2)*256/length.saturating_sub(cap*2).max(1)
}

// ------------------------=
// FUNC: integer_sqrt
// DESC: Resolves rounded image-edge sampling depth without floating point or architecture-specific instructions.
// ------------------=
fn integer_sqrt(value: u64) -> u64 {
    if value < 2 { return value; }
    let mut low=1u64;let mut high=value.min(u32::MAX as u64);
    while low<high {
        let middle=low+(high-low+1)/2;
        if middle<=value/middle {low=middle;}else{high=middle-1;}
    }
    low
}

// ------------------------=
// FUNC: rounded_shape
// DESC: Returns four-by-four subpixel silhouette coverage and fixed-point inward edge depth for image-backed rounded styles.
// ------------------=
fn rounded_shape(x:usize,y:usize,w:usize,h:usize,radius:usize)->(u8,usize) {
    let r=radius.min(w/2).min(h/2);
    let ix=(x*2+1).min((w-x)*2-1)*128;
    let iy=(y*2+1).min((h-y)*2-1)*128;
    if r==0 || ix>=r*256 || iy>=r*256 {return (255,ix.min(iy));}
    let rx=r*256-ix;let ry=r*256-iy;
    let depth=(r*256).saturating_sub(integer_sqrt((rx*rx+ry*ry) as u64) as usize);
    let cx=if x<r {r*8}else{(w-r)*8} as i64;
    let cy=if y<r {r*8}else{(h-r)*8} as i64;
    let mut count=0usize;
    for sy in [1,3,5,7] {for sx in [1,3,5,7] {
        let dx=(x*8+sx) as i64-cx;let dy=(y*8+sy) as i64-cy;
        if dx*dx+dy*dy<=(r*8).pow(2) as i64 {count+=1;}
    }}
    (((count*255+8)/16) as u8,depth)
}

// ------------------------=
// FUNC: pixel_styled
// DESC: Applies image-backed surface variants and independently tinted borders without replacing the approved atlas with drawn chrome.
// ------------------=
pub fn pixel_styled(mut kind:Surface,width:usize,height:usize,x:usize,y:usize,mut m:Material,mut style:SurfaceStyle)->Rgba {
    if width==0 || height==0 || x>=width || y>=height { return (0,0,0,0); }
    if matches!(kind,Surface::Frame|Surface::WindowFill) && style.corners==1 { style.corners=0; }
    if let Some(color)=style.fill {
        m.fill=((color>>16) as u8,(color>>8) as u8,color as u8);
        m.opacity=(u16::from(m.opacity)*((color>>24) as u16)/255) as u8;
    }
    if let Some(color)=style.border {
        m.edge=((color>>16) as u8,(color>>8) as u8,color as u8);
        m.edge_opacity=(u16::from(m.edge_opacity)*((color>>24) as u16)/255) as u8;
    }
    if style.material!=2 && style.corners==0 { return pixel_authored(kind,width,height,x,y,m); }
    if matches!(kind,Surface::Current|Surface::Inactive) {
        if style.corners==0 {return pixel_authored_material(kind,width,height,x,y,m,true);}
        // Explicit silhouettes reuse the approved filled/quiet surface edges;
        // authored circular markers retain their original coverage above.
        kind=if kind==Surface::Current {Surface::Primary}else{Surface::Secondary};
    }
    let scale=m.scale.max(1);
    let distance=x.min(y).min(width-1-x).min(height-1-y);
    let rounded=style.corners==2 && style.radius!=0;
    let square=style.corners==1 || (style.corners==2 && style.radius==0);
    let (normal,focused)=match kind {
        Surface::Frame if rounded=>(2,3),Surface::Frame=>(0,1),
        Surface::WindowFill|Surface::Panel=>(2,3),
        Surface::Primary=>if m.pressed {(6,6)}else{(4,5)},
        Surface::Secondary=>if m.pressed {(13,13)}else{(11,12)},
        Surface::Field=>(7,8),_=>(2,3),
    };
    let cut=match kind {Surface::Frame if !rounded=>18,Surface::Frame|Surface::WindowFill|Surface::Panel=>12,_=>8};
    let cap=if rounded {usize::from(style.radius)*scale}else{cut*scale};
    let (fx,fy)=if rounded {(styled_axis(x,width,cut,cap),styled_axis(y,height,cut,cap))}
        else{(axis(x,width,cut,scale),axis(y,height,cut,scale))};
    let state=|layer:usize,sx:usize,sy:usize| {
        if normal==focused || m.focus==0 {texture(normal+layer*CELLS,sx,sy)}
        else if m.focus==255 {texture(focused+layer*CELLS,sx,sy)}
        else{transition(texture(normal+layer*CELLS,sx,sy),texture(focused+layer*CELLS,sx,sy),m.focus)}
    };
    let base=if kind==Surface::Primary {(62,111,255)}else{(16,27,58)};
    let (coverage,depth)=if rounded {rounded_shape(x,y,width,height,cap)}else{(255,0)};
    if coverage==0 {return (0,0,0,0);}
    let mut fill=if square || rounded || kind==Surface::WindowFill {
        state(0,fx.clamp(cut*256,(255-cut)*256),fy.clamp(cut*256,(255-cut)*256))
    }else{state(0,fx,fy)};
    if style.material==2 {
        let flatten=|value:u8,base:u8| (i16::from(base)+(i16::from(value)-i16::from(base))/5).clamp(0,255) as u8;
        fill=(flatten(fill.0,base.0),flatten(fill.1,base.1),flatten(fill.2,base.2),fill.3);
    }
    let background=if kind==Surface::Frame { (0,0,0,0) } else { themed(fill,base,m.fill,m.opacity) };
    let edge=if kind==Surface::WindowFill || m.edge_opacity==0 {(0,0,0,0)}else {
        let raw=if rounded {
            if depth>=cut*scale*256 {(0,0,0,0)}else {
                let sample=(depth/scale).min(cut*256);
                if x.min(width-1-x)<y.min(height-1-y) {
                    state(1,if x<width/2 {sample}else{255*256-sample},128*256)
                }else{
                    state(1,128*256,if y<height/2 {sample}else{255*256-sample})
                }
            }
        }else if square {
            let horizontal=if y.min(height-1-y)<cut*scale {state(1,128*256,fy)}else{(0,0,0,0)};
            let vertical=if x.min(width-1-x)<cut*scale {state(1,fx,128*256)}else{(0,0,0,0)};
            over(horizontal,vertical)
        }else if distance<cap.max(cut*scale) {state(1,fx,fy)}else{(0,0,0,0)};
        themed(raw,(134,199,255),m.edge,m.edge_opacity)
    };
    let result=over(background,edge);
    (result.0,result.1,result.2,(u16::from(result.3)*u16::from(coverage)/255)as u8)
}
