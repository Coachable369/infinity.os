//! Unified pointer, keyboard, text, composition, focus, and accessibility input.

use super::geometry::Point;

pub const MAX_FOCUSABLE: usize = 64;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ElementId(pub u32);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Key {
    Tab,
    Enter,
    Escape,
    Space,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    Backspace,
    Delete,
    Character(char),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PointerPhase {
    Move,
    Down,
    Up,
    Scroll,
    Cancel,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum UiEvent<'a> {
    Pointer { position: Point, phase: PointerPhase, button: u8 },
    Key { key: Key, pressed: bool, modifiers: u8 },
    Text(&'a [u8]),
    CompositionStart,
    CompositionUpdate(&'a [u8]),
    CompositionCommit(&'a [u8]),
    CompositionCancel,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FocusError {
    Full,
    Unknown,
}

pub struct FocusManager {
    order: [Option<ElementId>; MAX_FOCUSABLE],
    count: u8,
    current: Option<ElementId>,
    trapped_root: Option<ElementId>,
}

impl FocusManager {
    // ------------------------=
    // FUNC: new
    // DESC: Creates an empty deterministic focus order for one UI scene.
    // ------------------=
    pub const fn new() -> Self {
        Self { order: [None; MAX_FOCUSABLE], count: 0, current: None, trapped_root: None }
    }

    // ------------------------=
    // FUNC: register
    // DESC: Adds one semantic focus target in document order without duplication.
    // ------------------=
    pub fn register(&mut self, id: ElementId) -> Result<(), FocusError> {
        if self.order[..self.count as usize].contains(&Some(id)) {
            return Ok(());
        }
        if self.count as usize >= MAX_FOCUSABLE {
            return Err(FocusError::Full);
        }
        self.order[self.count as usize] = Some(id);
        self.count += 1;
        if self.current.is_none() {
            self.current = Some(id);
        }
        Ok(())
    }

    // ------------------------=
    // FUNC: focus
    // DESC: Moves keyboard focus to a registered semantic element.
    // ------------------=
    pub fn focus(&mut self, id: ElementId) -> Result<(), FocusError> {
        if !self.order[..self.count as usize].contains(&Some(id)) {
            return Err(FocusError::Unknown);
        }
        self.current = Some(id);
        Ok(())
    }

    // ------------------------=
    // FUNC: move_next
    // DESC: Advances or reverses focus with wrapping for Tab and Shift-Tab navigation.
    // ------------------=
    pub fn move_next(&mut self, reverse: bool) -> Option<ElementId> {
        if self.count == 0 {
            self.current = None;
            return None;
        }
        let current_index = self.order[..self.count as usize]
            .iter()
            .position(|entry| *entry == self.current)
            .unwrap_or(0);
        let next = if reverse {
            if current_index == 0 { self.count as usize - 1 } else { current_index - 1 }
        } else {
            (current_index + 1) % self.count as usize
        };
        self.current = self.order[next];
        self.current
    }

    // ------------------------=
    // FUNC: current
    // DESC: Returns the active keyboard focus target.
    // ------------------=
    pub const fn current(&self) -> Option<ElementId> {
        self.current
    }

    // ------------------------=
    // FUNC: trap
    // DESC: Records a modal semantic root so assistive navigation cannot escape it.
    // ------------------=
    pub fn trap(&mut self, root: Option<ElementId>) {
        self.trapped_root = root;
    }

    // ------------------------=
    // FUNC: trapped_root
    // DESC: Returns the modal focus boundary currently enforced by the scene.
    // ------------------=
    pub const fn trapped_root(&self) -> Option<ElementId> {
        self.trapped_root
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PointerAccelerator {
    remainder_x: i32,
    remainder_y: i32,
}

impl PointerAccelerator {
    // ------------------------=
    // FUNC: new
    // DESC: Creates deterministic subpixel pointer acceleration state.
    // ------------------=
    pub const fn new() -> Self {
        Self { remainder_x: 0, remainder_y: 0 }
    }

    // ------------------------=
    // FUNC: apply
    // DESC: Preserves fine motion while accelerating deliberate larger gestures.
    // ------------------=
    pub fn apply(&mut self, dx: i32, dy: i32) -> Point {
        let magnitude = dx.abs().max(dy.abs());
        let gain = if magnitude <= 2 { 100 } else if magnitude <= 7 { 135 } else { 185 };
        let scaled_x = dx.saturating_mul(gain).saturating_add(self.remainder_x);
        let scaled_y = dy.saturating_mul(gain).saturating_add(self.remainder_y);
        self.remainder_x = scaled_x % 100;
        self.remainder_y = scaled_y % 100;
        Point { x: scaled_x / 100, y: scaled_y / 100 }
    }
}
