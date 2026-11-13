//! Retained presentation state and geometry for the desktop refinement kit.

use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use super::geometry::{Point, Rect};

pub const POLISH_COUNT: usize = 10;
pub const DOCK_ITEMS: usize = 8;

const NONE: usize = 0;
const BRAND: usize = 1;
const HELP: usize = 2;
const STATUS_BASE: usize = 3;
const CLOCK: usize = STATUS_BASE + 7;
const DOCK_BASE: usize = CLOCK + 1;

static HOVER: AtomicUsize = AtomicUsize::new(NONE);

static PIN_SLOT: AtomicUsize = AtomicUsize::new(0);
static PIN_LIGHT: AtomicUsize = AtomicUsize::new(0);
static PIN_START: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);
static DOCK_DIRTY: AtomicBool = AtomicBool::new(false);

// ------------------------=
// FUNC: illuminate_pin
// DESC: Starts one bounded light sweep on a successful app assignment.
// ------------------=
pub fn illuminate_pin(slot:usize, now:u64, reduced:bool) {
    PIN_SLOT.store(slot,Ordering::Relaxed);
    PIN_START.store(now,Ordering::Relaxed);
    PIN_LIGHT.store(if reduced {0} else {255},Ordering::Relaxed);
    DOCK_DIRTY.store(true,Ordering::Relaxed);
}

// ------------------------=
// FUNC: advance_pin
// DESC: Samples the drop sweep by elapsed time and stops repainting once it ends.
// ------------------=
pub fn advance_pin(now:u64)->bool {
    if PIN_LIGHT.load(Ordering::Relaxed)==0 {return false;}
    let elapsed=now.saturating_sub(PIN_START.load(Ordering::Relaxed)).min(320);
    let light=255-(elapsed*255/320) as usize;
    let changed=PIN_LIGHT.swap(light,Ordering::Relaxed)!=light;
    if changed { DOCK_DIRTY.store(true,Ordering::Relaxed); }
    changed
}

// ------------------------=
// FUNC: pin_light
// DESC: Supplies the current slot's confirmation intensity to the dock painter.
// ------------------=
pub fn pin_light(slot:usize)->usize {
    if slot==PIN_SLOT.load(Ordering::Relaxed) {PIN_LIGHT.load(Ordering::Relaxed)} else {0}
}

// ------------------------=
// FUNC: take_dock_damage
// DESC: Repaints the app bar and its caption on hover transitions and pin changes.
// ------------------=
pub fn take_dock_damage()->bool { DOCK_DIRTY.swap(false,Ordering::Relaxed) }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HoverTarget {
    None,
    Brand,
    Help,
    Status(usize),
    Clock,
    Dock(usize),
}

// ------------------------=
// FUNC: encode
// DESC: Converts a semantic hover target into one bounded atomic value.
// ------------------=
const fn encode(target: HoverTarget) -> usize {
    match target {
        HoverTarget::None => NONE,
        HoverTarget::Brand => BRAND,
        HoverTarget::Help => HELP,
        HoverTarget::Status(index) => STATUS_BASE + index,
        HoverTarget::Clock => CLOCK,
        HoverTarget::Dock(index) => DOCK_BASE + index,
    }
}

// ------------------------=
// FUNC: decode
// DESC: Restores a semantic hover target while rejecting out-of-range values.
// ------------------=
const fn decode(value: usize) -> HoverTarget {
    match value {
        NONE => HoverTarget::None,
        BRAND => HoverTarget::Brand,
        HELP => HoverTarget::Help,
        CLOCK => HoverTarget::Clock,
        value if value >= STATUS_BASE && value < CLOCK => HoverTarget::Status(value - STATUS_BASE),
        value if value >= DOCK_BASE && value < DOCK_BASE + DOCK_ITEMS => {
            HoverTarget::Dock(value - DOCK_BASE)
        }
        _ => HoverTarget::None,
    }
}

// ------------------------=
// FUNC: target
// DESC: Resolves only shell-chrome hover regions, leaving empty gutters and application content untouched.
// ------------------=
pub fn target(width: usize, height: usize, normalized_x: i32, normalized_y: i32) -> HoverTarget {
    let scale = if width >= 2560 && height >= 1440 {
        2
    } else {
        1
    };
    let point = Point {
        x: width as i32 * normalized_x.clamp(0, 1000) / 1000,
        y: height as i32 * normalized_y.clamp(0, 1000) / 1000,
    };
    if let Some(role)=super::desktop_template::navigation_hit(point,width,height) {
        return match role {49=>HoverTarget::Brand,50=>HoverTarget::Help,52=>HoverTarget::Clock,
            53..=59=>HoverTarget::Status(usize::from(role-53)),_=>HoverTarget::None};
    }
    if super::desktop_template::dock_scene().is_some() {
        return super::desktop_template::dock_hit(point,width,height)
            .map(HoverTarget::Dock).unwrap_or(HoverTarget::None);
    }
    let dock_width = width * 54 / 100;
    let dock_height = 72 * scale;
    let dock_left = width.saturating_sub(dock_width) / 2;
    let dock_top = height.saturating_sub(dock_height + 10 * scale);
    let gap = dock_width / DOCK_ITEMS;
    let icon_size = 46 * scale;
    for index in 0..DOCK_ITEMS {
        let icon_left = dock_left + gap / 2 + index * gap;
        if (Rect {
            x: icon_left.saturating_sub(5 * scale) as i32,
            y: (dock_top + 4 * scale) as i32,
            width: icon_size.saturating_add(10 * scale) as u32,
            height: icon_size.saturating_add(16 * scale) as u32,
        })
        .contains(point)
        {
            return HoverTarget::Dock(index);
        }
    }
    HoverTarget::None
}

// ------------------------=
// FUNC: publish
// DESC: Publishes the current shell hover and reports only semantic boundary changes.
// ------------------=
pub fn publish(width: usize, height: usize, normalized_x: i32, normalized_y: i32) -> bool {
    let next = encode(target(width, height, normalized_x, normalized_y));
    let previous=HOVER.swap(next, Ordering::Relaxed);
    if previous!=next && (previous>=DOCK_BASE || next>=DOCK_BASE) {DOCK_DIRTY.store(true,Ordering::Relaxed);}
    previous!=next
}

// ------------------------=
// FUNC: current
// DESC: Reads the retained shell hover without consulting input devices during painting.
// ------------------=
pub fn current() -> HoverTarget {
    decode(HOVER.load(Ordering::Relaxed))
}

// ------------------------=
// FUNC: dock_label
// DESC: Returns the concise design-kit label for one dock destination.
// ------------------=
pub const fn dock_label(index: usize) -> &'static [u8] {
    match index {
        0 => b"Applications",
        1 => b"Files",
        2 => b"Settings",
        3 => b"About",
        4 => b"AI & Voice",
        5 => b"Appearance",
        6 => b"Spatial Desktop",
        7 => b"Recycle Bin",
        _ => b"",
    }
}

// ------------------------=
// FUNC: dock_running
// DESC: Derives dock indicators from current visible surfaces instead of hard-coded icon positions.
// ------------------=
pub const fn dock_running<const N:usize>(
    index: usize,
    launcher_open: bool,
    visible: [bool; N],
    settings_section: usize,
    ai_widget_visible: bool,
) -> bool {
    match index {
        0 => launcher_open,
        1 => visible[0],
        2 => visible[4] && settings_section != 9 && settings_section != 1,
        3 => visible[4] && settings_section == 9,
        4 => ai_widget_visible || visible[4] && settings_section == 3,
        5 => visible[4] && settings_section == 1,
        _ => false,
    }
}
