//! UI-thread snapshot for the native minimized-window shelf. No service calls in paint.
use crate::ui::geometry::{Point, Rect};

pub const NAVIGATORS: usize = 6;
pub const COMMAND: usize = 6;
pub const EDITOR: usize = 7;
pub const TASKS: usize = 8;
pub const SETTINGS: usize = 9;
pub const BROWSER: usize = 10;
pub const NATIVE: usize = 11;
pub const COUNT: usize = NATIVE + crate::ui::window::MAX_WINDOWS;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Restore,
    Maximize,
    Close,
    ForceKill,
    Inspect,
}
pub const ACTIONS: [Action;5] = [Action::Restore,Action::Maximize,Action::Close,Action::ForceKill,Action::Inspect];
pub const ACTION_LABELS: [&[u8];5] = [b"Show",b"Maximize",b"Close",b"Force Kill",b"Show in Task Browser"];
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct State {
    pub left: bool,
    pub expanded: bool,
    pub drag: Option<(i32, i32, i32, i32)>,
    pub floating: [u16; 2],
    pub mask: u32,
    pub native: [Option<crate::ui::window::WindowId>; crate::ui::window::MAX_WINDOWS],
    pub offset: usize,
    pub menu: Option<usize>,
    pub hover: Option<usize>,
    pub row: Option<usize>,
}
impl State {
    // ------------------------=
    // FUNC: sync_native
    // DESC: Keeps tray identities stable across z-order changes and retires only restored or closed windows.
    // ------------------=
    pub fn sync_native(&mut self, windows: [Option<crate::ui::window::WindowId>; crate::ui::window::MAX_WINDOWS]) {
        for slot in &mut self.native {
            if !windows.contains(slot) { *slot=None; }
        }
        for id in windows.into_iter().flatten() {
            if !self.native.contains(&Some(id)) {
                if let Some(slot)=self.native.iter_mut().find(|slot|slot.is_none()) {*slot=Some(id);}
            }
        }
        for slot in 0..self.native.len() {self.set(NATIVE+slot,self.native[slot].is_some());}
    }
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
            expanded: false,
            drag: None,
            floating: [0; 2],
            mask: 0,
            native: [None; crate::ui::window::MAX_WINDOWS],
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
    // FUNC: visible
    // DESC: Empty shelves neither cover application content nor intercept input.
    // ------------------=
    pub fn visible(self) -> bool {
        self.mask != 0
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
    pub base: u32,
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
        let expansion = presentation().0 as u32;
        let extra = w * 2 * expansion / 255;
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
                        (width * 99 / 100) as i32 - (w + extra) as i32
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
            width: w + extra,
            height: h,
        };
        let tile = w + w / 4;
        let capacity = (h.saturating_sub(w) / tile.max(1)).max(1) as usize;
        let row_height = (height / 24).clamp(28, 52) as u32;
        let menu_width = (width / 7).clamp(280, 340) as u32;
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
                .min((height as i32 - (row_height * 6) as i32).max(0)),
            width: menu_width,
            height: row_height * 6,
        };
        Self {
            rail,
            tile,
            base: w,
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
            x: self.rail.x + (self.base / 10) as i32,
            y: self.rail.y + (self.base / 2 + index as u32 * self.tile) as i32,
            width: self.base.saturating_sub(self.base / 5),
            height: self.base,
        }
    }
    // ------------------------=
    // FUNC: hit
    // DESC: Hit tests only populated visible tiles.
    // ------------------=
    pub fn hit(self, state: State, point: Point) -> Option<usize> {
        if presentation().0>220 {return None;}
        (0..self.capacity)
            .find(|i| self.tile_rect(*i).contains(point))
            .and_then(|i| state.item(state.offset + i))
    }
    // ------------------------=
    // FUNC: action_rect
    // DESC: Places the selected app action rows beside the fixed icon rail.
    // ------------------=
    pub fn action_rect(self, index:usize) -> Rect {
        let height=(self.base*46/100).max(28);
        Rect {x:self.rail.x+(self.base/10)as i32,y:self.rail.y+self.base as i32+(index as u32*(height+8))as i32,width:self.rail.width.saturating_sub(self.base/10+self.base*62/100+6),height}
    }
    // ------------------------=
    // FUNC: action_hit
    // DESC: Resolves only fully exposed task actions; hidden controls cannot receive clicks.
    // ------------------=
    pub fn action_hit(self, point:Point)->Option<usize> {
        if presentation().0<=220 {return None;}
        (0..ACTIONS.len()).find(|i|self.action_rect(*i).contains(point))
    }
    // ------------------------=
    // FUNC: menu_row
    // DESC: Resolves shared task actions below the context menu title.
    // ------------------=
    pub fn menu_row(self, point: Point) -> Option<usize> {
        if !self.menu.contains(point) {
            return None;
        }
        let row = (point.y - self.menu.y) as u32 / self.row_height;
        (row > 0 && row <= ACTIONS.len() as u32).then_some(row.saturating_sub(1) as usize)
    }
    // ------------------------=
    // FUNC: animation_damage
    // DESC: Clears the entire expansion envelope on every motion frame, including the collapsed endpoint.
    // ------------------=
    pub fn animation_damage(self, state:State)->Rect {
        self.damage(state).union(Rect {x:if state.left || state.floating[0]>0 || state.drag.is_some() {self.rail.x-12}else{self.rail.right()-(self.base*4)as i32-12}, y:self.rail.y-12,width:self.base*4+24,height:self.rail.height+24})
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
        BROWSER => b"Browser",
        NATIVE..COUNT => b"Application",
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
        BROWSER => 60,
        NATIVE..COUNT => 49,
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
            let previous=Geometry::new(width,height,old);
            let next=Geometry::new(width,height,new);
            let old_damage=if old.expanded {previous.animation_damage(old)} else {previous.damage(old)};
            let new_damage=if new.expanded {next.animation_damage(new)} else {next.damage(new)};
            old_damage.union(new_damage)
        })
    }
}


static mut EXPANSION: crate::ui::app_launcher::motion::Motion = crate::ui::app_launcher::motion::Motion::settled(0);
static mut SELECTION: crate::ui::app_launcher::motion::Motion = crate::ui::app_launcher::motion::Motion::settled(0);
static mut SELECTION_TAIL: crate::ui::app_launcher::motion::Motion = crate::ui::app_launcher::motion::Motion::settled(0);
static mut HOVER: [crate::ui::app_launcher::motion::Motion; 5] = [crate::ui::app_launcher::motion::Motion::settled(0); 5];
static mut HOVER_TARGETS: [i32;5] = [0;5];
static mut HOVER_VALUES: [i32; 5] = [0; 5];
static mut TAIL_VALUE: i32 = 0;
static mut PRESENTATION: (i32, i32) = (0, 0);
static mut MOTION_TARGET: (i32, i32) = (0, 0);
// ------------------------=
// FUNC: presentation
// DESC: Reads prepared expansion and selection keyframes without changing animation during paint.
// ------------------=
pub fn presentation() -> (i32, i32) { unsafe { *(&raw const PRESENTATION) } }
// ------------------------=
// FUNC: bubble_state
// DESC: Supplies continuous hover growth and the trailing selection edge to the shared painter.
// ------------------=
pub fn bubble_state() -> ([i32;5],i32) { unsafe {(*(&raw const HOVER_VALUES),TAIL_VALUE)} }
// ------------------------=
// FUNC: circle_extent
// DESC: Computes a rounded scanline extent without floating point or allocation.
// ------------------=
pub fn circle_extent(radius:i32, offset:i32)->i32 {
    let n=(radius*radius-offset*offset).max(0);
    let (mut lo,mut hi)=(0,radius.max(0)+1);
    while hi-lo>1 {let mid=(lo+hi)/2;if mid*mid<=n {lo=mid;}else{hi=mid;}}
    lo
}
// ------------------------=
// FUNC: bubble_span
// DESC: Joins the rounded selection cap to its spine with concave tangent shoulders.
// ------------------=
pub fn bubble_span(y:i32,top:i32,bottom:i32,left:i32,spine:i32,shoulder:i32)->Option<(i32,i32)> {
    let radius=((bottom-top)/2).max(1);
    if y>=top && y<bottom {
        let dy=if y<top+radius {top+radius-y}else{y-(bottom-radius)};
        Some((left+radius-circle_extent(radius,dy.max(0)),spine+1))
    } else if y>=top-shoulder && y<top {
        let dy=y-(top-shoulder);
        Some((spine-shoulder+circle_extent(shoulder,dy),spine+1))
    } else if y>=bottom && y<bottom+shoulder {
        let dy=bottom+shoulder-1-y;
        Some((spine-shoulder+circle_extent(shoulder,dy),spine+1))
    } else {None}
}
// ------------------------=
// FUNC: animate
// DESC: Retargets the expanding rail and sliding selection from current positions; reduced motion settles immediately.
// ------------------=
pub fn animate(now: u64, reduced: bool) -> bool {
    let s = current();
    let row = s.row.map(|row|row as i32*256);
    unsafe {
        let target = (if s.visible() && s.drag.is_none() && (s.expanded || s.menu.is_some()) {255} else {0}, row.unwrap_or(MOTION_TARGET.1));
        if target.0 != MOTION_TARGET.0 || reduced { (*(&raw mut EXPANSION)).retarget(target.0, now, 240, reduced); }
        if target.1 != MOTION_TARGET.1 || reduced { (*(&raw mut SELECTION)).retarget(target.1, now, 220, reduced); (*(&raw mut SELECTION_TAIL)).retarget(target.1,now,320,reduced); }
        MOTION_TARGET = target;
        let next = ((*(&raw const EXPANSION)).value(now), (*(&raw const SELECTION)).value(now));
        let mut changed = next != PRESENTATION;
        let tail=(*(&raw const SELECTION_TAIL)).value(now);
        changed |= tail!=TAIL_VALUE; TAIL_VALUE=tail;
        for i in 0..5 {
            let destination=if s.expanded && s.row==Some(i) {255}else{0};
            if (*(&raw const HOVER_TARGETS))[i]!=destination || reduced {
                (*(&raw mut HOVER))[i].retarget(destination,now,220,reduced);
                (*(&raw mut HOVER_TARGETS))[i]=destination;
            }
            let value=(*(&raw const HOVER))[i].value(now);
            changed |= value!=(*(&raw const HOVER_VALUES))[i];
            (*(&raw mut HOVER_VALUES))[i]=value;
        }
        PRESENTATION = next;
        if changed { crate::ui::widget_keyframes::invalidate(3); }
        changed
    }
}
