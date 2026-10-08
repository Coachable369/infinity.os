//! Finite, UI-thread-owned focus and peek transitions; no window ownership changes.
use crate::ui::app_launcher::motion::Motion;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Presentation {
    pub dim: u8,
    pub peek: Option<usize>,
    pub opacity: u8,
}

pub struct Effects {
    enabled: bool,
    lens: Motion,
    peek_motion: Motion,
    owner: Option<usize>,
    deadline: u64,
    pub view: Presentation,
}
impl Effects {
    // ------------------------=
    // FUNC: new
    // DESC: Initializes settled, disabled effects without allocating surfaces or timers.
    // ------------------=
    pub const fn new() -> Self {
        Self { enabled: false, lens: Motion::settled(0), peek_motion: Motion::settled(255),
            owner: None, deadline: 0, view: Presentation { dim: 0, peek: None, opacity: 255 } }
    }
    // ------------------------=
    // FUNC: toggle_lens
    // DESC: Reverses the finite focus transition from its current sample.
    // ------------------=
    pub fn toggle_lens(&mut self, now: u64, reduced: bool) {
        self.enabled = !self.enabled;
        self.lens.retarget(if self.enabled { 144 } else { 0 }, now, 180, reduced);
    }
    // ------------------------=
    // FUNC: peek
    // DESC: Briefly reveals the background beneath a visible active window without raising it.
    // ------------------=
    pub fn peek(&mut self, owner: usize, now: u64, reduced: bool) {
        self.owner = Some(owner);
        self.deadline = now.saturating_add(2500);
        self.peek_motion.retarget(0, now, 140, reduced);
    }
    // ------------------------=
    // FUNC: dismiss_peek
    // DESC: Restores pixels immediately before subsequent input can be dispatched.
    // ------------------=
    pub fn dismiss_peek(&mut self) -> bool {
        let changed = self.owner.take().is_some();
        self.peek_motion = Motion::settled(255);
        changed
    }
    // ------------------------=
    // FUNC: advance
    // DESC: Samples elapsed time, expires peeks, and emits changes only when presentation changes.
    // ------------------=
    pub fn advance(&mut self, now: u64, active: Option<usize>, reduced: bool) -> bool {
        if self.owner.is_some() && (self.owner != active || now >= self.deadline) {
            if self.owner != active { self.dismiss_peek(); }
            else { self.peek_motion.retarget(255, now, 140, reduced); self.deadline = u64::MAX; }
        }
        let opacity = self.peek_motion.value(now).clamp(0,255) as u8;
        if opacity == 255 && !self.peek_motion.active(now) { self.owner = None; }
        let next = Presentation {
            dim: if active.is_some() && self.owner.is_none() { self.lens.value(now).clamp(0,255) as u8 } else {0},
            peek: self.owner, opacity,
        };
        let changed = next != self.view;
        self.view = next;
        changed
    }
}
static mut STATE: Effects = Effects::new();
static mut DIRTY: u8 = 0;
static mut LAYER: Option<usize> = None;

// ------------------------=
// FUNC: mutate
// DESC: Updates effect state only from the serialized desktop UI owner.
// ------------------=
pub fn mutate(action: impl FnOnce(&mut Effects)) { unsafe { action(&mut *(&raw mut STATE)); } }
// ------------------------=
// FUNC: advance
// DESC: Publishes one bounded effect sample and records compositor damage independently of app content.
// ------------------=
pub fn advance(now: u64, active: Option<usize>, reduced: bool) -> bool {
    unsafe {
        let state=&mut *(&raw mut STATE);
        let old=state.view;
        let changed=state.advance(now,active,reduced);
        if changed {DIRTY |= if old.dim!=state.view.dim {2}else{1};}
        changed
    }
}
// ------------------------=
// FUNC: current
// DESC: Returns immutable presentation values without allocation or service queries.
// ------------------=
pub fn current() -> Presentation { unsafe { (*(&raw const STATE)).view } }
// ------------------------=
// FUNC: take_damage
// DESC: Consumes effect damage once; stationary pointers do not create presentation work.
// ------------------=
pub fn take_damage() -> u8 { unsafe { core::mem::replace(&mut *(&raw mut DIRTY),0) } }
// ------------------------=
// FUNC: layer
// DESC: Selects a composition layer without altering activation or persistent z-order.
// ------------------=
pub fn layer(id: Option<usize>) { unsafe { LAYER=id; } }
// ------------------------=
// FUNC: opacity
// DESC: Applies peek only to the current owner layer, never to unrelated cached surfaces.
// ------------------=
pub fn opacity() -> u8 {
    let view=current();
    if view.peek.is_some() && view.peek==unsafe {LAYER} {view.opacity} else {255}
}
// ------------------------=
// FUNC: reset
// DESC: Clears all transient effects at a security/session boundary.
// ------------------=
pub fn reset() { unsafe { STATE=Effects::new(); DIRTY=2; LAYER=None; } }
// ------------------------=
// FUNC: fade_pixel
// DESC: Scales premultiplied RGB and alpha equally without allocating or changing channel order.
// ------------------=
pub fn fade_pixel(pixel:u32,opacity:u8)->u32 {
    let mut out=0;
    for shift in [0,8,16,24] {out |= (((pixel>>shift)&255)*u32::from(opacity)/255)<<shift;}
    out
}
