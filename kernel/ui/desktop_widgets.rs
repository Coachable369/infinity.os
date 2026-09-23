//! UI-thread widget placement, bounded damage, and architecture-neutral persistence.
use super::geometry::{Point, Rect};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct State {
    pub positions: [[u16; 2]; 2],
    pub visible: u8,
    pub drag: Option<(usize, i32, i32)>,
    pub menu: Option<Point>,
}
impl State {
    // ------------------------=
    // FUNC: new
    // DESC: Keeps legacy right-column placement until a user moves a widget.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            positions: [[0; 2]; 2],
            visible: 3,
            drag: None,
            menu: None,
        }
    }
    // ------------------------=
    // FUNC: encode
    // DESC: Packs only persistent placement and visibility into reserved session bytes.
    // ------------------=
    pub fn encode(self) -> u64 {
        let mut value = (1u64 << 63) | ((self.visible as u64 & 3) << 40);
        for i in 0..4 {
            value |= (self.positions[i / 2][i % 2].min(1001) as u64) << (i * 10);
        }
        value
    }
    // ------------------------=
    // FUNC: decode
    // DESC: Restores layout while dropping transient drag and menu capture; zero is legacy default.
    // ------------------=
    pub fn decode(value: u64) -> Self {
        let mut state = Self::new();
        if value >> 63 == 0 {
            return state;
        }
        state.visible = ((value >> 40) & 3) as u8;
        for i in 0..4 {
            state.positions[i / 2][i % 2] = (((value >> (i * 10)) & 1023) as u16).min(1001);
        }
        state
    }
    // ------------------------=
    // FUNC: rect
    // DESC: Shares clamped physical widget geometry between painting, hit testing and damage.
    // ------------------=
    pub fn rect(
        self,
        id: usize,
        width: usize,
        height: usize,
        scale: usize,
        minimized: bool,
    ) -> Rect {
        let top = height * 7 / 100;
        let overview_height = (330 * scale).min(height * 30 / 100);
        let default_y = if id == 0 {
            top
        } else {
            top + overview_height + 20 * scale
        };
        let bottom = height.saturating_sub(102 * scale);
        let h = if id == 0 {
            overview_height
        } else if minimized {
            50 * scale
        } else {
            bottom.saturating_sub(default_y)
        };
        let w = width * 22 / 100;
        let [x, y] = self.positions[id];
        let x = if x == 0 {
            width * 68 / 100
        } else {
            (x as usize - 1) * width / 1000
        };
        let y = if y == 0 {
            default_y
        } else {
            (y as usize - 1) * height / 1000
        };
        Rect {
            x: x.min(width.saturating_sub(w + 8 * scale)) as i32,
            y: y.max(top).min(bottom.saturating_sub(h).max(top)) as i32,
            width: w as u32,
            height: h as u32,
        }
    }
    // ------------------------=
    // FUNC: move_pointer
    // DESC: Tracks a captured header and snaps the final drop to work-area and adjacent-widget edges.
    // ------------------=
    pub fn move_pointer(
        &mut self,
        point: Point,
        held: bool,
        width: usize,
        height: usize,
        scale: usize,
        minimized: bool,
    ) {
        let Some((id, grab_x, grab_y)) = self.drag else {
            return;
        };
        let bounds = self.rect(id, width, height, scale, minimized && id == 1);
        let mut x = point.x - grab_x;
        let mut y = point.y - grab_y;
        let left = (8 * scale) as i32;
        let right = width.saturating_sub(bounds.width as usize + 8 * scale) as i32;
        let top = (height * 7 / 100) as i32;
        let bottom = height
            .saturating_sub(102 * scale + bounds.height as usize)
            .max(top as usize) as i32;
        if !held {
            let other = self.rect(1 - id, width, height, scale, minimized && id == 0);
            for edge in [left, right] {
                if (x - edge).abs() <= (20 * scale) as i32 {
                    x = edge;
                }
            }
            for edge in [top, bottom] {
                if (y - edge).abs() <= (20 * scale) as i32 {
                    y = edge;
                }
            }
            if self.visible & (1 << (1 - id)) != 0 {
                for edge in [
                    other.x,
                    other.right() + (12 * scale) as i32,
                    other.x - bounds.width as i32 - (12 * scale) as i32,
                ] {
                    if (x - edge).abs() <= (12 * scale) as i32 {
                        x = edge;
                    }
                }
                for edge in [
                    other.y,
                    other.bottom() + (12 * scale) as i32,
                    other.y - bounds.height as i32 - (12 * scale) as i32,
                ] {
                    if (y - edge).abs() <= (12 * scale) as i32 {
                        y = edge;
                    }
                }
            }
            self.drag = None;
        }
        self.positions[id] = [
            (x.clamp(left.min(right), right).max(0) as usize * 1000 / width.max(1) + 1) as u16,
            (y.clamp(top, bottom).max(0) as usize * 1000 / height.max(1) + 1) as u16,
        ];
    }
    // ------------------------=
    // FUNC: menu_rect
    // DESC: Keeps the widget chooser inside the display at the click position.
    // ------------------=
    pub fn menu_rect(self, width: usize, height: usize, scale: usize) -> Option<Rect> {
        self.menu.map(|p| {
            let w = (260 * scale).min(width);
            let h = (152 * scale).min(height);
            Rect {
                x: p.x.clamp(0, width.saturating_sub(w) as i32),
                y: p.y.clamp(0, height.saturating_sub(h) as i32),
                width: w as u32,
                height: h as u32,
            }
        })
    }
}
static mut STATE: State = State::new();
static mut PAINTED: State = State::new();
// ------------------------=
// FUNC: current
// DESC: Reads the UI-thread presentation snapshot.
// ------------------=
pub fn current() -> State {
    unsafe { *(&raw const STATE) }
}
// ------------------------=
// FUNC: publish
// DESC: Updates widget presentation without service calls or allocations.
// ------------------=
pub fn publish(state: State) {
    unsafe {
        *(&raw mut STATE) = state;
    }
}
// ------------------------=
// FUNC: take_damage
// DESC: Reports only changed widgets and chooser coverage, leaving ordinary pointer movement damage-free.
// ------------------=
pub fn take_damage(width: usize, height: usize, scale: usize) -> Option<Rect> {
    let old = unsafe { *(&raw const PAINTED) };
    let new = current();
    unsafe {
        *(&raw mut PAINTED) = new;
    }
    let mut damage: Option<Rect> = None;
    for id in 0..2 {
        if old.positions[id] != new.positions[id] || (old.visible ^ new.visible) & (1 << id) != 0 {
            for state in [old, new] {
                if state.visible & (1 << id) != 0 {
                    let rect = state.rect(id, width, height, scale, false);
                    damage = Some(damage.map_or(rect, |d| d.union(rect)));
                }
            }
        }
    }
    if old.menu != new.menu {
        for state in [old, new] {
            if let Some(rect) = state.menu_rect(width, height, scale) {
                damage = Some(damage.map_or(rect, |d| d.union(rect)));
            }
        }
    }
    damage.map(|mut rect| {
        rect.width = rect.width.saturating_add(12);
        rect.height = rect.height.saturating_add(12);
        rect.intersection(Rect {
            x: 0,
            y: 0,
            width: width as u32,
            height: height as u32,
        })
    })
}
