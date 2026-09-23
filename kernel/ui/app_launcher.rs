//! Typed catalog and deterministic filtering for the installed native app launcher.

use core::sync::atomic::{AtomicBool, AtomicI32, AtomicU64, AtomicUsize, Ordering};

pub const LAUNCHER_COLUMNS: usize = 6;
pub const LAUNCHER_VISIBLE_ROWS: usize = 2;
pub const LAUNCHER_NO_ITEM: usize = usize::MAX;
const LAUNCHER_TRANSITION_STEP: usize = 32;

#[path = "motion.rs"]
pub mod motion;
#[path = "minimized_shelf.rs"]
pub mod minimized_shelf;
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
#[path = "app_shortcuts.rs"]
pub mod shortcuts;
const LAUNCHER_DRAG_THRESHOLD: i32 = 8;

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
static LAUNCHER_ORDER: AtomicU64 = AtomicU64::new(default_launcher_order());
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
static LAUNCHER_SCROLL: AtomicI32 = AtomicI32::new(0);
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
static LAUNCHER_SCROLL_TARGET: AtomicI32 = AtomicI32::new(0);
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
static LAUNCHER_TRANSITION: AtomicUsize = AtomicUsize::new(255);
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
static LAUNCHER_CLOSING: AtomicBool = AtomicBool::new(false);
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
static LAUNCHER_DRAG_SOURCE: AtomicUsize = AtomicUsize::new(LAUNCHER_NO_ITEM);
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
static LAUNCHER_DRAG_TARGET: AtomicUsize = AtomicUsize::new(LAUNCHER_NO_ITEM);
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
static LAUNCHER_DRAG_X: AtomicI32 = AtomicI32::new(0);
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
static LAUNCHER_DRAG_Y: AtomicI32 = AtomicI32::new(0);
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
static LAUNCHER_DRAG_ORIGIN_X: AtomicI32 = AtomicI32::new(0);
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
static LAUNCHER_DRAG_ORIGIN_Y: AtomicI32 = AtomicI32::new(0);
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
static LAUNCHER_DRAG_MOVED: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LauncherPresentation {
    pub order: [u8; LAUNCHER_APPS.len()],
    pub scroll: usize,
    pub transition: u8,
    pub closing: bool,
    pub drag_source: Option<usize>,
    pub drag_target: Option<usize>,
    pub drag_x: i32,
    pub drag_y: i32,
    pub drag_moved: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LauncherRelease {
    None,
    Activate(usize),
    Reordered,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LauncherTick {
    pub changed: bool,
    pub closed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LauncherAction {
    Home(usize),
    Settings(usize),
    TextEditor,
    CommandWindow,
    TaskManager,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LauncherEntry {
    pub label: &'static [u8],
    pub icon_role: usize,
    pub action: LauncherAction,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DockAction {
    Launcher,
    Files,
    Settings,
    About,
    AiVoice,
    Appearance,
    SpatialDesktop,
    Trash,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DockEntry {
    pub icon_kind: usize,
    pub action: DockAction,
}

pub const DESKTOP_DOCK_ENTRIES: [DockEntry; 8] = [
    DockEntry {
        icon_kind: usize::MAX,
        action: DockAction::Launcher,
    },
    DockEntry {
        icon_kind: 1,
        action: DockAction::Files,
    },
    DockEntry {
        icon_kind: 2,
        action: DockAction::Settings,
    },
    DockEntry {
        icon_kind: 3,
        action: DockAction::About,
    },
    DockEntry {
        icon_kind: 4,
        action: DockAction::AiVoice,
    },
    DockEntry {
        icon_kind: 5,
        action: DockAction::Appearance,
    },
    DockEntry {
        icon_kind: 8,
        action: DockAction::SpatialDesktop,
    },
    DockEntry {
        icon_kind: 7,
        action: DockAction::Trash,
    },
];

pub const LAUNCHER_APPS: [LauncherEntry; 15] = [
    LauncherEntry {
        label: b"File Navigator",
        icon_role: 4,
        action: LauncherAction::Home(0),
    },
    LauncherEntry {
        label: b"Documents",
        icon_role: 4,
        action: LauncherAction::Home(2),
    },
    LauncherEntry {
        label: b"Projects",
        icon_role: 9,
        action: LauncherAction::Home(7),
    },
    LauncherEntry {
        label: b"Media",
        icon_role: 8,
        action: LauncherAction::Home(6),
    },
    LauncherEntry {
        label: b"Recycle Bin",
        icon_role: 10,
        action: LauncherAction::Home(8),
    },
    LauncherEntry {
        label: b"Settings",
        icon_role: 26,
        action: LauncherAction::Settings(0),
    },
    LauncherEntry {
        label: b"Text Editor",
        icon_role: 49,
        action: LauncherAction::TextEditor,
    },
    LauncherEntry {
        label: b"AI & Voice",
        icon_role: 23,
        action: LauncherAction::Settings(3),
    },
    LauncherEntry {
        label: b"Security",
        icon_role: 32,
        action: LauncherAction::Settings(4),
    },
    LauncherEntry {
        label: b"Storage",
        icon_role: 12,
        action: LauncherAction::Settings(8),
    },
    LauncherEntry {
        label: b"About",
        icon_role: 28,
        action: LauncherAction::Settings(9),
    },
    LauncherEntry {
        label: b"Command Window",
        icon_role: 25,
        action: LauncherAction::CommandWindow,
    },
    LauncherEntry {
        label: b"Task Manager",
        icon_role: 19,
        action: LauncherAction::TaskManager,
    },
    LauncherEntry {
        label: b"Network",
        icon_role: 13,
        action: LauncherAction::Settings(6),
    },
    LauncherEntry {
        label: b"Nodes & Mesh",
        icon_role: 32,
        action: LauncherAction::Settings(7),
    },
];

pub const LAUNCHER_CATEGORIES: [LauncherEntry; 5] = [
    LauncherEntry {
        label: b"Home",
        icon_role: 0,
        action: LauncherAction::Home(0),
    },
    LauncherEntry {
        label: b"Work",
        icon_role: 9,
        action: LauncherAction::Home(7),
    },
    LauncherEntry {
        label: b"System",
        icon_role: 19,
        action: LauncherAction::Settings(0),
    },
    LauncherEntry {
        label: b"Utilities",
        icon_role: 25,
        action: LauncherAction::CommandWindow,
    },
    LauncherEntry {
        label: b"Personal Space",
        icon_role: 1,
        action: LauncherAction::Home(1),
    },
];

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
// ------------------------=
// FUNC: default_launcher_order
// DESC: Packs the deterministic fresh-install application order into four-bit slots.
// ------------------=
const fn default_launcher_order() -> u64 {
    let mut bits = 0u64;
    let mut index = 0usize;
    while index < LAUNCHER_APPS.len() {
        bits |= (index as u64) << (index * 4);
        index += 1;
    }
    bits
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
// ------------------------=
// FUNC: decoded_launcher_order
// DESC: Expands the atomic packed application order for rendering and hit testing.
// ------------------=
fn decoded_launcher_order() -> [u8; LAUNCHER_APPS.len()] {
    let bits = LAUNCHER_ORDER.load(Ordering::Relaxed);
    let mut order = [0u8; LAUNCHER_APPS.len()];
    let mut index = 0usize;
    while index < order.len() {
        order[index] = ((bits >> (index * 4)) & 0x0f) as u8;
        index += 1;
    }
    order
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
// ------------------------=
// FUNC: encode_launcher_order
// DESC: Packs and publishes a complete application order after a native drag operation.
// ------------------=
pub(crate) fn encode_launcher_order(order: &[u8; LAUNCHER_APPS.len()]) {
    let mut bits = 0u64;
    let mut index = 0usize;
    while index < order.len() {
        bits |= u64::from(order[index]) << (index * 4);
        index += 1;
    }
    LAUNCHER_ORDER.store(bits, Ordering::Relaxed);
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
// ------------------------=
// FUNC: launcher_open
// DESC: Starts the native launcher entrance transition and resets transient interaction state.
// ------------------=
pub fn launcher_open() {
    LAUNCHER_TRANSITION.store(LAUNCHER_TRANSITION_STEP, Ordering::Relaxed);
    LAUNCHER_CLOSING.store(false, Ordering::Relaxed);
    LAUNCHER_DRAG_SOURCE.store(LAUNCHER_NO_ITEM, Ordering::Relaxed);
    LAUNCHER_DRAG_TARGET.store(LAUNCHER_NO_ITEM, Ordering::Relaxed);
    LAUNCHER_DRAG_MOVED.store(false, Ordering::Relaxed);
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
// ------------------------=
// FUNC: launcher_begin_close
// DESC: Starts the native launcher exit transition without removing the surface prematurely.
// ------------------=
pub fn launcher_begin_close() {
    LAUNCHER_CLOSING.store(true, Ordering::Relaxed);
    LAUNCHER_DRAG_SOURCE.store(LAUNCHER_NO_ITEM, Ordering::Relaxed);
    LAUNCHER_DRAG_TARGET.store(LAUNCHER_NO_ITEM, Ordering::Relaxed);
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
// ------------------------=
// FUNC: launcher_scroll_by
// DESC: Moves the smooth-scroll destination while retaining the current eased viewport position.
// ------------------=
pub fn launcher_scroll_by(distance: i32, maximum: usize) -> bool {
    let previous = LAUNCHER_SCROLL_TARGET.load(Ordering::Relaxed);
    let next = previous
        .saturating_add(distance)
        .clamp(0, maximum.min(i32::MAX as usize) as i32);
    LAUNCHER_SCROLL_TARGET.store(next, Ordering::Relaxed);
    LAUNCHER_SCROLL.store(next, Ordering::Relaxed);
    previous != next
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
// ------------------------=
// FUNC: launcher_scroll_to
// DESC: Sets a bounded smooth-scroll destination for scrollbar dragging and focus reveal.
// ------------------=
pub fn launcher_scroll_to(offset: usize, maximum: usize) -> bool {
    let previous = LAUNCHER_SCROLL_TARGET.load(Ordering::Relaxed);
    let next = offset.min(maximum).min(i32::MAX as usize) as i32;
    LAUNCHER_SCROLL_TARGET.store(next, Ordering::Relaxed);
    LAUNCHER_SCROLL.store(next, Ordering::Relaxed);
    previous != next
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
// ------------------------=
// FUNC: launcher_begin_drag
// DESC: Captures one visible application tile for click-or-drag resolution on release.
// ------------------=
pub fn launcher_begin_drag(visible_index: usize, pointer_x: i32, pointer_y: i32) {
    LAUNCHER_DRAG_SOURCE.store(visible_index, Ordering::Relaxed);
    LAUNCHER_DRAG_TARGET.store(visible_index, Ordering::Relaxed);
    LAUNCHER_DRAG_X.store(pointer_x, Ordering::Relaxed);
    LAUNCHER_DRAG_Y.store(pointer_y, Ordering::Relaxed);
    LAUNCHER_DRAG_ORIGIN_X.store(pointer_x, Ordering::Relaxed);
    LAUNCHER_DRAG_ORIGIN_Y.store(pointer_y, Ordering::Relaxed);
    LAUNCHER_DRAG_MOVED.store(false, Ordering::Relaxed);
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
// ------------------------=
// FUNC: launcher_update_drag
// DESC: Updates the animated tile ghost and insertion target while pointer capture is held.
// ------------------=
pub fn launcher_update_drag(target: Option<usize>, pointer_x: i32, pointer_y: i32) -> bool {
    if LAUNCHER_DRAG_SOURCE.load(Ordering::Relaxed) == LAUNCHER_NO_ITEM {
        return false;
    }
    let prior_x = LAUNCHER_DRAG_X.swap(pointer_x, Ordering::Relaxed);
    let prior_y = LAUNCHER_DRAG_Y.swap(pointer_y, Ordering::Relaxed);
    let target = target.unwrap_or_else(|| LAUNCHER_DRAG_TARGET.load(Ordering::Relaxed));
    let prior_target = LAUNCHER_DRAG_TARGET.swap(target, Ordering::Relaxed);
    let distance = (pointer_x - LAUNCHER_DRAG_ORIGIN_X.load(Ordering::Relaxed)).abs()
        + (pointer_y - LAUNCHER_DRAG_ORIGIN_Y.load(Ordering::Relaxed)).abs();
    if distance >= LAUNCHER_DRAG_THRESHOLD {
        LAUNCHER_DRAG_MOVED.store(true, Ordering::Relaxed);
    }
    prior_x != pointer_x || prior_y != pointer_y || prior_target != target
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
// ------------------------=
// FUNC: launcher_finish_drag
// DESC: Activates a click or commits a stable insertion reorder when captured pointer input ends.
// ------------------=
pub fn launcher_finish_drag(query: &[u8]) -> LauncherRelease {
    let source = LAUNCHER_DRAG_SOURCE.swap(LAUNCHER_NO_ITEM, Ordering::Relaxed);
    let target = LAUNCHER_DRAG_TARGET.swap(LAUNCHER_NO_ITEM, Ordering::Relaxed);
    let moved = LAUNCHER_DRAG_MOVED.swap(false, Ordering::Relaxed);
    if source == LAUNCHER_NO_ITEM {
        return LauncherRelease::None;
    }
    if !moved {
        return LauncherRelease::Activate(source);
    }
    let Some(source_id) = launcher_visible_app_id(query, source) else {
        return LauncherRelease::None;
    };
    let Some(target_id) = launcher_visible_app_id(query, target) else {
        return LauncherRelease::None;
    };
    if source_id == target_id {
        return LauncherRelease::None;
    }
    let mut order = decoded_launcher_order();
    let Some(source_slot) = order.iter().position(|id| *id == source_id) else {
        return LauncherRelease::None;
    };
    let Some(target_slot) = order.iter().position(|id| *id == target_id) else {
        return LauncherRelease::None;
    };
    let moved_id = order[source_slot];
    if source_slot < target_slot {
        order.copy_within(source_slot + 1..=target_slot, source_slot);
    } else {
        order.copy_within(target_slot..source_slot, target_slot + 1);
    }
    order[target_slot] = moved_id;
    encode_launcher_order(&order);
    LauncherRelease::Reordered
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
// ------------------------=
// FUNC: launcher_animation_tick
// DESC: Advances entrance, exit, and eased scrolling by one display refresh interval.
// ------------------=
pub fn launcher_animation_tick(maximum_scroll: usize) -> LauncherTick {
    launcher_animation_advance(maximum_scroll, 16)
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
// ------------------------=
// FUNC: launcher_animation_advance
// DESC: Advances transitions by elapsed milliseconds so slow frames cannot stretch the animation duration.
// ------------------=
pub fn launcher_animation_advance(maximum_scroll: usize, elapsed_ms: usize) -> LauncherTick {
    let step = if elapsed_ms == 0 { 0 } else { (255 * elapsed_ms.min(160) / 160).max(1) };
    let closing = LAUNCHER_CLOSING.load(Ordering::Relaxed);
    let progress = LAUNCHER_TRANSITION.load(Ordering::Relaxed);
    let next_progress = if closing {
        progress.saturating_sub(step)
    } else {
        progress.saturating_add(step).min(255)
    };
    if next_progress != progress {
        LAUNCHER_TRANSITION.store(next_progress, Ordering::Relaxed);
    }
    let maximum = maximum_scroll.min(i32::MAX as usize) as i32;
    let target = LAUNCHER_SCROLL_TARGET
        .load(Ordering::Relaxed)
        .clamp(0, maximum);
    LAUNCHER_SCROLL_TARGET.store(target, Ordering::Relaxed);
    let current = LAUNCHER_SCROLL.load(Ordering::Relaxed).clamp(0, maximum);
    let difference = target - current;
    let next_scroll = if difference == 0 { current } else { target };
    if next_scroll != current {
        LAUNCHER_SCROLL.store(next_scroll, Ordering::Relaxed);
    }
    LauncherTick {
        changed: next_progress != progress || next_scroll != current,
        closed: closing && next_progress == 0,
    }
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
// ------------------------=
// FUNC: launcher_presentation
// DESC: Captures a coherent immutable launcher render snapshot from atomic interaction state.
// ------------------=
pub fn launcher_presentation() -> LauncherPresentation {
    let drag_source = LAUNCHER_DRAG_SOURCE.load(Ordering::Relaxed);
    let drag_target = LAUNCHER_DRAG_TARGET.load(Ordering::Relaxed);
    LauncherPresentation {
        order: decoded_launcher_order(),
        scroll: LAUNCHER_SCROLL.load(Ordering::Relaxed).max(0) as usize,
        transition: motion::ease_byte(LAUNCHER_TRANSITION.load(Ordering::Relaxed).min(255) as u8),
        closing: LAUNCHER_CLOSING.load(Ordering::Relaxed),
        drag_source: (drag_source != LAUNCHER_NO_ITEM).then_some(drag_source),
        drag_target: (drag_target != LAUNCHER_NO_ITEM).then_some(drag_target),
        drag_x: LAUNCHER_DRAG_X.load(Ordering::Relaxed),
        drag_y: LAUNCHER_DRAG_Y.load(Ordering::Relaxed),
        drag_moved: LAUNCHER_DRAG_MOVED.load(Ordering::Relaxed),
    }
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
// ------------------------=
// FUNC: launcher_state_hash
// DESC: Produces a compact change token for compositor invalidation during interaction animations.
// ------------------=
pub fn launcher_state_hash() -> u64 {
    let presentation = launcher_presentation();
    launcher_interaction_state_hash() ^ u64::from(presentation.transition).rotate_left(19)
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
// ------------------------=
// FUNC: launcher_interaction_state_hash
// DESC: Produces a launcher change token that excludes presentation progress for bounded animation damage.
// ------------------=
pub fn launcher_interaction_state_hash() -> u64 {
    let presentation = launcher_presentation();
    LAUNCHER_ORDER.load(Ordering::Relaxed)
        ^ (presentation.scroll as u64).rotate_left(7)
        ^ (presentation.drag_source.unwrap_or(0xff) as u64).rotate_left(27)
        ^ (presentation.drag_target.unwrap_or(0xff) as u64).rotate_left(35)
        ^ (presentation.drag_x as u32 as u64).rotate_left(43)
        ^ (presentation.drag_y as u32 as u64).rotate_left(51)
        ^ u64::from(presentation.closing).rotate_left(61)
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
// ------------------------=
// FUNC: launcher_display_slot
// DESC: Shifts neighboring tiles toward the live drag insertion gap for animated rearrangement.
// ------------------=
pub fn launcher_display_slot(visible_index: usize, presentation: LauncherPresentation) -> usize {
    let (Some(source), Some(target)) = (presentation.drag_source, presentation.drag_target) else {
        return visible_index;
    };
    if !presentation.drag_moved || source == target {
        return visible_index;
    }
    if source < target && (source + 1..=target).contains(&visible_index) {
        visible_index - 1
    } else if target < source && (target..source).contains(&visible_index) {
        visible_index + 1
    } else {
        visible_index
    }
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
// ------------------------=
// FUNC: launcher_visible_app_id
// DESC: Resolves a filtered visible slot into its stable application catalog identifier.
// ------------------=
pub(crate) fn launcher_visible_app_id(query: &[u8], visible_index: usize) -> Option<u8> {
    decoded_launcher_order()
        .iter()
        .copied()
        .filter(|id| launcher_query_matches(LAUNCHER_APPS[*id as usize].label, query))
        .nth(visible_index)
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
// ------------------------=
// FUNC: launcher_restore_default_order
// DESC: Restores the deterministic fresh-install order and clears transient launcher positioning.
// ------------------=
pub fn launcher_restore_default_order() {
    LAUNCHER_ORDER.store(default_launcher_order(), Ordering::Relaxed);
    LAUNCHER_SCROLL.store(0, Ordering::Relaxed);
    LAUNCHER_SCROLL_TARGET.store(0, Ordering::Relaxed);
    LAUNCHER_TRANSITION.store(255, Ordering::Relaxed);
    LAUNCHER_CLOSING.store(false, Ordering::Relaxed);
    LAUNCHER_DRAG_SOURCE.store(LAUNCHER_NO_ITEM, Ordering::Relaxed);
    LAUNCHER_DRAG_TARGET.store(LAUNCHER_NO_ITEM, Ordering::Relaxed);
    LAUNCHER_DRAG_MOVED.store(false, Ordering::Relaxed);
}

#[cfg(target_arch = "x86")]
// ------------------------=
// FUNC: launcher_open
// DESC: Keeps the legacy text-only x86 launcher entry point allocation free.
// ------------------=
pub fn launcher_open() {}

#[cfg(target_arch = "x86")]
// ------------------------=
// FUNC: launcher_begin_close
// DESC: Completes the unavailable graphical launcher immediately on legacy x86.
// ------------------=
pub fn launcher_begin_close() {}

#[cfg(target_arch = "x86")]
// ------------------------=
// FUNC: launcher_scroll_by
// DESC: Rejects graphical smooth scrolling on the legacy text-only x86 surface.
// ------------------=
pub fn launcher_scroll_by(_distance: i32, _maximum: usize) -> bool {
    false
}

#[cfg(target_arch = "x86")]
// ------------------------=
// FUNC: launcher_scroll_to
// DESC: Rejects graphical scrollbar positioning on the legacy text-only x86 surface.
// ------------------=
pub fn launcher_scroll_to(_offset: usize, _maximum: usize) -> bool {
    false
}

#[cfg(target_arch = "x86")]
// ------------------------=
// FUNC: launcher_begin_drag
// DESC: Ignores graphical tile capture on the legacy text-only x86 surface.
// ------------------=
pub fn launcher_begin_drag(_visible_index: usize, _pointer_x: i32, _pointer_y: i32) {}

#[cfg(target_arch = "x86")]
// ------------------------=
// FUNC: launcher_update_drag
// DESC: Rejects graphical tile movement on the legacy text-only x86 surface.
// ------------------=
pub fn launcher_update_drag(_target: Option<usize>, _pointer_x: i32, _pointer_y: i32) -> bool {
    false
}

#[cfg(target_arch = "x86")]
// ------------------------=
// FUNC: launcher_finish_drag
// DESC: Resolves no graphical tile release on the legacy text-only x86 surface.
// ------------------=
pub fn launcher_finish_drag(_query: &[u8]) -> LauncherRelease {
    LauncherRelease::None
}

#[cfg(target_arch = "x86")]
// ------------------------=
// FUNC: launcher_animation_tick
// DESC: Reports the unavailable graphical launcher as settled on legacy x86.
// ------------------=
pub fn launcher_animation_tick(_maximum_scroll: usize) -> LauncherTick {
    LauncherTick {
        changed: false,
        closed: false,
    }
}

#[cfg(target_arch = "x86")]
// ------------------------=
// FUNC: launcher_animation_advance
// DESC: Keeps elapsed-time animation calls available on legacy x86.
// ------------------=
pub fn launcher_animation_advance(maximum_scroll: usize, _elapsed_ms: usize) -> LauncherTick {
    launcher_animation_tick(maximum_scroll)
}

#[cfg(target_arch = "x86")]
// ------------------------=
// FUNC: launcher_presentation
// DESC: Returns a static presentation for shared legacy x86 layout compilation.
// ------------------=
pub fn launcher_presentation() -> LauncherPresentation {
    let mut order = [0u8; LAUNCHER_APPS.len()];
    let mut index = 0usize;
    while index < order.len() {
        order[index] = index as u8;
        index += 1;
    }
    LauncherPresentation {
        order,
        scroll: 0,
        transition: 255,
        closing: false,
        drag_source: None,
        drag_target: None,
        drag_x: 0,
        drag_y: 0,
        drag_moved: false,
    }
}

#[cfg(target_arch = "x86")]
// ------------------------=
// FUNC: launcher_state_hash
// DESC: Returns the stable legacy x86 presentation token.
// ------------------=
pub fn launcher_state_hash() -> u64 {
    0
}

#[cfg(target_arch = "x86")]
// ------------------------=
// FUNC: launcher_interaction_state_hash
// DESC: Returns the stable legacy x86 interaction token.
// ------------------=
pub fn launcher_interaction_state_hash() -> u64 {
    0
}

#[cfg(target_arch = "x86")]
// ------------------------=
// FUNC: launcher_display_slot
// DESC: Keeps catalog order unchanged on the legacy text-only x86 surface.
// ------------------=
pub fn launcher_display_slot(visible_index: usize, _presentation: LauncherPresentation) -> usize {
    visible_index
}

#[cfg(target_arch = "x86")]
// ------------------------=
// FUNC: launcher_visible_app_id
// DESC: Resolves filtered catalog entries without graphical reorder state on legacy x86.
// ------------------=
fn launcher_visible_app_id(query: &[u8], visible_index: usize) -> Option<u8> {
    LAUNCHER_APPS
        .iter()
        .enumerate()
        .filter(|(_, entry)| launcher_query_matches(entry.label, query))
        .nth(visible_index)
        .map(|(index, _)| index as u8)
}

#[cfg(target_arch = "x86")]
// ------------------------=
// FUNC: launcher_restore_default_order
// DESC: Retains the immutable catalog order on the legacy text-only x86 surface.
// ------------------=
pub fn launcher_restore_default_order() {}

// ------------------------=
// FUNC: launcher_query_matches
// DESC: Performs allocation-free ASCII case-insensitive substring matching for launcher search.
// ------------------=
pub fn launcher_query_matches(label: &[u8], query: &[u8]) -> bool {
    if query.is_empty() {
        return true;
    }
    if query.len() > label.len() {
        return false;
    }
    label.windows(query.len()).any(|window| {
        window
            .iter()
            .zip(query.iter())
            .all(|(left, right)| left.eq_ignore_ascii_case(right))
    })
}

// ------------------------=
// FUNC: launcher_visible_count
// DESC: Counts application results matching the live native launcher query.
// ------------------=
pub fn launcher_visible_count(query: &[u8]) -> usize {
    LAUNCHER_APPS
        .iter()
        .filter(|entry| launcher_query_matches(entry.label, query))
        .count()
}

// ------------------------=
// FUNC: launcher_visible_entry
// DESC: Resolves one compacted visible grid slot into its typed launcher entry.
// ------------------=
pub fn launcher_visible_entry(query: &[u8], visible_index: usize) -> Option<LauncherEntry> {
    launcher_visible_app_id(query, visible_index).map(|id| LAUNCHER_APPS[id as usize])
}
