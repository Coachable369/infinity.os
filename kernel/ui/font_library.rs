//! Shared Settings font geometry and bounded navigation state.
use super::geometry::{Point, Rect};
use core::sync::atomic::{AtomicBool, AtomicU8, Ordering};
static DIRTY: AtomicBool=AtomicBool::new(false);
static PAGE: AtomicU8=AtomicU8::new(0);
static FOCUS: AtomicU8=AtomicU8::new(0);
static STATUS: AtomicU8=AtomicU8::new(0);
pub const ROWS: usize=6;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action { Use(u8), Toggle(u8), Previous, Next }
pub struct Layout { pub rows:[Rect;ROWS], pub select:[Rect;ROWS],pub toggle:[Rect;ROWS],pub previous:Rect,pub next:Rect,pub footer:Rect }
// ------------------------=
// FUNC: page
// DESC: Reads the bounded page index used by both input and painting.
// ------------------=
pub fn page()->usize {PAGE.load(Ordering::Relaxed) as usize}
// ------------------------=
// FUNC: set_page
// DESC: Keeps navigation within the 46-family catalog including the final partial page.
// ------------------=
pub fn set_page(value:usize) {PAGE.store(value.min(7) as u8,Ordering::Relaxed);set_focus(focus());DIRTY.store(true,Ordering::Release);}
// ------------------------=
// FUNC: focus
// DESC: Returns the keyboard-focused visible font row.
// ------------------=
pub fn focus()->usize {FOCUS.load(Ordering::Relaxed) as usize}
// ------------------------=
// FUNC: set_focus
// DESC: Clamps keyboard focus to a real family on the current page and requests one repaint.
// ------------------=
pub fn set_focus(value:usize) {FOCUS.store(value.min(if page()==7 {3}else{5}) as u8,Ordering::Relaxed);DIRTY.store(true,Ordering::Release);}
// ------------------------=
// FUNC: status
// DESC: Reads non-sensitive save feedback, independent of the persisted registry.
// ------------------=
pub fn status()->u8 {STATUS.load(Ordering::Relaxed)}
// ------------------------=
// FUNC: set_status
// DESC: Publishes save success or failure without claiming failed mutations were applied.
// ------------------=
pub fn set_status(value:u8) {STATUS.store(value,Ordering::Relaxed);DIRTY.store(true,Ordering::Release);}
// ------------------------=
// FUNC: take_damage
// DESC: Invalidates retained text surfaces once after a font or library presentation change.
// ------------------=
pub fn take_damage()->bool {DIRTY.swap(false,Ordering::AcqRel)}
impl Layout {
    // ------------------------=
    // FUNC: new
    // DESC: Constructs six compact two-line family cards and disjoint right-aligned action targets.
    // ------------------=
    pub fn new(detail:Rect,scale:usize)->Self {
        let s=scale.max(1) as i32;
        let w=(detail.width as i32-32*s).max(0) as u32;
        let rows=core::array::from_fn(|i|Rect{x:detail.x+16*s,y:detail.y+(16+i as i32*64)*s,width:w,height:(56*s) as u32});
        let button=(80*s) as u32;
        let select=core::array::from_fn(|i|Rect{x:rows[i].right()- (176*s),y:rows[i].y+12*s,width:button,height:(32*s) as u32});
        let toggle=core::array::from_fn(|i|Rect{x:rows[i].right()- (88*s),y:rows[i].y+12*s,width:button,height:(32*s) as u32});
        let footer=Rect{x:detail.x+16*s,y:detail.y+412*s,width:w,height:(32*s) as u32};
        Self { rows,select,toggle,footer,previous:Rect{x:footer.x,y:footer.y,width:button,height:footer.height},
            next:Rect{x:footer.x+88*s,y:footer.y,width:button,height:footer.height} }
    }
    // ------------------------=
    // FUNC: hit
    // DESC: Rejects clipped rows, gaps, protected fallback removal and nonexistent final-page families.
    // ------------------=
    pub fn hit(&self,point:Point,viewport:Rect,page:usize)->Option<Action> {
        if !viewport.contains(point) || page>7 {return None;}
        if page>0 && self.previous.contains(point) {return Some(Action::Previous);}
        if page<7 && self.next.contains(point) {return Some(Action::Next);}
        for i in 0..ROWS {
            let id=(page*ROWS+i+1) as u8;if id>46 {break;}
            if self.select[i].contains(point) {return Some(Action::Use(id));}
            if id!=2 && self.toggle[i].contains(point) {return Some(Action::Toggle(id));}
        }
        None
    }
}
