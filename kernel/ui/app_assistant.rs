//! Shared app-window assistant contract. No ambient file access or command execution.
use super::geometry::{Point, Rect};
pub const PANEL_SLOTS: usize = 16;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    None,
    Maximize,
    Restore,
    Minimize,
    Find,
    Insert,
    Undo,
    Redo,
    SelectAll,
    Save,
    Refresh,
    Navigate,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Toggle,
    Composer,
    Send,
    Apply,
    Dismiss,
    Body,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Panel {
    pub expanded: bool,
    pub focused: bool,
    pub input: [u8; 192],
    pub length: usize,
    pub response: [u8; 512],
    pub request: [u8; 192],
    pub request_len: usize,
    pub response_len: usize,
    pub pending: Action,
    pub argument: [u8; 192],
    pub argument_len: usize,
    pub document_revision: u64,
}
impl Panel {
    // ------------------------=
    // FUNC: new
    // DESC: Creates a collapsed private app assistant with no captured document content.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            expanded: false,
            focused: false,
            input: [0; 192],
            length: 0,
            response: [0; 512],
            request: [0; 192],
            request_len: 0,
            response_len: 0,
            pending: Action::None,
            argument: [0; 192],
            argument_len: 0,
            document_revision: 0,
        }
    }
    // ------------------------=
    // FUNC: reply
    // DESC: Records a bounded response without creating external side effects.
    // ------------------=
    pub fn reply(&mut self, bytes: &[u8]) {
        self.response_len = bytes.len().min(self.response.len());
        self.response[..self.response_len].copy_from_slice(&bytes[..self.response_len]);
    }
    // ------------------------=
    // FUNC: propose
    // DESC: Resolves supported local app intents into explicit reviewable actions; unsupported requests never execute.
    // ------------------=
    pub fn propose(&mut self, editor: bool, revision: u64) -> bool {
        self.pending = Action::None;
        self.argument_len = 0;
        let mut command = self.input;
        for c in &mut command[..self.length] {
            *c = c.to_ascii_lowercase();
        }
        let text = &command[..self.length];
        let action = match text {
            b"maximize" | b"maximize window" => Action::Maximize,
            b"restore" | b"restore window" => Action::Restore,
            b"minimize" | b"minimize window" => Action::Minimize,
            b"undo" if editor => Action::Undo,
            b"redo" if editor => Action::Redo,
            b"select all" if editor => Action::SelectAll,
            b"save" if editor => Action::Save,
            b"refresh" => Action::Refresh,
            _ if !editor && text.starts_with(b"open folder ") => {
                self.argument_len = self.length - 12;
                self.argument[..self.argument_len].copy_from_slice(&self.input[12..self.length]);
                Action::Navigate
            }
            _ if editor && text.starts_with(b"find ") => {
                self.argument_len = self.length - 5;
                self.argument[..self.argument_len].copy_from_slice(&self.input[5..self.length]);
                Action::Find
            }
            _ if editor && text.starts_with(b"insert ") => {
                self.argument_len = self.length - 7;
                self.argument[..self.argument_len].copy_from_slice(&self.input[7..self.length]);
                Action::Insert
            }
            _ => Action::None,
        };
        if action == Action::None {
            return false;
        }
        self.pending = action;
        self.document_revision = revision;
        if action == Action::Navigate {
            self.reply(b"Navigate this File Navigator to the supplied folder. Apply to confirm.");
            return true;
        }
        self.reply(match action {Action::Maximize=>b"Maximize this window. Apply to confirm.",Action::Restore=>b"Restore this window's saved size. Apply to confirm.",Action::Minimize=>b"Minimize this window. Apply to confirm.",Action::Find=>b"Find and select this text in the current document. Apply to confirm.",Action::Insert=>b"Insert your supplied text at the caret, replacing any selection. This is undoable. Apply to confirm.",Action::Undo=>b"Undo the latest document edit. Apply to confirm.",Action::Redo=>b"Redo the last undone edit. Apply to confirm.",Action::SelectAll=>b"Select the current document. Apply to confirm.",Action::Save=>b"Save the current document through the app's normal save workflow. Apply to confirm.",_=>b"Refresh this app's visible state. Apply to confirm."});
        true
    }
    // ------------------------=
    // FUNC: take_action
    // DESC: Rejects stale document proposals and consumes confirmed actions exactly once.
    // ------------------=
    pub fn take_action(&mut self, revision: u64) -> Action {
        let action = self.pending;
        self.pending = Action::None;
        if matches!(
            action,
            Action::Find
                | Action::Insert
                | Action::Undo
                | Action::Redo
                | Action::SelectAll
                | Action::Save
        ) && revision != self.document_revision
        {
            self.reply(b"The document changed. Submit the request again before applying.");
            return Action::None;
        }
        action
    }
}
#[derive(Clone, Copy)]
pub struct Geometry {
    pub panel: Rect,
    pub toggle: Rect,
    pub composer: Rect,
    pub send: Rect,
    pub apply: Rect,
    pub dismiss: Rect,
}
// ------------------------=
// FUNC: geometry
// DESC: Places the collapse tab at the upper-middle right edge and docks the expanded panel inside window bounds.
// ------------------=
pub fn geometry(window: Rect, scale: usize, expanded: bool) -> Geometry {
    let s = scale.max(1) as u32;
    let width = (window.width * 336 / 1000)
        .max(320 * s)
        .min(480 * s)
        .min(window.width.saturating_sub(280 * s))
        .max(1);
    let panel = Rect {
        x: window.right() - width as i32 - s as i32,
        y: window.y + 48 * s as i32,
        width,
        height: window.height.saturating_sub(49 * s),
    };
    let toggle = Rect {
        x: window.right() - 32 * s as i32,
        y: if expanded {
            panel.y + 8 * s as i32
        } else {
            window.y + (window.height / 3).max(64 * s) as i32
        },
        width: 28 * s,
        height: if expanded { 32 * s } else { 72 * s },
    };
    let composer = Rect {
        x: panel.x + 12 * s as i32,
        y: panel.bottom() - 52 * s as i32,
        width: width.saturating_sub(68 * s),
        height: 40 * s,
    };
    let send = Rect {
        x: composer.right() + 8 * s as i32,
        y: composer.y,
        width: 36 * s,
        height: 40 * s,
    };
    let apply = Rect {
        x: panel.x + 12 * s as i32,
        y: composer.y - 44 * s as i32,
        width: (width.saturating_sub(32 * s)) / 2,
        height: 32 * s,
    };
    let dismiss = Rect {
        x: apply.right() + 8 * s as i32,
        y: apply.y,
        width: apply.width,
        height: apply.height,
    };
    Geometry {
        panel,
        toggle,
        composer,
        send,
        apply,
        dismiss,
    }
}
// ------------------------=
// FUNC: hit
// DESC: Routes panel hits ahead of app controls, preventing click-through into obscured content.
// ------------------=
pub fn hit(g: Geometry, expanded: bool, p: Point) -> Option<Target> {
    if g.toggle.contains(p) {
        return Some(Target::Toggle);
    }
    if !expanded {
        return None;
    }
    for (r, t) in [
        (g.composer, Target::Composer),
        (g.send, Target::Send),
        (g.apply, Target::Apply),
        (g.dismiss, Target::Dismiss),
        (g.panel, Target::Body),
    ] {
        if r.contains(p) {
            return Some(t);
        }
    }
    None
}
static mut PANELS: [Panel; PANEL_SLOTS] = [Panel::new(); PANEL_SLOTS];
static REVISION: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
// ------------------------=
// FUNC: reset
// DESC: Erases all transient app conversations and pending authority when the authenticated session changes.
// ------------------=
pub fn reset() {
    unsafe {
        *(&raw mut PANELS) = [Panel::new(); PANEL_SLOTS];
    }
    REVISION.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
}
// ------------------------=
// FUNC: read
// DESC: Retrieves only the requested native window's conversation state on the UI thread.
// ------------------=
pub fn read(id: usize) -> Panel {
    unsafe {
        (*(&raw const PANELS))
            .get(id)
            .copied()
            .unwrap_or(Panel::new())
    }
}
// ------------------------=
// FUNC: write
// DESC: Publishes one native window's state without modifying other app conversations.
// ------------------=
pub fn write(id: usize, panel: Panel) {
    if id < PANEL_SLOTS {
        unsafe {
            if (*(&raw const PANELS))[id] != panel {
                (*(&raw mut PANELS))[id] = panel;
                REVISION.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
            }
        }
    }
}
// ------------------------=
// FUNC: revision
// DESC: Supplies the shared window compositor with a non-secret assistant damage token.
// ------------------=
pub fn revision() -> u32 {
    REVISION.load(core::sync::atomic::Ordering::Relaxed)
}
// ------------------------=
// FUNC: fingerprint
// DESC: Binds a proposal to document bytes, caret and selection without retaining private text in panel state.
// ------------------=
pub fn fingerprint(bytes: &[u8], cursor: usize, selection: Option<(usize, usize)>) -> u64 {
    let mut h = 1469598103934665603u64;
    for b in bytes {
        h = (h ^ *b as u64).wrapping_mul(1099511628211);
    }
    h ^ cursor as u64 ^ selection.map_or(0, |(a, b)| ((a as u64) << 32) | b as u64)
}
