//! UI-thread-owned desktop icon drag presentation. Persist only on release.
use crate::ui::{geometry::Rect, app_launcher::motion::Motion};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Drag {
    pub item: usize,
    pub position: [i32; 2],
    pub source: Option<[i32; 2]>,
    pub held: bool,
    pub emphasis: u8,
}
// ------------------------=
// FUNC: crossed_threshold
// DESC: Measures drag slop in physical pixels so high-resolution desktops do not require exaggerated motion.
// ------------------=
pub fn crossed_threshold(grab: [i32;2], pointer: [i32;2], width: usize, height: usize) -> bool {
    let dx=i64::from(pointer[0]-grab[0]).abs()*width as i64/1000;
    let dy=i64::from(pointer[1]-grab[1]).abs()*height as i64/1000;
    dx*dx+dy*dy >= 64
}
// ------------------------=
// FUNC: anchored_position
// DESC: Preserves the exact grab offset while constraining the icon to the usable desktop.
// ------------------=
pub fn anchored_position(origin: [i32; 2], grab: [i32; 2], pointer: [i32; 2]) -> [i32; 2] {
    [(origin[0] + pointer[0] - grab[0]).clamp(35, 950),
     (origin[1] + pointer[1] - grab[1]).clamp(90, 880)]
}
// ------------------------=
// FUNC: bounds
// DESC: Covers the shared icon, label and soft highlight in physical pixels.
// ------------------=
pub fn bounds(position: [i32; 2], width: usize, height: usize) -> Rect {
    let size = (height / 21).max(44) as u32;
    Rect { x: (width as i32 * position[0] / 1000) - 12,
        y: (height as i32 * position[1] / 1000) - 12,
        width: size.max(180) + 24, height: size + 64 }
}
static mut VIEW: Option<Drag> = None;
static mut PREVIOUS: Option<Drag> = None;
static mut DIRTY: bool = false;
static mut EMPHASIS: Motion = Motion::settled(0);
// ------------------------=
// FUNC: current
// DESC: Returns the single UI-thread presentation snapshot.
// ------------------=
pub fn current() -> Option<Drag> { unsafe { VIEW } }
// ------------------------=
// FUNC: publish
// DESC: Retains the first unpresented position so coalesced movement restores every old pixel.
// ------------------=
fn publish(next: Option<Drag>) {
    unsafe { if VIEW != next { if !DIRTY { PREVIOUS = VIEW; } VIEW = next; DIRTY = true; } }
}
// ------------------------=
// FUNC: motion
// DESC: Tracks pointer position immediately while animating only the lift highlight.
// ------------------=
pub fn motion(item: usize, position: [i32; 2], source: Option<[i32; 2]>, now: u64, reduced: bool) {
    unsafe {
        if !VIEW.map(|v| v.held && v.item == item).unwrap_or(false) {
            EMPHASIS = Motion::settled(0);
            (&mut *(&raw mut EMPHASIS)).retarget(255, now, 100, reduced);
        }
        publish(Some(Drag { item, position, source, held: true,
            emphasis: (&*(&raw const EMPHASIS)).value(now).clamp(0,255) as u8 }));
    }
}
// ------------------------=
// FUNC: finish
// DESC: Ends capture with a finite drop highlight or immediately cancels without changing stored positions.
// ------------------=
pub fn finish(accepted: bool, now: u64, reduced: bool) {
    unsafe {
        if accepted && !reduced {
            if let Some(mut view) = VIEW {
                view.held = false; view.source = None;
                EMPHASIS = Motion::settled(255);
                (&mut *(&raw mut EMPHASIS)).retarget(0, now, 160, false);
                view.emphasis = 255; publish(Some(view));
            }
        } else { publish(None); }
    }
}
// ------------------------=
// FUNC: advance
// DESC: Samples finite lift/drop motion without scheduling work after settlement.
// ------------------=
pub fn advance(now: u64) -> bool {
    unsafe {
        let Some(mut view) = VIEW else { return false; };
        let emphasis = (&*(&raw const EMPHASIS)).value(now).clamp(0,255) as u8;
        if !view.held && emphasis == 0 { publish(None); return true; }
        if view.emphasis == emphasis { return false; }
        view.emphasis = emphasis; publish(Some(view)); true
    }
}
// ------------------------=
// FUNC: take_damage
// DESC: Reconstructs bounded old/new drag and source regions once per frame.
// ------------------=
pub fn take_damage(width: usize, height: usize) -> Option<Rect> {
    unsafe {
        if !DIRTY { return VIEW.map(|v| bounds(v.position, width, height)); }
        DIRTY = false;
        let source_changed = PREVIOUS.map(|v| (v.item,v.source,v.held)) != VIEW.map(|v| (v.item,v.source,v.held));
        let mut damage: Option<Rect> = None;
        for view in [PREVIOUS, VIEW].into_iter().flatten() {
            for p in [Some(view.position), if source_changed { view.source } else { None }].into_iter().flatten() {
                let r = bounds(p, width, height);
                damage = Some(damage.map(|d| d.union(r)).unwrap_or(r));
            }
        }
        damage
    }
}
