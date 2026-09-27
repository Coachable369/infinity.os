//! One bounded native window order for both painting and activation.
use super::geometry::{Point, Rect};
pub const BROWSER:usize=5;
pub const SURFACES:usize=6;
#[derive(Clone, Copy)]
pub struct Stack {
    pub order: [usize; SURFACES],
    pub visible: [bool; SURFACES],
    pub active: usize,
    pub settings_section: usize,
}
impl Stack {
    // ------------------------=
    // FUNC: new
// DESC: Initializes native desktop surfaces, including the browser, in stable back-to-front order.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            order: [0, 1, 2, 3, 4, BROWSER],
            visible: [false; SURFACES],
            active: 0,
            settings_section: 0,
        }
    }
    // ------------------------=
    // FUNC: sync
    // DESC: Raises only the active visible window, preserving the relative order of other windows.
    // ------------------=
    pub fn sync<const N:usize>(&mut self, visible: [bool; N], active: usize, section: usize) {
        assert!(N<=SURFACES);
        self.visible.fill(false);
        self.visible[..N].copy_from_slice(&visible);
        if active == 4 {
            self.settings_section = section;
        }
        if active < N && visible[active] {
            let mut next = [0; SURFACES];
            let mut count = 0;
            for id in self.order {
                if id != active {
                    next[count] = id;
                    count += 1;
                }
            }
            next[SURFACES-1] = active;
            self.order = next;
            self.active = active;
        }
    }
    // ------------------------=
    // FUNC: hit
    // DESC: Resolves the same topmost visible rectangle that painting puts above its peers.
    // ------------------=
    pub fn hit<const N:usize>(&self, bounds: [Rect; N], point: Point) -> Option<usize> {
        self.order
            .iter()
            .rev()
            .copied()
            .find(|id| *id<N && self.visible[*id] && bounds[*id].contains(point))
    }
}
static mut STATE: Stack = Stack::new();
// ------------------------=
// FUNC: publish
// DESC: Updates UI-thread ordering without allocating or repainting on pointer motion.
// ------------------=
pub fn publish<const N:usize>(visible: [bool; N], active: usize, section: usize) {
    unsafe {
        (&mut *(&raw mut STATE)).sync(visible, active, section);
    }
}
// ------------------------=
// FUNC: current
// DESC: Reads the current immutable UI-thread ordering snapshot.
// ------------------=
pub fn current() -> Stack {
    unsafe { *(&raw const STATE) }
}
