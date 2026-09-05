//! Typed catalog and deterministic filtering for the installed native app launcher.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LauncherAction {
    Home(usize),
    Settings(usize),
    TextEditor,
    CommandWindow,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LauncherEntry {
    pub label: &'static [u8],
    pub icon_role: usize,
    pub action: LauncherAction,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DockAction {
    Launcher,
    Files,
    Settings,
    About,
    AiVoice,
    Appearance,
    Network,
    Trash,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DockEntry {
    pub icon_kind: usize,
    pub action: DockAction,
}

pub const DESKTOP_DOCK_ENTRIES: [DockEntry; 8] = [
    DockEntry {
        icon_kind: usize::MAX,
        action: DockAction::Launcher,
    },
    DockEntry {
        icon_kind: 1,
        action: DockAction::Files,
    },
    DockEntry {
        icon_kind: 2,
        action: DockAction::Settings,
    },
    DockEntry {
        icon_kind: 3,
        action: DockAction::About,
    },
    DockEntry {
        icon_kind: 4,
        action: DockAction::AiVoice,
    },
    DockEntry {
        icon_kind: 5,
        action: DockAction::Appearance,
    },
    DockEntry {
        icon_kind: 6,
        action: DockAction::Network,
    },
    DockEntry {
        icon_kind: 7,
        action: DockAction::Trash,
    },
];

pub const LAUNCHER_APPS: [LauncherEntry; 12] = [
    LauncherEntry {
        label: b"Files",
        icon_role: 3,
        action: LauncherAction::Home(0),
    },
    LauncherEntry {
        label: b"Documents",
        icon_role: 4,
        action: LauncherAction::Home(2),
    },
    LauncherEntry {
        label: b"Projects",
        icon_role: 9,
        action: LauncherAction::Home(7),
    },
    LauncherEntry {
        label: b"Media",
        icon_role: 8,
        action: LauncherAction::Home(6),
    },
    LauncherEntry {
        label: b"Recycle Bin",
        icon_role: 10,
        action: LauncherAction::Home(8),
    },
    LauncherEntry {
        label: b"Settings",
        icon_role: 26,
        action: LauncherAction::Settings(0),
    },
    LauncherEntry {
        label: b"Text Editor",
        icon_role: 49,
        action: LauncherAction::TextEditor,
    },
    LauncherEntry {
        label: b"AI & Voice",
        icon_role: 23,
        action: LauncherAction::Settings(3),
    },
    LauncherEntry {
        label: b"Security",
        icon_role: 32,
        action: LauncherAction::Settings(4),
    },
    LauncherEntry {
        label: b"Storage",
        icon_role: 12,
        action: LauncherAction::Settings(6),
    },
    LauncherEntry {
        label: b"About",
        icon_role: 28,
        action: LauncherAction::Settings(7),
    },
    LauncherEntry {
        label: b"Command Window",
        icon_role: 25,
        action: LauncherAction::CommandWindow,
    },
];

pub const LAUNCHER_CATEGORIES: [LauncherEntry; 5] = [
    LauncherEntry {
        label: b"Home",
        icon_role: 0,
        action: LauncherAction::Home(0),
    },
    LauncherEntry {
        label: b"Work",
        icon_role: 9,
        action: LauncherAction::Home(7),
    },
    LauncherEntry {
        label: b"System",
        icon_role: 19,
        action: LauncherAction::Settings(0),
    },
    LauncherEntry {
        label: b"Utilities",
        icon_role: 25,
        action: LauncherAction::CommandWindow,
    },
    LauncherEntry {
        label: b"Personal Space",
        icon_role: 1,
        action: LauncherAction::Home(1),
    },
];

// ------------------------=
// FUNC: launcher_query_matches
// DESC: Performs allocation-free ASCII case-insensitive substring matching for launcher search.
// ------------------=
pub fn launcher_query_matches(label: &[u8], query: &[u8]) -> bool {
    if query.is_empty() {
        return true;
    }
    if query.len() > label.len() {
        return false;
    }
    label.windows(query.len()).any(|window| {
        window
            .iter()
            .zip(query.iter())
            .all(|(left, right)| left.eq_ignore_ascii_case(right))
    })
}

// ------------------------=
// FUNC: launcher_visible_count
// DESC: Counts application results matching the live native launcher query.
// ------------------=
pub fn launcher_visible_count(query: &[u8]) -> usize {
    LAUNCHER_APPS
        .iter()
        .filter(|entry| launcher_query_matches(entry.label, query))
        .count()
}

// ------------------------=
// FUNC: launcher_visible_entry
// DESC: Resolves one compacted visible grid slot into its typed launcher entry.
// ------------------=
pub fn launcher_visible_entry(query: &[u8], visible_index: usize) -> Option<LauncherEntry> {
    LAUNCHER_APPS
        .iter()
        .filter(|entry| launcher_query_matches(entry.label, query))
        .nth(visible_index)
        .copied()
}
