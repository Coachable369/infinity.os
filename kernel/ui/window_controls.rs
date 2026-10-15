//! Shared image-backed chrome geometry and UI-thread-only hover presentation.
use crate::ui::geometry::{Point, Rect};
const SIDE: usize = 96;
const IMAGE: &[u8; SIDE * SIDE * 4 * 3] = include_bytes!("../../assets/ui-design-kit/default/window-controls-v1.rgba");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Highlight { pub rect: Rect, pub pressed: bool }
static mut CURRENT: Option<Highlight> = None;
static mut PAINTED: Option<Highlight> = None;

// ------------------------=
// FUNC: bounds
// DESC: Shares top-right button geometry between painting and hit testing.
// ------------------=
pub fn bounds(window: Rect, title_height: u32, scale: usize) -> [Rect; 3] {
    let s=scale.max(1) as u32;
    core::array::from_fn(|i| Rect {
        x: window.right() - ((12+26+(2-i as u32)*34)*s) as i32,
        y: window.y + (title_height.saturating_sub(26*s)/2) as i32,
        width:26*s, height:26*s,
    })
}

// ------------------------=
// FUNC: hit
// DESC: Resolves a control without consuming title dragging or content clicks.
// ------------------=
pub fn hit(controls: [Rect;3], point: Point) -> Option<usize> {
    controls.iter().position(|r| r.contains(point))
}

// ------------------------=
// FUNC: publish
// DESC: Updates hover state only on semantic transitions rather than every pointer pixel.
// ------------------=
pub fn publish(next: Option<Highlight>) -> bool {
    unsafe { let changed=CURRENT!=next; CURRENT=next; changed }
}

// ------------------------=
// FUNC: state
// DESC: Returns the authored state for an exact visible control rectangle.
// ------------------=
pub fn state(rect: Rect) -> usize {
    unsafe { CURRENT.filter(|h| h.rect==rect).map(|h| if h.pressed {2}else{1}).unwrap_or(0) }
}

// ------------------------=
// FUNC: take_damage
// DESC: Returns separate old/new bounds, avoiding a large union across distant windows.
// ------------------=
pub fn take_damage() -> [Option<Rect>;2] {
    unsafe {
        if CURRENT==PAINTED {return [None,None];}
        let old=PAINTED; PAINTED=CURRENT;
        [old.map(|h|h.rect),CURRENT.map(|h|h.rect)]
    }
}

// ------------------------=
// FUNC: pixel
// DESC: Area-filters authored alpha and tints neutral glass without allocations or decoding.
// ------------------=
pub fn pixel(size: usize,x:usize,y:usize,state:usize,primary:u32,accent:u32,_close:bool)->(u8,u8,u8,u8) {
    if size==0 || x>=size || y>=size {return (0,0,0,0);}
    let (x0,y0)=(x*SIDE/size,y*SIDE/size);
    let (x1,y1)=(((x+1)*SIDE/size).max(x0+1).min(SIDE),((y+1)*SIDE/size).max(y0+1).min(SIDE));
    let mut lum=0u32; let mut alpha=0u32;
    for sy in y0..y1 {for sx in x0..x1 {
        let at=(state.min(2)*SIDE*SIDE+sy*SIDE+sx)*4;
        let a=IMAGE[at+3] as u32; alpha+=a;
        lum+=IMAGE[at] as u32*a;
    }}
    if alpha==0 {return (0,0,0,0);}
    let lum=lum/alpha;
    // Keep the authored reflections instead of flooding the whole face with accent.
    // Closing is a normal window action, not a destructive/error state.
    let accent_weight=if state==0 {0}else if state==1 {32}else{20};
    let reflection=lum.saturating_sub(48)*3/5;
    let channel=|shift:u32| {
        let base=(primary>>shift)&255u32;
        let accent=(accent>>shift)&255u32;
        let glass=(base*(256-accent_weight)+accent*accent_weight)/256;
        let glass=if state==2 {glass*4/5}else{glass};
        (glass+(255-glass)*reflection/255).min(255) as u8
    };
    (channel(16),channel(8),channel(0),(alpha/((x1-x0)*(y1-y0)) as u32) as u8)
}
