//! UI-thread snapshot for the native minimized-window shelf. No service calls in paint.
use crate::ui::geometry::{Point, Rect};

pub const NAVIGATORS: usize = 6;
pub const COMMAND: usize = 6;
pub const EDITOR: usize = 7;
pub const TASKS: usize = 8;
pub const SETTINGS: usize = 9;
pub const COUNT: usize = 10;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Restore,
    Maximize,
    Close,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct State {
    pub left: bool,
    pub drag: Option<(i32, i32, i32, i32)>,
    pub floating: [u16; 2],
    pub mask: u16,
    pub offset: usize,
    pub menu: Option<usize>,
    pub hover: Option<usize>,
    pub row: Option<usize>,
}
impl State {
    // ------------------------=
    // FUNC: move_drag
    // DESC: Keeps the drawer floating on interior drops and anchors only inside the edge docking zones.
    // ------------------=
    pub fn move_drag(&mut self, pointer_x: i32, pointer_y: i32, held: bool, rail_width: i32) {
        let Some((_, _, grab_x, grab_y)) = self.drag else {
            return;
        };
        let x = (pointer_x - grab_x).clamp(0, 1000 - rail_width);
        let y = (pointer_y - grab_y).clamp(60, 340);
        if held {
            self.drag = Some((x, y, grab_x, grab_y));
        } else {
            self.left = x + rail_width / 2 < 500;
            self.floating = if x <= 35 || x + rail_width >= 965 {
                [0; 2]
            } else {
                [(x + 1) as u16, (y + 1) as u16]
            };
            self.drag = None;
        }
    }
    // ------------------------=
    // FUNC: new
    // DESC: Starts with no minimized windows or transient menu.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            left: true,
            drag: None,
            floating: [0; 2],
            mask: 0,
            offset: 0,
            menu: None,
            hover: None,
            row: None,
        }
    }
    // ------------------------=
    // FUNC: set
    // DESC: Changes one stable window identity without treating closed windows as minimized.
    // ------------------=
    pub fn set(&mut self, id: usize, minimized: bool) {
        if id >= COUNT {
            return;
        }
        if minimized {
            self.mask |= 1 << id;
        } else {
            self.mask &= !(1 << id);
        }
        if self.menu == Some(id) && !minimized {
            self.menu = None;
            self.row = None;
        }
        if self.hover == Some(id) && !minimized {
            self.hover = None;
        }
        self.offset = self.offset.min(self.count().saturating_sub(1));
    }
    // ------------------------=
    // FUNC: count
    // DESC: Counts live minimized window identities.
    // ------------------=
    pub fn count(self) -> usize {
        self.mask.count_ones() as usize
    }
    // ------------------------=
    // FUNC: item
    // DESC: Resolves a display index to an exact window identity rather than a most-recent app.
    // ------------------=
    pub fn item(self, index: usize) -> Option<usize> {
        (0..COUNT)
            .filter(|id| self.mask & (1 << id) != 0)
            .nth(index)
    }
    // ------------------------=
    // FUNC: scroll
    // DESC: Pages overflowing windows without losing their stable identities.
    // ------------------=
    pub fn scroll(&mut self, delta: i32, capacity: usize) {
        self.offset = (self.offset as i32 + delta)
            .clamp(0, self.count().saturating_sub(capacity) as i32) as usize;
        self.menu = None;
        self.row = None;
        self.hover = None;
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Geometry {
    pub rail: Rect,
    pub tile: u32,
    pub capacity: usize,
    pub menu: Rect,
    pub row_height: u32,
}
impl Geometry {
    // ------------------------=
    // FUNC: new
    // DESC: Reserves a narrow vertical shelf above the dock and places menus to its left.
    // ------------------=
    pub fn new(width: usize, height: usize, state: State) -> Self {
        let w = (width * 6 / 100).max(72).min(width / 4) as u32;
        let h = (height * 58 / 100) as u32;
        let rail = Rect {
            x: state
                .drag
                .map(|(x, _, _, _)| x * width as i32 / 1000)
                .unwrap_or_else(|| {
                    if state.floating[0] > 0 {
                        (state.floating[0] as i32 - 1) * width as i32 / 1000
                    } else if state.left {
                        (width / 100) as i32
                    } else {
                        (width * 99 / 100) as i32 - w as i32
                    }
                }),
            y: state
                .drag
                .map(|(_, y, _, _)| y * height as i32 / 1000)
                .unwrap_or_else(|| {
                    if state.floating[0] > 0 {
                        (state.floating[1] as i32 - 1).max(0) * height as i32 / 1000
                    } else {
                        (height * 18 / 100) as i32
                    }
                }),
            width: w,
            height: h,
        };
        let tile = w + w / 4;
        let capacity = (h.saturating_sub(w) / tile.max(1)).max(1) as usize;
        let row_height = (height / 24).clamp(28, 52) as u32;
        let menu_width = (width / 7).clamp(150, 300) as u32;
        let index = state
            .menu
            .or(state.hover)
            .and_then(|id| (0..state.count()).find(|n| state.item(*n) == Some(id)))
            .unwrap_or(0)
            .saturating_sub(state.offset);
        let menu = Rect {
            x: if rail.x < width as i32 / 2 {
                (rail.right() + 12).min((width as i32 - menu_width as i32).max(0))
            } else {
                (rail.x - menu_width as i32 - 12).max(0)
            },
            y: (rail.y + (w / 2 + index as u32 * tile) as i32)
                .min((height as i32 - (row_height * 4) as i32).max(0)),
            width: menu_width,
            height: row_height * 4,
        };
        Self {
            rail,
            tile,
            capacity,
            menu,
            row_height,
        }
    }
    // ------------------------=
    // FUNC: tile_rect
    // DESC: Returns a tile hit region with even vertical gutters.
    // ------------------=
    pub fn tile_rect(self, index: usize) -> Rect {
        Rect {
            x: self.rail.x + (self.rail.width / 10) as i32,
            y: self.rail.y + (self.rail.width / 2 + index as u32 * self.tile) as i32,
            width: self.rail.width * 8 / 10,
            height: self.rail.width,
        }
    }
    // ------------------------=
    // FUNC: hit
    // DESC: Hit tests only populated visible tiles.
    // ------------------=
    pub fn hit(self, state: State, point: Point) -> Option<usize> {
        (0..self.capacity)
            .find(|i| self.tile_rect(*i).contains(point))
            .and_then(|i| state.item(state.offset + i))
    }
    // ------------------------=
    // FUNC: menu_row
    // DESC: Resolves the three real context actions below the menu title.
    // ------------------=
    pub fn menu_row(self, point: Point) -> Option<usize> {
        if !self.menu.contains(point) {
            return None;
        }
        let row = (point.y - self.menu.y) as u32 / self.row_height;
        (row > 0 && row <= 3).then_some(row.saturating_sub(1) as usize)
    }
    // ------------------------=
    // FUNC: damage
    // DESC: Bounds shelf/menu effects independently of the full desktop.
    // ------------------=
    pub fn damage(self, state: State) -> Rect {
        let r = if state.menu.is_some() || state.hover.is_some() {
            self.rail.union(self.menu)
        } else {
            self.rail
        };
        Rect {
            x: r.x.saturating_sub(12),
            y: r.y.saturating_sub(12),
            width: r.width + 24,
            height: r.height + 24,
        }
    }
}

// ------------------------=
// FUNC: label
// DESC: Provides concise labels for live native window types.
// ------------------=
pub fn label(id: usize) -> &'static [u8] {
    match id {
        COMMAND => b"Console",
        EDITOR => b"Editor",
        TASKS => b"Tasks",
        SETTINGS => b"Settings",
        _ => b"Files",
    }
}
// ------------------------=
// FUNC: icon
// DESC: Resolves the existing theme-aware icon role for a native application.
// ------------------=
pub fn icon(id: usize) -> usize {
    match id {
        COMMAND => 25,
        EDITOR => 49,
        TASKS => 19,
        SETTINGS => 26,
        _ => 2,
    }
}

static mut STATE: State = State::new();
static mut PAINTED: State = State::new();
// ------------------------=
// FUNC: current
// DESC: Copies the immutable UI-thread presentation snapshot.
// ------------------=
pub fn current() -> State {
    unsafe { *(&raw const STATE) }
}
// ------------------------=
// FUNC: publish
// DESC: Commits a UI-thread state change without allocating or accessing services.
// ------------------=
pub fn publish(state: State) {
    unsafe {
        *(&raw mut STATE) = state;
    }
}
// ------------------------=
// FUNC: set
// DESC: Records a native minimize, restore, or close transition.
// ------------------=
pub fn set(id: usize, minimized: bool) {
    let mut state = current();
    state.set(id, minimized);
    publish(state);
}
// ------------------------=
// FUNC: take_damage
// DESC: Returns old and new overlay coverage only when its state changed.
// ------------------=
pub fn take_damage(width: usize, height: usize) -> Option<Rect> {
    unsafe {
        let old = *(&raw const PAINTED);
        let new = current();
        *(&raw mut PAINTED) = new;
        (old != new).then(|| {
            Geometry::new(width, height, old)
                .damage(old)
                .union(Geometry::new(width, height, new).damage(new))
        })
    }
}
