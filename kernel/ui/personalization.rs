//! Shared native personalization geometry; input and painting use identical regions.
use super::geometry::{Point, Rect};
use core::sync::atomic::{AtomicU8,Ordering};
static SAVE_STATUS: AtomicU8=AtomicU8::new(0);
// ------------------------=
// FUNC: save_status
// DESC: Exposes bounded feedback for the visible personalization controls.
// ------------------=
pub fn save_status()->u8 {SAVE_STATUS.load(Ordering::Relaxed)}
// ------------------------=
// FUNC: set_save_status
// DESC: Publishes preview, successful persistence or rollback feedback without storing user content.
// ------------------=
pub fn set_save_status(status:u8) {SAVE_STATUS.store(status.min(3),Ordering::Relaxed);}
pub const CURSOR_NAMES: [&[u8]; 10] = [b"Classic White", b"Classic Black", b"Outline", b"Crystal", b"Silver", b"Comet", b"Rocket", b"Leaf", b"Wand", b"Pixel"];
pub const TINT_NAMES: [&[u8]; 5] = [b"Neutral", b"Navy", b"Teal", b"Violet", b"Amber"];
pub const TINTS: [[u8;4];5] = [[20,24,30,165],[8,23,42,165],[4,39,42,165],[30,18,48,165],[44,30,14,165]];
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target { Slider(usize), Choice(usize) }
pub struct Layout { pub sliders: [Rect;4], pub choices: [Rect;10], pub pointer: bool }
impl Layout {
    // ------------------------=
    // FUNC: new
    // DESC: Lays out responsive cursor cards or tint swatches beneath accessible sliders.
    // ------------------=
    pub fn new(detail: Rect, scale: usize, pointer: bool) -> Self {
        let s=scale.max(1) as i32; let width=(detail.width as i32-32*s).max(30);
        let sliders=core::array::from_fn(|i| Rect {x:detail.x+16*s,y:detail.y+(42+i as i32*62)*s,width:width as u32,height:(20*s) as u32});
        let columns=if pointer && width<420*s {3} else {5};
        let step=width/columns; let top=if pointer {162} else {280};
        let choices=core::array::from_fn(|i| Rect {x:detail.x+16*s+i as i32%columns*step,y:detail.y+(top+(i as i32/columns)*108)*s,width:(step-8*s).max(12) as u32,height:((if pointer {96}else{62})*s) as u32});
        Self { sliders, choices, pointer }
    }
    // ------------------------=
    // FUNC: hit
    // DESC: Ignores clipped controls and gutters rather than activating hidden content.
    // ------------------=
    pub fn hit(&self, point: Point, viewport: Rect) -> Option<Target> {
        if !viewport.contains(point) {return None;}
        for i in 0..if self.pointer {2}else{4} {if self.sliders[i].contains(point) {return Some(Target::Slider(i));}}
        for i in 0..if self.pointer {10}else{5} {if self.choices[i].contains(point) {return Some(Target::Choice(i));}}
        None
    }
    // ------------------------=
    // FUNC: value
    // DESC: Maps a captured pointer to a clamped inclusive slider range, including off-track release.
    // ------------------=
    pub fn value(&self, index: usize, x: i32, maximum: u8) -> u8 {
        let r=self.sliders[index];
        ((x-r.x).clamp(0,r.width.saturating_sub(1) as i32) as u32*maximum as u32/r.width.saturating_sub(1).max(1)) as u8
    }
}
