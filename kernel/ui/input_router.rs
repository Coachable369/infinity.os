//! Trusted, modal-aware routing from normalized device events to window owners.

use super::geometry::Point;
use super::trusted::TrustedUiManager;
use super::window::{ContextId, WindowId, WindowServer};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum InputDestination {
    Window(WindowId),
    SecureContext(ContextId),
}

pub struct InputRouter;

impl InputRouter {
    // ------------------------=
    // FUNC: route_pointer
    // DESC: Routes normalized pointer input through secure ownership, capture, modal policy, and z-order hit testing.
    // ------------------=
    pub fn route_pointer(
        windows: &WindowServer,
        trusted: &TrustedUiManager,
        point: Point,
        now: u64,
    ) -> Option<InputDestination> {
        if let Some(owner) = trusted.secure_input_owner(now) {
            return Some(InputDestination::SecureContext(ContextId(owner)));
        }
        windows.pointer_target(point).map(InputDestination::Window)
    }

    // ------------------------=
    // FUNC: route_keyboard
    // DESC: Routes keyboard input only to the secure-input owner or currently focused eligible window.
    // ------------------=
    pub fn route_keyboard(
        windows: &WindowServer,
        trusted: &TrustedUiManager,
        now: u64,
    ) -> Option<InputDestination> {
        if let Some(owner) = trusted.secure_input_owner(now) {
            return Some(InputDestination::SecureContext(ContextId(owner)));
        }
        windows.focused().map(InputDestination::Window)
    }
}
