//! Typed system-wide desktop icon theme selection and semantic icon catalog.

pub const ICON_THEME_COUNT: u8 = 3;
pub const ICON_ROLE_COUNT: usize = 60;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum IconThemeId {
    CrystalBlueGlass = 0,
    LuminousObsidian = 1,
    FrostedQuartz = 2,
}

impl IconThemeId {
    // ------------------------=
    // FUNC: from_u8
    // DESC: Validates a durable icon theme identifier.
    // ------------------=
    pub const fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::CrystalBlueGlass),
            1 => Some(Self::LuminousObsidian),
            2 => Some(Self::FrostedQuartz),
            _ => None,
        }
    }

    // ------------------------=
    // FUNC: name
    // DESC: Returns the user-facing name of this complete icon family.
    // ------------------=
    pub const fn name(self) -> &'static [u8] {
        match self {
            Self::CrystalBlueGlass => b"Crystal Blue Glass",
            Self::LuminousObsidian => b"Luminous Obsidian",
            Self::FrostedQuartz => b"Frosted Quartz",
        }
    }

    // ------------------------=
    // FUNC: next
    // DESC: Advances through the installed icon families in stable order.
    // ------------------=
    pub const fn next(self) -> Self {
        match self {
            Self::CrystalBlueGlass => Self::LuminousObsidian,
            Self::LuminousObsidian => Self::FrostedQuartz,
            Self::FrostedQuartz => Self::CrystalBlueGlass,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IconThemeRegistry {
    active: IconThemeId,
    generation: u32,
}

impl IconThemeRegistry {
    // ------------------------=
    // FUNC: new
    // DESC: Creates the default desktop icon registry with blue glass selected.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            active: IconThemeId::CrystalBlueGlass,
            generation: 0,
        }
    }

    // ------------------------=
    // FUNC: active
    // DESC: Reads the icon family used by every semantic OS surface.
    // ------------------=
    pub const fn active(&self) -> IconThemeId {
        self.active
    }

    // ------------------------=
    // FUNC: generation
    // DESC: Returns the monotonic theme generation used to invalidate rendered surfaces.
    // ------------------=
    pub const fn generation(&self) -> u32 {
        self.generation
    }

    // ------------------------=
    // FUNC: activate
    // DESC: Atomically selects one validated installed icon family.
    // ------------------=
    pub fn activate(&mut self, value: u8) -> Result<IconThemeId, ()> {
        let next = IconThemeId::from_u8(value).ok_or(())?;
        if self.active != next {
            self.active = next;
            self.generation = self.generation.wrapping_add(1);
        }
        Ok(next)
    }

    // ------------------------=
    // FUNC: cycle
    // DESC: Selects the next installed icon family and invalidates consumers.
    // ------------------=
    pub fn cycle(&mut self) -> IconThemeId {
        self.active = self.active.next();
        self.generation = self.generation.wrapping_add(1);
        self.active
    }
}
