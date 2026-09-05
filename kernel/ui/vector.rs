//! Semantic icon identities and renderer-native bounded vector commands.

use super::geometry::{Point, Rect};

pub const MAX_VECTOR_COMMANDS: usize = 32;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u16)]
pub enum IconId {
    Infinity = 1,
    Wifi,
    User,
    UserAdd,
    Lock,
    Eye,
    ArrowRight,
    Power,
    Restart,
    Accessibility,
    Settings,
    Search,
    Menu,
    Volume,
    Bluetooth,
    Battery,
    Home,
    Folder,
    Document,
    Download,
    Picture,
    Music,
    Video,
    Project,
    Trash,
    Shield,
    Network,
    Storage,
    Device,
    Display,
    Microphone,
    Ai,
    Terminal,
    Close,
    Minimize,
    Maximize,
    Back,
    Forward,
    ChevronDown,
    Check,
    Warning,
    Information,
    Help,
    Calendar,
    Clock,
    Key,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum VectorCommand {
    Move(Point),
    Line(Point),
    Cubic(Point, Point, Point),
    Close,
}

#[derive(Clone, Copy)]
pub struct VectorIcon {
    pub id: IconId,
    pub view_box: Rect,
    pub commands: [Option<VectorCommand>; MAX_VECTOR_COMMANDS],
    pub command_count: u8,
    pub stroke_width: u8,
    pub filled: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum VectorError {
    Empty,
    TooComplex,
    InvalidViewBox,
    CoordinateOverflow,
}

// ------------------------=
// FUNC: validate
// DESC: Rejects malformed or unbounded vector assets before they enter a skin cache.
// ------------------=
pub fn validate(icon: &VectorIcon) -> Result<(), VectorError> {
    if icon.command_count == 0 {
        return Err(VectorError::Empty);
    }
    if icon.command_count as usize > MAX_VECTOR_COMMANDS {
        return Err(VectorError::TooComplex);
    }
    if icon.view_box.width == 0 || icon.view_box.height == 0 {
        return Err(VectorError::InvalidViewBox);
    }
    for command in icon.commands[..icon.command_count as usize]
        .iter()
        .flatten()
    {
        let points = match command {
            VectorCommand::Move(a) | VectorCommand::Line(a) => {
                [*a, Point { x: 0, y: 0 }, Point { x: 0, y: 0 }]
            }
            VectorCommand::Cubic(a, b, c) => [*a, *b, *c],
            VectorCommand::Close => continue,
        };
        if points
            .iter()
            .any(|point| point.x.abs() > 32_000 || point.y.abs() > 32_000)
        {
            return Err(VectorError::CoordinateOverflow);
        }
    }
    Ok(())
}

// ------------------------=
// FUNC: semantic_name
// DESC: Projects a stable icon identity into a human-readable diagnostics name.
// ------------------=
pub const fn semantic_name(id: IconId) -> &'static [u8] {
    match id {
        IconId::Infinity => b"infinity",
        IconId::Wifi => b"wifi",
        IconId::User => b"user",
        IconId::UserAdd => b"user-add",
        IconId::Lock => b"lock",
        IconId::Eye => b"eye",
        IconId::ArrowRight => b"arrow-right",
        IconId::Power => b"power",
        IconId::Restart => b"restart",
        IconId::Accessibility => b"accessibility",
        IconId::Settings => b"settings",
        IconId::Search => b"search",
        IconId::Menu => b"menu",
        IconId::Volume => b"volume",
        IconId::Bluetooth => b"bluetooth",
        IconId::Battery => b"battery",
        IconId::Home => b"home",
        IconId::Folder => b"folder",
        IconId::Document => b"document",
        IconId::Download => b"download",
        IconId::Picture => b"picture",
        IconId::Music => b"music",
        IconId::Video => b"video",
        IconId::Project => b"project",
        IconId::Trash => b"trash",
        IconId::Shield => b"shield",
        IconId::Network => b"network",
        IconId::Storage => b"storage",
        IconId::Device => b"device",
        IconId::Display => b"display",
        IconId::Microphone => b"microphone",
        IconId::Ai => b"ai",
        IconId::Terminal => b"terminal",
        IconId::Close => b"close",
        IconId::Minimize => b"minimize",
        IconId::Maximize => b"maximize",
        IconId::Back => b"back",
        IconId::Forward => b"forward",
        IconId::ChevronDown => b"chevron-down",
        IconId::Check => b"check",
        IconId::Warning => b"warning",
        IconId::Information => b"information",
        IconId::Help => b"help",
        IconId::Calendar => b"calendar",
        IconId::Clock => b"clock",
        IconId::Key => b"key",
    }
}
