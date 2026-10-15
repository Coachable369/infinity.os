//! Retained presentation state and geometry for the desktop refinement kit.

use core::sync::atomic::{AtomicUsize, Ordering};

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
    let top_height = (38 * scale).min(height / 14).max(34 * scale);
    let brand_width = (280 * scale).min(width / 3);
    if (Rect {
        x: (8 * scale) as i32,
        y: (4 * scale) as i32,
        width: brand_width as u32,
        height: top_height.saturating_sub(8 * scale) as u32,
    })
    .contains(point)
    {
        return HoverTarget::Brand;
    }
    if (Rect {
        x: (610 * scale) as i32,
        y: (4 * scale) as i32,
        width: (62 * scale) as u32,
        height: top_height.saturating_sub(8 * scale) as u32,
    })
    .contains(point)
    {
        return HoverTarget::Help;
    }
    let status_width = 32 * scale;
    let clock_width = 104 * scale;
    let status_left = width.saturating_sub(7 * status_width + clock_width + 10 * scale);
    for index in 0..7 {
        if (Rect {
            x: (status_left + index * status_width) as i32,
            y: (4 * scale) as i32,
            width: status_width as u32,
            height: top_height.saturating_sub(8 * scale) as u32,
        })
        .contains(point)
        {
            return HoverTarget::Status(index);
        }
    }
    if (Rect {
        x: width.saturating_sub(clock_width + 8 * scale) as i32,
        y: (4 * scale) as i32,
        width: clock_width as u32,
        height: top_height.saturating_sub(8 * scale) as u32,
    })
    .contains(point)
    {
        return HoverTarget::Clock;
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
    HOVER.swap(next, Ordering::Relaxed) != next
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
pub const fn dock_running(
    index: usize,
    launcher_open: bool,
    visible: [bool; 6],
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
