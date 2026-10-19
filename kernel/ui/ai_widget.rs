//! Shared desktop assistant input metrics and transient menu state; no AI execution authority.
use super::geometry::{Point, Rect};
use core::sync::atomic::{AtomicU8, AtomicUsize, Ordering};

const METRICS: &[u8] = include_bytes!("../../assets/fonts/InfinityUI-Regular-16.metrics");
const KERN: &[u8] = include_bytes!("../../assets/fonts/InfinityUI-Regular-16.kern");
static MENU: AtomicU8 = AtomicU8::new(0);
static PROMPT_START: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum MenuAction { Voice = 0, Stop = 1, Settings = 2 }

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MenuState { pub open: bool, pub selected: usize }

impl MenuState {
    // ------------------------=
    // FUNC: move_selection
    // DESC: Moves keyboard focus through the three real status actions without leaving the menu.
    // ------------------=
    pub fn move_selection(&mut self, reverse: bool) {
        self.selected = if reverse { (self.selected + 2) % 3 } else { (self.selected + 1) % 3 };
    }

    // ------------------------=
    // FUNC: action
    // DESC: Resolves a menu row to its typed action, disabling Stop when no request is active.
    // ------------------=
    pub fn action(self, can_stop: bool) -> Option<MenuAction> {
        if !self.open { return None; }
        match self.selected {
            0 => Some(MenuAction::Voice),
            1 if can_stop => Some(MenuAction::Stop),
            2 => Some(MenuAction::Settings),
            _ => None,
        }
    }
}

// ------------------------=
// FUNC: current_menu
// DESC: Reads the nonpersistent status-menu presentation shared by input, painting and diagnostics.
// ------------------=
pub fn current_menu() -> MenuState {
    let value = MENU.load(Ordering::Relaxed);
    MenuState { open: value & 4 != 0, selected: (value & 3).min(2) as usize }
}

// ------------------------=
// FUNC: publish_menu
// DESC: Publishes a bounded menu selection without granting permissions or changing service state.
// ------------------=
pub fn publish_menu(menu: MenuState) {
    MENU.store((if menu.open { 4 } else { 0 }) | menu.selected.min(2) as u8, Ordering::Relaxed);
}

// ------------------------=
// FUNC: menu_hash
// DESC: Includes opening, dismissal and keyboard selection in the widget's retained damage key.
// ------------------=
pub fn menu_hash() -> u64 { MENU.load(Ordering::Relaxed) as u64 }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MenuGeometry { pub panel: Rect, pub rows: [Rect; 3] }

impl MenuGeometry {
    // ------------------------=
    // FUNC: new
    // DESC: Anchors the reusable compact glass menu below its control and clamps it to the screen.
    // ------------------=
    pub fn new(control: Rect, width: usize, height: usize, scale: usize) -> Self {
        let s = scale.max(1);
        let w = (204 * s).min(width.saturating_sub(16 * s));
        let h = super::app_menu::panel_height(3, false) * s;
        let x = (control.right() - w as i32).max((8 * s) as i32)
            .min(width.saturating_sub(w + 8 * s) as i32);
        let y = (control.bottom() + (6 * s) as i32)
            .min(height.saturating_sub(h + 8 * s) as i32).max(0);
        let panel = Rect { x, y, width: w as u32, height: h as u32 };
        let rows = core::array::from_fn(|i| Rect {
            x: x + (6 * s) as i32, y: y + ((6 + i * 28) * s) as i32,
            width: w.saturating_sub(12 * s) as u32, height: (28 * s) as u32,
        });
        Self { panel, rows }
    }

    // ------------------------=
    // FUNC: hit
    // DESC: Maps only real row interiors to actions; menu gutters are deliberately inert.
    // ------------------=
    pub fn hit(self, point: Point) -> Option<usize> {
        self.rows.iter().position(|row| row.contains(point))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextViewport { pub start: usize, pub end: usize, pub caret_x: usize }

// ------------------------=
// FUNC: prompt_viewport
// DESC: Retains the horizontal scroll origin while the caret remains visible, so clicking a word does not shift it away.
// ------------------=
pub fn prompt_viewport(text: &[u8], caret: usize, width: usize, scale: usize) -> TextViewport {
    let view=retained_viewport(text,caret,width,scale,PROMPT_START.load(Ordering::Relaxed));
    PROMPT_START.store(view.start,Ordering::Relaxed);
    view
}

// ------------------------=
// FUNC: retained_viewport
// DESC: Scrolls only when the insertion caret leaves the previous viewport or its contents shrink.
// ------------------=
pub fn retained_viewport(text: &[u8], caret: usize, width: usize, scale: usize, previous: usize) -> TextViewport {
    let caret=caret.min(text.len());
    let start=previous.min(text.len());
    let available=width.saturating_sub(2*scale.max(1));
    if caret<start || text_width(&text[start..caret],scale)>available {
        return text_viewport(text,caret,width,scale);
    }
    let caret_x=text_width(&text[start..caret],scale);
    let mut end=caret;
    let mut measured=caret_x;
    while end<text.len() {
        let next=measured.saturating_add_signed(advance(
            end.checked_sub(1).filter(|index|*index>=start).map(|index|text[index]),text[end],scale));
        if next>available {break;}
        measured=next;end+=1;
    }
    TextViewport{start,end,caret_x}
}

// ------------------------=
// FUNC: advance
// DESC: Uses the same authored sixteen-pixel glyph advances and pair kerning as the native app painter.
// ------------------=
fn advance(previous: Option<u8>, current: u8, scale: usize) -> isize {
    if !(32..=126).contains(&current) { return 0; }
    let index = (current - 32) as usize;
    let pair = previous.filter(|c| (32..=126).contains(c)).map_or(0, |left| {
        KERN[(left - 32) as usize * 95 + index] as isize - 128
    });
    (METRICS[index] as isize + pair) * scale.max(1) as isize
}

// ------------------------=
// FUNC: text_width
// DESC: Measures one visible prompt slice using the exact native painter's proportional advances.
// ------------------=
pub fn text_width(text: &[u8], scale: usize) -> usize {
    let mut width = 0usize;
    let mut previous = None;
    for &byte in text {
        width = width.saturating_add_signed(advance(previous, byte, scale));
        if (32..=126).contains(&byte) { previous = Some(byte); }
    }
    width
}

// ------------------------=
// FUNC: text_viewport
// DESC: Keeps the insertion caret visible and returns a bounded horizontal slice without truncating stored input.
// ------------------=
pub fn text_viewport(text: &[u8], caret: usize, width: usize, scale: usize) -> TextViewport {
    let caret = caret.min(text.len());
    let available = width.saturating_sub(2 * scale.max(1));
    let mut start = caret;
    let mut measured = 0usize;
    while start > 0 {
        let candidate = start - 1;
        let mut next = measured.saturating_add_signed(advance(None, text[candidate], scale));
        if start < caret && (32..=126).contains(&text[candidate]) && (32..=126).contains(&text[start]) {
            next = next.saturating_add_signed((KERN[(text[candidate] - 32) as usize * 95
                + (text[start] - 32) as usize] as isize - 128) * scale.max(1) as isize);
        }
        if next > available { break; }
        measured = next;
        start = candidate;
    }
    let mut end = caret;
    let mut end_width = measured;
    while end < text.len() {
        let next = end_width.saturating_add_signed(advance(
            end.checked_sub(1).filter(|index| *index >= start).map(|index| text[index]), text[end], scale));
        if next > available { break; }
        end_width = next;
        end += 1;
    }
    TextViewport { start, end, caret_x: measured.min(available) }
}

// ------------------------=
// FUNC: caret_at
// DESC: Resolves a pointer to the nearest proportional insertion boundary in the currently visible slice.
// ------------------=
pub fn caret_at(text: &[u8], view: TextViewport, offset: i32, scale: usize) -> usize {
    let offset = offset.max(0) as usize;
    let mut position = 0usize;
    let mut previous = None;
    for index in view.start..view.end.min(text.len()) {
        let next = position.saturating_add_signed(advance(previous, text[index], scale));
        if offset < position + next.saturating_sub(position) / 2 { return index; }
        position = next;
        previous = Some(text[index]);
    }
    view.end.min(text.len())
}
