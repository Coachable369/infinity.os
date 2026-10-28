//! Window-local native input over IEF; only trusted normalized device dispatch
//! calls this adapter. There is deliberately no input-injection syscall here.
use super::InfinityRuntime;
use crate::ui::{
    input::{Key, PointerPhase, UiEvent},
    input_router::{InputDestination, InputRouter},
};

impl InfinityRuntime {
    // ------------------------=
    // FUNC: native_input
    // DESC: Routes normalized input through secure focus/capture policy and publishes a bounded window-scoped C packet.
    // ------------------=
    pub fn native_input(
        &mut self,
        event: UiEvent<'_>,
        scroll_x: i32,
        scroll_y: i32,
        now: u64,
    ) -> bool {
        let destination = match event {
            UiEvent::Pointer { position, .. } => {
                InputRouter::route_pointer(&self.ui.windows, &self.ui.trusted, position, now)
            }
            _ => InputRouter::route_keyboard(&self.ui.windows, &self.ui.trusted, now),
        };
        let Some(InputDestination::Window(id)) = destination else {
            return false;
        };
        let Some(window) = self.ui.windows.inspect(id).copied() else {
            return false;
        };
        if !window.visible || !window.accepts_input {
            return false;
        }
        let mut bytes = [0u8; 96];
        bytes[..4].copy_from_slice(&1u32.to_le_bytes());
        bytes[8..12].copy_from_slice(&id.0.to_le_bytes());
        let (kind, flags, a, b, c, d, text): (u32, u32, i32, i32, i32, i32, &[u8]) = match event {
            UiEvent::Pointer {
                position,
                phase,
                button,
            } => (
                3,
                match phase {
                    PointerPhase::Move => 0,
                    PointerPhase::Down => 1,
                    PointerPhase::Up => 2,
                    PointerPhase::Scroll => 3,
                    PointerPhase::Cancel => 4,
                } | ((button as u32) << 8),
                position.x.saturating_sub(window.bounds.x),
                position.y.saturating_sub(window.bounds.y),
                scroll_x,
                scroll_y,
                &[],
            ),
            UiEvent::Key {
                key,
                pressed,
                modifiers,
            } => {
                let (code, unicode) = key_code(key);
                (
                    1,
                    (pressed as u32) | ((modifiers as u32) << 8),
                    code,
                    unicode,
                    0,
                    0,
                    &[],
                )
            }
            UiEvent::Text(text) => (2, 0, 0, 0, text.len() as i32, 0, text),
            UiEvent::CompositionStart => (4, 0, 0, 0, 0, 0, &[]),
            UiEvent::CompositionUpdate(text) => (5, 0, 0, 0, text.len() as i32, 0, text),
            UiEvent::CompositionCommit(text) => (6, 0, 0, 0, text.len() as i32, 0, text),
            UiEvent::CompositionCancel => (7, 0, 0, 0, 0, 0, &[]),
        };
        if text.len() > 64 || core::str::from_utf8(text).is_err() {
            return false;
        }
        bytes[4..8].copy_from_slice(&kind.to_le_bytes());
        bytes[12..16].copy_from_slice(&flags.to_le_bytes());
        for (index, value) in [a, b, c, d].iter().enumerate() {
            bytes[16 + index * 4..20 + index * 4].copy_from_slice(&value.to_le_bytes());
        }
        bytes[32..32 + text.len()].copy_from_slice(text);
        self.publish_ui_event(
            super::EVENT_NATIVE_INPUT,
            id.0 as u64,
            &bytes,
            200,
            now,
            0,
            0,
        )
    }
}

// ------------------------=
// FUNC: key_code
// DESC: Encodes stable C key identifiers independently of Rust enum representation and keyboard hardware.
// ------------------=
fn key_code(key: Key) -> (i32, i32) {
    (
        match key {
            Key::Tab => 1,
            Key::Enter => 2,
            Key::Escape => 3,
            Key::Space => 4,
            Key::Left => 5,
            Key::Right => 6,
            Key::Up => 7,
            Key::Down => 8,
            Key::Home => 9,
            Key::End => 10,
            Key::Backspace => 11,
            Key::Delete => 12,
            Key::Character(_) => 13,
        },
        match key {
            Key::Character(value) => value as i32,
            _ => 0,
        },
    )
}
