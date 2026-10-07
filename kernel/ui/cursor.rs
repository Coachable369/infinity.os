//! Built-in cursor sprites are linked into both live and installed kernels.
use super::geometry::Rect;
use super::input_preferences::Preferences;
use core::sync::atomic::{AtomicUsize,Ordering};
static BUSY_FRAME:AtomicUsize=AtomicUsize::new(0);
static BUSY_SPRITE:&[u8;16*64*64*4]=include_bytes!("../../assets/ui-design-kit/default/wait-infinity-v1.rgba");
// ------------------------=
// FUNC: animate_busy
// DESC: Publishes one fixed-rate sprite frame and reports transitions including return to the normal pointer.
// ------------------=
pub fn animate_busy(loading:bool,now_ms:u64)->bool {
    let next=if loading {1+(now_ms/80%16) as usize}else{0};
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
    let frame=BUSY_FRAME.load(Ordering::Relaxed).saturating_sub(1).min(15);
    let i=(frame*64*64+y*64/size*64+x*64/size)*4;
    [BUSY_SPRITE[i],BUSY_SPRITE[i+1],BUSY_SPRITE[i+2],BUSY_SPRITE[i+3]]
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
        let edge=size(p,scale).max((40*scale).min(120));
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
