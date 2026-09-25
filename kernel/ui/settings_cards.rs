//! Shared Settings card composition derived from the generated Settings kit.
use super::geometry::Rect;

#[derive(Clone, Copy, Debug)]
pub struct CardContent {
    pub icon: Rect,
    pub label: Rect,
    pub description: Rect,
    pub value: Rect,
}

// ------------------------=
// FUNC: content
// DESC: Keeps two-line card copy, icon well and trailing value in disjoint bounded columns.
// ------------------=
pub fn content(row: Rect, scale: usize) -> CardContent {
    let s = scale.max(1) as u32;
    let inset = 12 * s;
    let icon_size = (44 * s).min(row.height.saturating_sub(2 * inset));
    let icon = Rect { x: row.x + inset as i32,
        y: row.y + (row.height.saturating_sub(icon_size) / 2) as i32,
        width: icon_size, height: icon_size };
    let left = icon.x + icon.width as i32 + inset as i32;
    let available = row.width.saturating_sub((left - row.x) as u32 + 36 * s);
    let value_width = available * 38 / 100;
    let label_width = available.saturating_sub(value_width + inset);
    let line_height = 28.min(row.height / 2);
    let label = Rect { x: left, y: row.y + (row.height.saturating_sub(line_height * 2) / 2) as i32,
        width: label_width, height: line_height };
    CardContent { icon, label,
        description: Rect { y: label.y + line_height as i32, ..label },
        value: Rect { x: left + label_width as i32 + inset as i32,
            y: row.y + (row.height.saturating_sub(line_height) / 2) as i32,
            width: value_width, height: line_height } }
}

// ------------------------=
// FUNC: description
// DESC: Describes actual Settings operations without implying unsupported editors or services.
// ------------------=
pub fn description(section: usize, row: usize) -> &'static [u8] {
    let items: &[&[u8]] = match section {
        0 => &[b"Your device's network identity", b"Current system language", b"Current regional configuration", b"Installed system generation", b"System update availability"],
        1 => &[b"Window and control materials", b"Icons throughout your desktop", b"Primary interface accent", b"Secondary interface accent", b"Glass transparency", b"Background diffusion", b"Automatic display sizing", b"Your desktop background"],
        2 => &[b"Signed-in local identity", b"Authentication information", b"Current session details", b"Your private storage space", b"Local profile information"],
        3 => &[b"Local and remote routing policy", b"Show or hide desktop chat", b"Choose an installed local model", b"Remote processing restrictions", b"Microphone authorization", b"Voice activation availability", b"Installed model access"],
        4 => &[b"Authority is granted explicitly", b"Microphone capability status", b"Remote processing permission", b"Lock after a period of inactivity", b"Protected interaction boundary"],
        5 => &[b"Observed display availability", b"Keyboard input availability", b"Choose your pointer appearance", b"Observed recording availability", b"Observed playback availability"],
        8 => &[b"Refresh pool observations", b"Measured storage capacity", b"Inspect observed storage nodes", b"Choose an object to inspect", b"Inspect verified replica placement", b"One verified replica", b"Two verified replicas", b"Three verified replicas"],
        9 => &[b"Operating system information", b"Native execution platform", b"Current boot information", b"Identity schema information", b"Packaged appearance families"],
        10 => &[b"Adjust pointer movement", b"Adjust wheel scrolling", b"Choose wheel direction", b"Choose the primary mouse button", b"Adjust pointer acceleration", b"Delay before a key repeats", b"Speed of repeated keystrokes", b"Restore default input preferences"],
        _ => &[],
    };
    items.get(row).copied().unwrap_or(b"Inspect current configuration")
}
