//! Shared pre-session visual tokens and antialiased surface coverage.
pub type Rgb = (u8, u8, u8);
pub const BACKGROUND: Rgb = (9, 15, 38);
pub const SURFACE: Rgb = (16, 27, 58);
pub const ELEVATED: Rgb = (26, 46, 99);
pub const PRIMARY: Rgb = (62, 111, 255);
pub const BORDER: Rgb = (134, 199, 255);
pub const MUTED_BORDER: Rgb = (67, 88, 140);
pub const FOCUS: Rgb = (255, 255, 255);
pub const TEXT: Rgb = (255, 255, 255);
pub const SECONDARY: Rgb = (190, 190, 190);
pub const ERROR: Rgb = (255, 118, 145);

// ------------------------=
// FUNC: semantic_color
// DESC: Supplies Studio-authored native palette colors to dynamic configuration content not represented by static text layers.
// ------------------=
pub fn semantic_color(index:usize)->Rgb {
    let colors=crate::ui::skin::kit_tokens::runtime_colors();
    let color=colors[index.min(colors.len()-1)];
    (((color>>16)&255)as u8,((color>>8)&255)as u8,(color&255)as u8)
}

#[derive(Clone, Copy)]
pub struct FocusMotion { pub key: u32, pub started_ms: u64, pub amount: u8, pub pressed: bool }
impl FocusMotion {
    // ------------------------=
    // FUNC: new
    // DESC: Starts the pre-session focus animation in its settled state.
    // ------------------=
    pub const fn new() -> Self { Self { key: u32::MAX, started_ms: 0, amount: 255, pressed: false } }
    // ------------------------=
    // FUNC: advance
    // DESC: Eases a changed control focus over 140ms and settles without an idle animation loop.
    // ------------------=
    pub fn advance(&mut self, key: u32, now_ms: u64, pressed: bool, reduced: bool) -> bool {
        let before = (self.key, self.amount, self.pressed);
        if self.key != key { self.key = key; self.started_ms = now_ms; }
        let t = (now_ms.saturating_sub(self.started_ms).min(140) * 255 / 140) as u32;
        self.amount = if reduced { 255 } else { (255 - (255-t)*(255-t)/255) as u8 };
        self.pressed = pressed;
        before != (self.key, self.amount, self.pressed)
    }
}
static mut MOTION: FocusMotion = FocusMotion::new();
static mut DIRTY: bool = false;
static mut PANEL: PanelMotion = PanelMotion::new();

#[derive(Clone, Copy)]
pub struct PanelMotion { pub scene: u32, pub started_ms: u64, pub extent: u16, pub glow: u8 }
impl PanelMotion {
    // ------------------------=
    // FUNC: new
    // DESC: Initializes an inactive, fully visible panel until its first scene tick.
    // ------------------=
    pub const fn new() -> Self { Self { scene: u32::MAX, started_ms: 0, extent: 1024, glow: 24 } }
    // ------------------------=
    // FUNC: advance
    // DESC: Advances entrance and subtle breathing from elapsed time, independently of input events.
    // ------------------=
    pub fn advance(&mut self, scene: u32, now_ms: u64, reduced: bool) -> bool {
        let before = (self.scene, self.extent, self.glow);
        if self.scene != scene { self.scene = scene; self.started_ms = now_ms; }
        let elapsed = now_ms.saturating_sub(self.started_ms);
        let t = elapsed.min(360) * 1024 / 360;
        self.extent = if reduced { 1024 } else { (1024 - (1024-t).pow(3)/1024/1024) as u16 };
        let phase = elapsed.saturating_sub(360) % 3200;
        let triangle = if phase < 1600 { phase } else { 3200-phase };
        let t = triangle * 1024 / 1600;
        let smooth = t*t*(3072-2*t)/1024/1024;
        self.glow = if reduced { 24 } else { (24 + 20*smooth/1024) as u8 };
        before != (self.scene, self.extent, self.glow)
    }
    // ------------------------=
    // FUNC: rectangle
    // DESC: Returns a centered aperture without changing authored layout or hit targets.
    // ------------------=
    pub fn rectangle(&self, rect: (usize, usize, usize, usize)) -> (usize, usize, usize, usize) {
        let (x,y,w,h)=rect;
        let width=w*self.extent as usize/1024;
        let height=h*self.extent as usize/1024;
        (x+(w-width)/2,y+(h-height)/2,width,height)
    }
}

// ------------------------=
// FUNC: panel_motion
// DESC: Copies UI-thread-owned presentation state for a bounded panel painter.
// ------------------=
pub fn panel_motion() -> PanelMotion { unsafe { *(&raw const PANEL) } }

// ------------------------=
// FUNC: leave_scene
// DESC: Rearms entrance when returning from the desktop to a locked session.
// ------------------=
pub fn leave_scene() { unsafe { PANEL = PanelMotion::new(); } }

// ------------------------=
// FUNC: animate
// DESC: Advances UI-thread-owned access focus state without changing input or authentication state.
// ------------------=
pub fn animate(key: u32, now_ms: u64, pressed: bool, reduced: bool) -> bool {
    unsafe {
        let focus_changed = (&mut *(&raw mut MOTION)).advance(key, now_ms / 16 * 16, pressed, reduced);
        let panel_changed = (&mut *(&raw mut PANEL)).advance(key >> 8, now_ms / 16 * 16, reduced);
        let changed = focus_changed || panel_changed;
        DIRTY |= changed;
        changed
    }
}

// ------------------------=
// FUNC: take_dirty
// DESC: Consumes one bounded access-control repaint request on the UI thread.
// ------------------=
pub fn take_dirty() -> bool { unsafe { let changed=DIRTY; DIRTY=false; changed } }

// ------------------------=
// FUNC: focus_amount
// DESC: Returns the current bounded ease value for the active pre-session control.
// ------------------=
pub fn focus_amount() -> u8 { unsafe { (*(&raw const MOTION)).amount } }

// ------------------------=
// FUNC: pressed
// DESC: Exposes presentation-only pressed state to access buttons without triggering actions.
// ------------------=
pub fn pressed() -> bool { unsafe { (*(&raw const MOTION)).pressed } }

// ------------------------=
// FUNC: rounded_coverage
// DESC: Returns four-by-four subpixel coverage only at rounded corners, keeping straight spans inexpensive.
// ------------------=
pub fn rounded_coverage(x: usize, y: usize, width: usize, height: usize, radius: usize) -> u8 {
    if x >= width || y >= height { return 0; }
    let radius = radius.min(width / 2).min(height / 2);
    if radius == 0 || (x >= radius && x < width - radius)
        || (y >= radius && y < height - radius) { return 255; }
    let cx = if x < radius { radius * 8 } else { (width - radius) * 8 } as i64;
    let cy = if y < radius { radius * 8 } else { (height - radius) * 8 } as i64;
    let squared = (radius * 8).pow(2) as i64;
    let mut covered = 0u16;
    for sy in [1, 3, 5, 7] {
        for sx in [1, 3, 5, 7] {
            let dx = (x * 8 + sx) as i64 - cx;
            let dy = (y * 8 + sy) as i64 - cy;
            covered += u16::from(dx * dx + dy * dy <= squared);
        }
    }
    ((covered * 255 + 8) / 16) as u8
}

// ------------------------=
// FUNC: mix
// DESC: Interpolates two authored palette tokens without floating-point paint work.
// ------------------=
pub fn mix(a: Rgb, b: Rgb, amount: u8) -> Rgb {
    let t = amount as u16;
    (((a.0 as u16 * (255-t) + b.0 as u16*t)/255) as u8,
     ((a.1 as u16 * (255-t) + b.1 as u16*t)/255) as u8,
     ((a.2 as u16 * (255-t) + b.2 as u16*t)/255) as u8)
}
