//! Eclipse Precision is the OS-owned image-backed surface contract, not an application style.
pub type Rgba = (u8, u8, u8, u8);
pub type Rgb = (u8, u8, u8);
const SIZE: usize = 256;
const CELLS: usize = 14;
const ART: &[u8; 2 * CELLS * SIZE * SIZE * 4] = include_bytes!("../../assets/ui-design-kit/default/eclipse-precision-v1.rgba");

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Surface { Frame, WindowFill, Panel, Primary, Secondary, Field, Current, Inactive }

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
    let (sx,sy)=(fx/256,fy/256); let (wx,wy)=(fx%256,fy%256);
    let address=|x:usize,y:usize| (index*SIZE*SIZE+y.min(SIZE-1)*SIZE+x.min(SIZE-1))*4;
    let offsets=[address(sx,sy),address(sx+1,sy),address(sx,sy+1),address(sx+1,sy+1)];
    if wx==0 && wy==0 {
        let at=offsets[0];return (ART[at],ART[at+1],ART[at+2],ART[at+3]);
    }
    if offsets.iter().all(|at|ART[at+3]==255) {
        let weights=[(256-wx)*(256-wy),wx*(256-wy),(256-wx)*wy,wx*wy];
        let channel=|c:usize| ((0..4).map(|i|usize::from(ART[offsets[i]+c])*weights[i]).sum::<usize>()>>16) as u8;
        return (channel(0),channel(1),channel(2),255);
    }
    let mut channels=[0u64;4];
    for (dy,vy) in [(0,256-wy),(1,wy)] {for (dx,vx) in [(0,256-wx),(1,wx)] {
        let at=(index*SIZE*SIZE+(sy+dy).min(SIZE-1)*SIZE+(sx+dx).min(SIZE-1))*4;
        let weight=(vx*vy) as u64; let alpha=ART[at+3] as u64;
        for c in 0..3 {channels[c]+=ART[at+c] as u64*alpha*weight;}
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
        themed(state(0),base,m.fill,m.opacity)
        }
    }else{(0,0,0,0)};
    let edge=if m.edge_opacity!=0 && !circular && distance<rim {
        themed(state(1),(134,199,255),m.edge,m.edge_opacity)
    }else{(0,0,0,0)};
    over(background,edge)
}
