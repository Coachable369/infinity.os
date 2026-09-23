//! Launcher copies and pointer capture; only the UI thread publishes this state.
use super::{LAUNCHER_APPS, LAUNCHER_NO_ITEM};

pub const STATE_BYTES: usize = 84;
const PATH_PREFIX: &[u8] = b"/home/default/.launcher-layout-";
// ------------------------=
// FUNC: user_path
// DESC: Derives a collision-free user-specific storage path without hand-counted slice lengths.
// ------------------=
pub fn user_path(user: [u8; 16]) -> [u8; PATH_PREFIX.len() + 32] {
    let mut path = [0; PATH_PREFIX.len() + 32];
    path[..PATH_PREFIX.len()].copy_from_slice(PATH_PREFIX);
    for (i, b) in user.iter().enumerate() {
        path[PATH_PREFIX.len() + i * 2] = b"0123456789abcdef"[(b >> 4) as usize];
        path[PATH_PREFIX.len() + i * 2 + 1] = b"0123456789abcdef"[(b & 15) as usize];
    }
    path
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct State {
    pub positions: [[u16; 2]; 15],
    pub drag: Option<(usize, i32, i32, bool)>,
    pub pointer: [i32; 2],
}
impl State {
    // ------------------------=
    // FUNC: new
    // DESC: Creates an empty shortcut collection without duplicating the app catalog.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            positions: [[0; 2]; 15],
            drag: None,
            pointer: [0; 2],
        }
    }
    // ------------------------=
    // FUNC: begin
    // DESC: Captures a catalog identity, independent of filtered launcher slot indices.
    // ------------------=
    pub fn begin(&mut self, id: usize, x: i32, y: i32, moved: bool) {
        if id < LAUNCHER_APPS.len() {
            self.drag = Some((id, x, y, moved));
            self.pointer = [x, y];
        }
    }
    // ------------------------=
    // FUNC: motion
    // DESC: Retains click slop while making an established drag follow both axes.
    // ------------------=
    pub fn motion(&mut self, x: i32, y: i32) {
        self.pointer = [x, y];
        if let Some((id, ox, oy, moved)) = self.drag {
            self.drag = Some((id, ox, oy, moved || (x - ox).abs() + (y - oy).abs() >= 8));
        }
    }
    // ------------------------=
    // FUNC: place
    // DESC: Places one desktop copy without removing its source launcher application.
    // ------------------=
    pub fn place(&mut self, id: usize, x: i32, y: i32) {
        if id < self.positions.len() {
            self.positions[id] = [x.clamp(35, 930) as u16, y.clamp(90, 840) as u16];
        }
    }
    // ------------------------=
    // FUNC: encode
    // DESC: Writes versioned app identities, normalized positions and launcher order with a checksum.
    // ------------------=
    pub fn encode(self, order: [u8; 15]) -> [u8; STATE_BYTES] {
        let mut out = [0; STATE_BYTES];
        out[..4].copy_from_slice(b"IAP1");
        out[4..19].copy_from_slice(&order);
        for (i, p) in self.positions.iter().enumerate() {
            out[20 + i * 4..22 + i * 4].copy_from_slice(&p[0].to_le_bytes());
            out[22 + i * 4..24 + i * 4].copy_from_slice(&p[1].to_le_bytes());
        }
        let sum = checksum(&out[..80]);
        out[80..].copy_from_slice(&sum.to_le_bytes());
        out
    }
    // ------------------------=
    // FUNC: decode
    // DESC: Rejects corrupt coordinates or non-permutation orders before publishing any state.
    // ------------------=
    pub fn decode(bytes: &[u8]) -> Option<(Self, [u8; 15])> {
        if bytes.len() != STATE_BYTES
            || &bytes[..4] != b"IAP1"
            || checksum(&bytes[..80]) != u32::from_le_bytes(bytes[80..84].try_into().ok()?)
        {
            return None;
        }
        let mut order = [0; 15];
        order.copy_from_slice(&bytes[4..19]);
        let mut seen = 0u16;
        for id in order {
            if id >= 15 || seen & (1 << id) != 0 {
                return None;
            }
            seen |= 1 << id;
        }
        let mut state = Self::new();
        for (i, p) in state.positions.iter_mut().enumerate() {
            p[0] = u16::from_le_bytes(bytes[20 + i * 4..22 + i * 4].try_into().ok()?);
            p[1] = u16::from_le_bytes(bytes[22 + i * 4..24 + i * 4].try_into().ok()?);
            if *p != [0, 0] && (!(35..=930).contains(&p[0]) || !(90..=840).contains(&p[1])) {
                return None;
            }
        }
        Some((state, order))
    }
}
// ------------------------=
// FUNC: checksum
// DESC: Detects damaged shortcut state without accepting a partial record.
// ------------------=
fn checksum(bytes: &[u8]) -> u32 {
    bytes
        .iter()
        .fold(2166136261u32, |h, b| (h ^ *b as u32).wrapping_mul(16777619))
}
static mut STATE: State = State::new();
static mut DIRTY: bool = false;
static mut PREVIOUS: State = State::new();
// ------------------------=
// FUNC: current
// DESC: Reads the UI-thread shortcut presentation snapshot.
// ------------------=
pub fn current() -> State {
    unsafe { STATE }
}
// ------------------------=
// FUNC: publish
// DESC: Retains old geometry for bounded desktop recomposition after a drag change.
// ------------------=
pub fn publish(state: State) {
    unsafe {
        if STATE != state {
            if !DIRTY {
                PREVIOUS = STATE;
            }
            STATE = state;
            DIRTY = true;
        }
    }
}
// ------------------------=
// FUNC: take_changed
// DESC: Returns old and new shortcut geometry once for the compositor.
// ------------------=
pub fn take_changed() -> Option<(State, State)> {
    unsafe {
        if !DIRTY {
            return None;
        }
        DIRTY = false;
        Some((PREVIOUS, STATE))
    }
}
// ------------------------=
// FUNC: icon_rect
// DESC: Shares pixel bounds for icon art, label, desktop hit testing and damage.
// ------------------=
pub fn icon_rect(
    position: [u16; 2],
    width: usize,
    height: usize,
) -> Option<(usize, usize, usize, usize)> {
    if position == [0, 0] {
        return None;
    }
    let size = (height / 21).max(44);
    Some((
        width * position[0] as usize / 1000,
        height * position[1] as usize / 1000,
        size + 120,
        size + 40,
    ))
}
// ------------------------=
// FUNC: take_damage
// DESC: Bounds old and new shortcut pixels so dragging does not invalidate the full framebuffer.
// ------------------=
pub fn take_damage(width: usize, height: usize) -> Option<(usize, usize, usize, usize)> {
    let (old, new) = take_changed()?;
    let mut bounds: Option<(usize, usize, usize, usize)> = None;
    for state in [old, new] {
        let mut positions = state.positions;
        if let Some((id, _, _, true)) = state.drag {
            positions[id] = [
                state.pointer[0].clamp(0, 1000) as u16,
                state.pointer[1].clamp(0, 1000) as u16,
            ];
        }
        for (id, p) in positions.iter().enumerate() {
            if old.positions[id] == new.positions[id]
                && old.drag.map(|d| d.0) != Some(id)
                && new.drag.map(|d| d.0) != Some(id)
            {
                continue;
            }
            if let Some((x, y, w, h)) = icon_rect(*p, width, height) {
                // Labels use the shared 16-pixel cell font, not the icon's
                // narrower hit region. Cover every catalog label and badge.
                let w = w.max(LAUNCHER_APPS[id].label.len() * 16 + 16);
                bounds = Some(match bounds {
                    None => (x, y, w, h),
                    Some((a, b, c, d)) => {
                        let l = a.min(x);
                        let t = b.min(y);
                        (l, t, (a + c).max(x + w) - l, (b + d).max(y + h) - t)
                    }
                });
            }
        }
    }
    bounds.map(|(x, y, w, h)| {
        (
            x.min(width),
            y.min(height),
            w.min(width.saturating_sub(x)),
            h.min(height.saturating_sub(y)),
        )
    })
}
// ------------------------=
// FUNC: cancel_launcher_drag
// DESC: Releases launcher capture without activating or reordering an app during a desktop transfer.
// ------------------=
pub fn cancel_launcher_drag() {
    super::LAUNCHER_DRAG_SOURCE.store(LAUNCHER_NO_ITEM, core::sync::atomic::Ordering::Relaxed);
    super::LAUNCHER_DRAG_TARGET.store(LAUNCHER_NO_ITEM, core::sync::atomic::Ordering::Relaxed);
    super::LAUNCHER_DRAG_MOVED.store(false, core::sync::atomic::Ordering::Relaxed);
}
