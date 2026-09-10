//! One bounded native window order for both painting and activation.
use super::geometry::{Point, Rect};
#[derive(Clone, Copy)]
pub struct Stack {
    pub order: [usize; 5],
    pub visible: [bool; 5],
    pub active: usize,
    pub settings_section: usize,
}
impl Stack {
    // ------------------------=
    // FUNC: new
    // DESC: Initializes the five native desktop surfaces in stable back-to-front order.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            order: [0, 1, 2, 3, 4],
            visible: [false; 5],
            active: 0,
            settings_section: 0,
        }
    }
    // ------------------------=
    // FUNC: sync
    // DESC: Raises only the active visible window, preserving the relative order of other windows.
    // ------------------=
    pub fn sync(&mut self, visible: [bool; 5], active: usize, section: usize) {
        self.visible = visible;
        if active == 4 {
            self.settings_section = section;
        }
        if active < 5 && visible[active] {
            let mut next = [0; 5];
            let mut count = 0;
            for id in self.order {
                if id != active {
                    next[count] = id;
                    count += 1;
                }
            }
            next[4] = active;
            self.order = next;
            self.active = active;
        }
    }
    // ------------------------=
    // FUNC: hit
    // DESC: Resolves the same topmost visible rectangle that painting puts above its peers.
    // ------------------=
    pub fn hit(&self, bounds: [Rect; 5], point: Point) -> Option<usize> {
        self.order
            .iter()
            .rev()
            .copied()
            .find(|id| self.visible[*id] && bounds[*id].contains(point))
    }
}
static mut STATE: Stack = Stack::new();
// ------------------------=
// FUNC: publish
// DESC: Updates UI-thread ordering without allocating or repainting on pointer motion.
// ------------------=
pub fn publish(visible: [bool; 5], active: usize, section: usize) {
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
