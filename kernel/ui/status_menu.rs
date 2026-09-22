//! Shared status-menu actions and a Gregorian calendar model.
use core::sync::atomic::{AtomicU32, Ordering};
static TODAY: AtomicU32 = AtomicU32::new(0);
static MONTH: AtomicU32 = AtomicU32::new(0);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Settings(usize),
    Devices(usize),
    Launcher,
    Files,
    Lock,
    Restart,
    Shutdown,
    PreviousMonth,
    Today,
    NextMonth,
    ConfirmRestart,
    ConfirmShutdown,
    Cancel,
    SnapLeft,
    SnapRight,
    CenterWindow,
    NextWindow,
    GrowWindow,
    ShrinkWindow,
}
// ------------------------=
// FUNC: items
// DESC: Returns dedicated status options whose rows map to real shell actions.
// ------------------=
pub fn items(menu: usize) -> &'static [(&'static [u8], Action)] {
    match menu {
        8 => &[
            (b"Audio output devices", Action::Devices(4)),
            (b"Audio input devices", Action::Devices(3)),
            (b"Voice & microphone permissions", Action::Settings(3)),
        ],
        9 => &[
            (b"Network configuration", Action::Settings(6)),
            (b"Nodes & mesh", Action::Settings(7)),
        ],
        10 => &[
            (b"Device management", Action::Settings(5)),
            (b"Mouse & keyboard settings", Action::Settings(10)),
        ],
        11 => &[
            (b"Lock screen", Action::Lock),
            (b"Restart", Action::Restart),
            (b"Shut down", Action::Shutdown),
        ],
        12 => &[
            (b"System settings", Action::Settings(0)),
            (b"Input settings", Action::Settings(10)),
            (b"Themes & skins", Action::Settings(1)),
            (b"Tile window left", Action::SnapLeft),
            (b"Tile window right", Action::SnapRight),
            (b"Center window", Action::CenterWindow),
            (b"Next open app", Action::NextWindow),
            (b"Increase window size", Action::GrowWindow),
            (b"Decrease window size", Action::ShrinkWindow),
        ],
        13 => &[
            (b"Search apps", Action::Launcher),
            (b"Browse files", Action::Files),
        ],
        14 => &[
            (b"Users & accounts", Action::Settings(2)),
            (b"Lock screen", Action::Lock),
            (b"About InfinityOS", Action::Settings(9)),
        ],
        15 | 16 => &[
            (b"Previous month", Action::PreviousMonth),
            (b"Today", Action::Today),
            (b"Next month", Action::NextMonth),
            (b"System settings", Action::Settings(0)),
        ],
        17 => &[
            (b"Cancel", Action::Cancel),
            (b"Restart now (save work first)", Action::ConfirmRestart),
        ],
        18 => &[
            (b"Cancel", Action::Cancel),
            (b"Shut down now (save work first)", Action::ConfirmShutdown),
        ],
        _ => &[],
    }
}
// ------------------------=
// FUNC: adjacent
// DESC: Traverses only visible top-bar menus, wrapping at either edge and folding the calendar refresh identity.
// ------------------=
pub fn adjacent(menu: usize, forward: bool) -> usize {
    let order = [0, 5, 8, 9, 10, 11, 12, 13, 14, 15];
    let menu = if menu == 16 { 15 } else { menu };
    let index = order.iter().position(|value| *value == menu).unwrap_or(0);
    order[(index + if forward { 1 } else { order.len() - 1 }) % order.len()]
}
// ------------------------=
// FUNC: set_today
// DESC: Updates calendar date from the existing live firmware clock.
// ------------------=
pub fn set_today(year: u16, month: u8, day: u8) {
    if year >= 1900 && (1..=12).contains(&month) && day > 0 && day <= days(year, month) {
        TODAY.store(
            (year as u32) << 16 | (month as u32) << 8 | day as u32,
            Ordering::Relaxed,
        );
    }
}
// ------------------------=
// FUNC: navigate
// DESC: Opens today's month or moves one month across year boundaries.
// ------------------=
pub fn navigate(delta: i32) {
    let today = TODAY.load(Ordering::Relaxed);
    let base = if delta == 0 || MONTH.load(Ordering::Relaxed) == 0 {
        ((today >> 16).max(1900)) * 12 + ((today >> 8) & 255).clamp(1, 12) - 1
    } else {
        MONTH.load(Ordering::Relaxed)
    };
    MONTH.store(
        (base as i32 + delta).clamp(1900 * 12, 9999 * 12 + 11) as u32,
        Ordering::Relaxed,
    );
}
// ------------------------=
// FUNC: month
// DESC: Returns the current displayed month and optional highlighted current day.
// ------------------=
pub fn month() -> (u16, u8, u8) {
    if MONTH.load(Ordering::Relaxed) == 0 {
        navigate(0);
    }
    let view = MONTH.load(Ordering::Relaxed);
    let year = (view / 12) as u16;
    let month = (view % 12 + 1) as u8;
    let today = TODAY.load(Ordering::Relaxed);
    (
        year,
        month,
        if today >> 16 == year as u32 && (today >> 8) & 255 == month as u32 {
            today as u8
        } else {
            0
        },
    )
}
// ------------------------=
// FUNC: days
// DESC: Computes month length with Gregorian century and leap-year rules.
// ------------------=
pub fn days(year: u16, month: u8) -> u8 {
    match month {
        4 | 6 | 9 | 11 => 30,
        2 => {
            if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) {
                29
            } else {
                28
            }
        }
        _ => 31,
    }
}
// ------------------------=
// FUNC: weekday
// DESC: Returns Sunday-based weekday for a valid Gregorian date.
// ------------------=
pub fn weekday(year: u16, month: u8, day: u8) -> usize {
    let y = year as usize - if month < 3 { 1 } else { 0 };
    (y + y / 4 - y / 100
        + y / 400
        + [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4][month as usize - 1]
        + day as usize)
        % 7
}
