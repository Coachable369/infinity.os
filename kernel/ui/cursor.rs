//! Built-in cursor sprites are linked into both live and installed kernels.
use super::geometry::Rect;
use super::input_preferences::Preferences;
use core::sync::atomic::{AtomicUsize,Ordering};
static BUSY_FRAME:AtomicUsize=AtomicUsize::new(0);
static BUSY_SPRITE:&[u8;60*64*64*4]=include_bytes!("../../assets/ui-design-kit/default/wait-orbit-v2.rgba");
// ------------------------=
// FUNC: animate_busy
// DESC: Publishes one fixed-rate sprite frame and reports transitions including return to the normal pointer.
// ------------------=
pub fn animate_busy(loading:bool,now_ms:u64)->bool {
    let next=if loading {1+((now_ms%1000)*60/1000) as usize}else{0};
    BUSY_FRAME.swap(next,Ordering::Relaxed)!=next
}
// ------------------------=
// FUNC: busy
// DESC: Reports the active wait-cursor state without querying application services during painting.
// ------------------=
pub fn busy()->bool {BUSY_FRAME.load(Ordering::Relaxed)!=0}
// ------------------------=
// FUNC: busy_sample
// DESC: Samples the authored animation in a bounded fixed footprint.
// ------------------=
pub fn busy_sample(x:usize,y:usize,size:usize)->[u8;4] {
    if size==0 || x>=size || y>=size {return [0;4];}
    let frame=BUSY_FRAME.load(Ordering::Relaxed).saturating_sub(1).min(59);
    // Alpha-weighted area sampling avoids jagged transparent edges at small sizes.
    let x0=x*64/size;let x1=((x+1)*64/size).max(x0+1).min(64);
    let y0=y*64/size;let y1=((y+1)*64/size).max(y0+1).min(64);
    let mut sum=[0u32;4];let count=((x1-x0)*(y1-y0)) as u32;
    for sy in y0..y1 {for sx in x0..x1 {
        let i=(frame*64*64+sy*64+sx)*4;let a=BUSY_SPRITE[i+3] as u32;
        for c in 0..3 {sum[c]+=BUSY_SPRITE[i+c] as u32*a;}sum[3]+=a;
    }}
    if sum[3]==0 {return [0;4];}
    [(sum[0]/sum[3]) as u8,(sum[1]/sum[3]) as u8,(sum[2]/sum[3]) as u8,(sum[3]/count) as u8]
}
include!("../../assets/cursors/hotspots.rs");
pub const EDGE: usize = 128;
pub static SPRITES: [&[u8; EDGE * EDGE * 4]; 10] = [
    include_bytes!("../../assets/cursors/classic-white.rgba"),
    include_bytes!("../../assets/cursors/classic-black.rgba"),
    include_bytes!("../../assets/cursors/outline.rgba"),
    include_bytes!("../../assets/cursors/crystal.rgba"),
    include_bytes!("../../assets/cursors/silver.rgba"),
    include_bytes!("../../assets/cursors/comet.rgba"),
    include_bytes!("../../assets/cursors/rocket.rgba"),
    include_bytes!("../../assets/cursors/leaf.rgba"),
    include_bytes!("../../assets/cursors/wand.rgba"),
    include_bytes!("../../assets/cursors/pixel.rgba"),
];
// ------------------------=
// FUNC: size
// DESC: Returns the selected physical cursor size within the 128-pixel restoration buffer.
// ------------------=
pub fn size(p: Preferences, scale: usize) -> usize {
    ([20,28,36,44,52,60][p.cursor_size.clamp(1,6) as usize-1] * scale.max(1)).min(120)
}
// ------------------------=
// FUNC: shape_scale
// DESC: Scales text and resize affordances to the user's cursor preference without overflowing saved pixels.
// ------------------=
pub fn shape_scale(p: Preferences, scale: usize) -> usize { ((size(p,scale)+13)/28).clamp(1,4) }
// ------------------------=
// FUNC: bounds
// DESC: Shares hotspot-aware paint and restoration geometry; hit-test coordinates remain unchanged.
// ------------------=
pub fn bounds(x: i32, y: i32, scale: usize, p: Preferences, special: bool) -> Rect {
    if busy() {
        let edge=size(p,scale).max((32*scale).min(120));
        return Rect{x:x-edge as i32/2,y:y-edge as i32/2,width:edge as u32,height:edge as u32};
    }
    let size=if special {28*shape_scale(p,scale)} else {size(p,scale)};
    let (hx,hy)=if special {(0,0)} else {HOTSPOTS[p.cursor_style.min(9) as usize]};
    Rect {x:x-(hx*size/EDGE) as i32,y:y-(hy*size/EDGE) as i32,width:size as u32,height:size as u32}
}
// ------------------------=
// FUNC: sample
// DESC: Anchors resampling to the opaque click tip so tiny pointers remain visible at screen edges.
// ------------------=
pub fn sample(style: usize, x: usize, y: usize, size: usize) -> [u8;4] {
    if size==0 {return [0;4];}
    let style=style.min(9);let (hx,hy)=HOTSPOTS[style];
    let sx=hx as isize+(x as isize-(hx*size/EDGE) as isize)*EDGE as isize/size as isize;
    let sy=hy as isize+(y as isize-(hy*size/EDGE) as isize)*EDGE as isize/size as isize;
    if sx<0 || sy<0 || sx>=EDGE as isize || sy>=EDGE as isize {return [0;4];}
    let i=(sy as usize*EDGE+sx as usize)*4;
    let p=SPRITES[style];[p[i],p[i+1],p[i+2],p[i+3]]
}
