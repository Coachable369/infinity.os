//! Typed native application window operations. Caller/context are supplied by
//! the execution dispatcher, never read from an untrusted application packet.
use super::{InfinityRuntime, UiOperationError, capability::{CapabilityId, CapabilityType}, execution::SecurityIdentity};
use crate::ui::{geometry::{Point, Rect, Size}, window::{ContextId, WindowId, WindowState, WindowError}};
use super::native_memory::{PageTable, copy_from_user};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command<'a> {
    Title(&'a str),
    Move(Point),
    Resize(Size),
    State(WindowState),
    Focus,
    Capture,
    ReleaseCapture,
    ModalParent(WindowId),
    Close,
}

impl InfinityRuntime {
    // ------------------------=
    // FUNC: native_window_syscall
    // DESC: Decodes the stable C ABI using checked user copies and a dispatcher-supplied security identity.
    // ------------------=
    pub fn native_window_syscall<T: PageTable>(&mut self, caller: SecurityIdentity,
        context: ContextId, memory: &T, operation: u64, args: [u64; 6], work_area: Rect,
        display_area: Rect, now: u64) -> i64 {
        let Ok(id) = u32::try_from(args[1]) else { return -3; };
        if operation == 0x1100 {
            let (Ok(x), Ok(y), Ok(width), Ok(height)) = (i32::try_from(args[2] as i64),
                i32::try_from(args[3] as i64), u32::try_from(args[4]), u32::try_from(args[5])) else { return -3; };
            if width == 0 || height == 0 || width > work_area.width || height > work_area.height { return -3; }
            return match self.create_window(caller, context, args[0], crate::ui::window::SurfaceId(id),
                Rect { x, y, width, height }, crate::ui::window::ZOrderClass::Normal, now) {
                Ok(window) => {
                    self.ui.damage.add(Rect { x, y, width, height });
                    window.0 as i64
                }, Err(error) => abi_error(error),
            };
        }
        let mut title = [0u8; crate::ui::window::MAX_WINDOW_TITLE];
        let command = match operation {
            0x1101 => Command::Close,
            0x1102 => {
                let Ok(length) = usize::try_from(args[3]) else { return -3; };
                if length > title.len() { return -3; }
                if copy_from_user(memory, args[2], &mut title[..length]).is_err() { return -5; }
                let Ok(text) = core::str::from_utf8(&title[..length]) else { return -3; };
                Command::Title(text)
            }
            0x1103 => {
                let (Ok(x), Ok(y)) = (i32::try_from(args[2] as i64), i32::try_from(args[3] as i64)) else { return -3; };
                Command::Move(Point { x, y })
            }
            0x1104 => {
                let (Ok(width), Ok(height)) = (u32::try_from(args[2]), u32::try_from(args[3])) else { return -3; };
                Command::Resize(Size { width, height })
            }
            0x1105 => Command::State(match args[2] {
                0 => WindowState::Normal, 1 => WindowState::Minimized,
                2 => WindowState::Maximized, 3 => WindowState::Fullscreen, _ => return -3,
            }),
            0x1106 => Command::Focus,
            0x1107 => Command::Capture,
            0x1108 => Command::ReleaseCapture,
            _ => return -6,
        };
        match self.native_window_command(caller, context, args[0], WindowId(id), command, work_area, display_area, now) {
            Ok(_) => 0, Err(error) => abi_error(error),
        }
    }
    // ------------------------=
    // FUNC: native_window_command
    // DESC: Authorizes every window mutation independently and publishes committed transitions through the existing IOP event path.
    // ------------------=
    pub fn native_window_command(&mut self, caller: SecurityIdentity, context: ContextId,
        capability: CapabilityId, window: WindowId, command: Command<'_>, work_area: Rect,
        display_area: Rect, now: u64) -> Result<Rect, UiOperationError> {
        let permission = match command {
            Command::Focus => CapabilityType::WindowFocus,
            Command::Capture | Command::ReleaseCapture => CapabilityType::WindowCaptureInput,
            _ => CapabilityType::WindowManageOwn,
        };
        self.capabilities.validate(capability, caller, permission, window.0 as u64, 1, 0, now)
            .map_err(|_| UiOperationError::AccessDenied)?;
        let before = self.ui.windows.inspect(window).copied()
            .ok_or(UiOperationError::Window(WindowError::Unknown))?;
        if before.owner != context { return Err(UiOperationError::AccessDenied); }
        let bounds = match command {
            Command::Title(title) => self.ui.windows.set_title(context, window, title).map(|_| before.bounds),
            Command::Move(point) => self.ui.windows.move_window(context, window, point, work_area),
            Command::Resize(size) => self.ui.windows.resize_window(context, window, size.width, size.height, work_area),
            Command::State(state) => self.ui.windows.set_state(context, window, state, work_area, display_area).map(|transition| transition.new_bounds),
            Command::Focus => self.ui.windows.focus(window).map(|_| before.bounds),
            Command::Capture => self.ui.windows.capture_pointer(context, window).map(|_| before.bounds),
            Command::ReleaseCapture => self.ui.windows.release_window_pointer(context, window).map(|_| before.bounds),
            Command::ModalParent(parent) => self.ui.windows.set_modal_parent(context, window, parent).map(|_| before.bounds),
            Command::Close => self.ui.windows.close(context, window).map(|closed| closed.bounds),
        }.map_err(UiOperationError::Window)?;
        if !matches!(command, Command::Capture | Command::ReleaseCapture) {
            if before.visible { self.ui.damage.add(before.bounds); }
            if let Some(after) = self.ui.windows.inspect(window) {
                if after.visible { self.ui.damage.add(after.bounds); }
            }
        }
        if command == Command::Close {
            // A closed window releases its association, not its caller-owned
            // pixels; the app may reuse or explicitly destroy that surface.
            if self.ui.surfaces.inspect(before.surface).is_some_and(|surface|
                surface.owner == context && surface.associated_window == Some(window)) {
                self.ui.surfaces.associate_window(context, before.surface, None)
                    .map_err(UiOperationError::Surface)?;
            }
        }
        self.flush_window_events(now, window.0 as u64, window.0 as u64);
        Ok(bounds)
    }
}

// ------------------------=
// FUNC: abi_error
// DESC: Maps typed service failures into stable native ABI errors without exposing kernel internals.
// ------------------=
fn abi_error(error: UiOperationError) -> i64 {
    match error {
        UiOperationError::AccessDenied | UiOperationError::Window(WindowError::AccessDenied | WindowError::ModalBlocked | WindowError::PrivilegedZOrder) => -1,
        UiOperationError::Window(WindowError::Unknown) => -2,
        UiOperationError::Window(WindowError::Full) => -4,
        _ => -3,
    }
}
