#[derive(Clone, Copy)]
pub enum DeviceKind {
    Display,
    Keyboard,
    Mouse,
}

#[derive(Clone, Copy)]
pub enum DeviceState {
    Ready,
    Unavailable,
}

#[derive(Clone, Copy)]
pub struct DeviceIdentity {
    pub name: &'static str,
    pub kind: DeviceKind,
    pub driver: &'static str,
    pub state: DeviceState,
}

impl DeviceKind {
    // ------------------------=
    // FUNC: name
    // DESC: Implements the name operation.
    // ------------------=
    pub const fn name(self) -> &'static [u8] {
        match self {
            Self::Display => b"Display",
            Self::Keyboard => b"Input",
            Self::Mouse => b"Input",
        }
    }
}

impl DeviceState {
    // ------------------------=
    // FUNC: name
    // DESC: Implements the name operation.
    // ------------------=
    pub const fn name(self) -> &'static [u8] {
        match self {
            Self::Ready => b"ready",
            Self::Unavailable => b"unavailable",
        }
    }

    // ------------------------=
    // FUNC: is_ready
    // DESC: Reports whether is ready.
    // ------------------=
    pub const fn is_ready(self) -> bool {
        matches!(self, Self::Ready)
    }
}

impl DeviceIdentity {
    // ------------------------=
    // FUNC: new
    // DESC: Creates and initializes a new instance.
    // ------------------=
    pub const fn new(
        name: &'static str,
        kind: DeviceKind,
        driver: &'static str,
        state: DeviceState,
    ) -> Self {
        Self {
            name,
            kind,
            driver,
            state,
        }
    }
}
