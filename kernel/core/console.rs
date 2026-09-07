use crate::intent::{
    ConsoleMode, IntentContext, IntentRuntime, KnownOperationPolicy, OperationPolicy,
    ResolutionSource, SystemOperation,
};
use crate::storage::{
    DateTimeConfiguration, StorageDevice, StorageError, StorageManager, StorageProfile,
    StorageProvisioningPlan,
};
use crate::system::SystemSnapshot;
use crate::ui::app_launcher::{
    launcher_visible_count, launcher_visible_entry, DockAction, LauncherAction,
    DESKTOP_DOCK_ENTRIES, LAUNCHER_CATEGORIES,
};
use crate::ui::session_state::{
    DesktopResumeSurface, DesktopSessionLayout, LockedDesktopState, SessionIdleState,
    WindowPlacement,
};
use crate::ui::system_layout::{
    AiChatTarget, AppLauncherTarget, DesktopAppWindowState, DesktopAppWindowTarget, DesktopTarget,
    EditorDialogTarget, EditorScrollTarget, NetworkSettingsTarget, OnboardingTarget,
    SettingsAccentTarget, SettingsTarget, SettingsWindowState, SystemLayout, SystemMenuTarget,
};
use crate::ui::text_editor::TextDocument;

const OUTPUT_ROWS: usize = 6;
const LINE_CAPACITY: usize = 96;
const COMMAND_CAPACITY: usize = 160;

#[derive(Clone, Copy, PartialEq, Eq)]
enum EditorDialog {
    None,
    SaveAs,
    Open,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DesktopAppKind {
    None,
    TextEditor,
    CommandWindow,
    TaskManager,
}

#[derive(Clone, Copy)]
struct TimeZoneChoice {
    id: u16,
    offset_minutes: i16,
    longitude_degrees: i16,
    label: &'static [u8],
}

const TIME_ZONES: [TimeZoneChoice; 11] = [
    TimeZoneChoice {
        id: 1,
        offset_minutes: -480,
        longitude_degrees: -122,
        label: b"UTC-08:00  Pacific",
    },
    TimeZoneChoice {
        id: 2,
        offset_minutes: -420,
        longitude_degrees: -111,
        label: b"UTC-07:00  Mountain",
    },
    TimeZoneChoice {
        id: 3,
        offset_minutes: -360,
        longitude_degrees: -95,
        label: b"UTC-06:00  Central",
    },
    TimeZoneChoice {
        id: 4,
        offset_minutes: -300,
        longitude_degrees: -74,
        label: b"UTC-05:00  Eastern",
    },
    TimeZoneChoice {
        id: 5,
        offset_minutes: -240,
        longitude_degrees: -63,
        label: b"UTC-04:00  Atlantic",
    },
    TimeZoneChoice {
        id: 6,
        offset_minutes: 0,
        longitude_degrees: 0,
        label: b"UTC+00:00  Universal",
    },
    TimeZoneChoice {
        id: 7,
        offset_minutes: 60,
        longitude_degrees: 10,
        label: b"UTC+01:00  Central Europe",
    },
    TimeZoneChoice {
        id: 8,
        offset_minutes: 330,
        longitude_degrees: 78,
        label: b"UTC+05:30  India",
    },
    TimeZoneChoice {
        id: 9,
        offset_minutes: 480,
        longitude_degrees: 104,
        label: b"UTC+08:00  Singapore",
    },
    TimeZoneChoice {
        id: 10,
        offset_minutes: 540,
        longitude_degrees: 139,
        label: b"UTC+09:00  Japan",
    },
    TimeZoneChoice {
        id: 11,
        offset_minutes: 600,
        longitude_degrees: 151,
        label: b"UTC+10:00  Eastern Australia",
    },
];

// ------------------------=
// FUNC: time_zone_index
// DESC: Selects the closest supported typed time-zone projection for a firmware UTC offset.
// ------------------=
fn time_zone_index(offset_minutes: i16) -> usize {
    let mut selected = 0usize;
    let mut distance = i32::MAX;
    for (index, zone) in TIME_ZONES.iter().enumerate() {
        let candidate = (zone.offset_minutes as i32 - offset_minutes as i32).abs();
        if candidate < distance {
            selected = index;
            distance = candidate;
        }
    }
    selected
}

// ------------------------=
// FUNC: wrap_installer_value
// DESC: Moves a bounded installer value by one step with deterministic wraparound.
// ------------------=
fn wrap_installer_value(value: i32, minimum: i32, maximum: i32, reverse: bool) -> i32 {
    if reverse {
        if value <= minimum {
            maximum
        } else {
            value - 1
        }
    } else if value >= maximum {
        minimum
    } else {
        value + 1
    }
}

// ------------------------=
// FUNC: parse_ipv4
// DESC: Parses one canonical dotted IPv4 value into its typed address representation.
// ------------------=
fn parse_ipv4(input: &[u8]) -> Option<[u8; 4]> {
    let mut result = [0u8; 4];
    let mut part = 0usize;
    let mut value = 0u16;
    let mut digits = 0usize;
    for byte in input.iter().copied().chain(core::iter::once(b'.')) {
        if byte == b'.' {
            if digits == 0 || part >= result.len() || value > 255 {
                return None;
            }
            result[part] = value as u8;
            part += 1;
            value = 0;
            digits = 0;
        } else if byte.is_ascii_digit() {
            value = value
                .saturating_mul(10)
                .saturating_add((byte - b'0') as u16);
            digits += 1;
            if digits > 3 || value > 255 {
                return None;
            }
        } else {
            return None;
        }
    }
    (part == 4).then_some(result)
}

// ------------------------=
// FUNC: parse_bounded_number
// DESC: Parses an unsigned decimal settings value while enforcing its semantic upper bound.
// ------------------=
fn parse_bounded_number(input: &[u8], maximum: u32) -> Option<u32> {
    if input.is_empty() {
        return None;
    }
    let mut value = 0u32;
    for byte in input.iter().copied() {
        if !byte.is_ascii_digit() {
            return None;
        }
        value = value.checked_mul(10)?.checked_add((byte - b'0') as u32)?;
    }
    (value <= maximum).then_some(value)
}

#[derive(Clone, Copy)]
pub enum ConsoleKey {
    Character(u8),
    Backspace,
    Enter,
    Escape,
    Tab(bool),
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    Delete,
    Help,
}

// ------------------------=
// FUNC: text_edit_key
// DESC: Converts kernel input events into the UI-owned text editing contract.
// ------------------=
fn text_edit_key(key: ConsoleKey) -> Option<crate::ui::text_input::TextEditKey> {
    use crate::ui::text_input::TextEditKey;
    match key {
        ConsoleKey::Character(value) => Some(TextEditKey::Character(value)),
        ConsoleKey::Backspace => Some(TextEditKey::Backspace),
        ConsoleKey::Delete => Some(TextEditKey::Delete),
        ConsoleKey::Left => Some(TextEditKey::Left),
        ConsoleKey::Right => Some(TextEditKey::Right),
        ConsoleKey::Home => Some(TextEditKey::Home),
        ConsoleKey::End => Some(TextEditKey::End),
        _ => None,
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum InstallerStep {
    Welcome,
    Hierarchy,
    Discovery,
    Details,
    DateTime,
    Preview,
    Confirm,
    Provisioning,
    Complete,
    Failed,
    Help,
}

struct ConsoleOutput {
    lines: [[u8; LINE_CAPACITY]; OUTPUT_ROWS],
    lengths: [usize; OUTPUT_ROWS],
    count: usize,
}

impl ConsoleOutput {
    // ------------------------=
    // FUNC: new
    // DESC: Creates and initializes a new instance.
    // ------------------=
    const fn new() -> Self {
        Self {
            lines: [[0; LINE_CAPACITY]; OUTPUT_ROWS],
            lengths: [0; OUTPUT_ROWS],
            count: 0,
        }
    }

    // ------------------------=
    // FUNC: clear
    // DESC: Implements the clear operation.
    // ------------------=
    fn clear(&mut self) {
        self.lines = [[0; LINE_CAPACITY]; OUTPUT_ROWS];
        self.lengths = [0; OUTPUT_ROWS];
        self.count = 0;
    }

    // ------------------------=
    // FUNC: write_line
    // DESC: Writes or updates write line data.
    // ------------------=
    fn write_line(&mut self, line: &[u8]) {
        if self.count == OUTPUT_ROWS {
            for row in 1..OUTPUT_ROWS {
                self.lines[row - 1] = self.lines[row];
                self.lengths[row - 1] = self.lengths[row];
            }
            self.count -= 1;
        }
        let length = line.len().min(LINE_CAPACITY);
        self.lines[self.count][..length].copy_from_slice(&line[..length]);
        self.lengths[self.count] = length;
        self.count += 1;
        crate::output_text(line);
        crate::output_text(b"\n");
    }

    // ------------------------=
    // FUNC: write_segments
    // DESC: Writes or updates write segments data.
    // ------------------=
    fn write_segments(&mut self, segments: &[&[u8]]) {
        let mut line = [0u8; LINE_CAPACITY];
        let mut length = 0;
        for segment in segments {
            let copied = segment.len().min(LINE_CAPACITY - length);
            line[length..length + copied].copy_from_slice(&segment[..copied]);
            length += copied;
            if length == LINE_CAPACITY {
                break;
            }
        }
        self.write_line(&line[..length]);
    }

    // ------------------------=
    // FUNC: write_number
    // DESC: Writes or updates write number data.
    // ------------------=
    fn write_number(&mut self, label: &[u8], value: u64) {
        let mut digits = [0u8; 20];
        let mut cursor = digits.len();
        let mut remaining = value;
        loop {
            cursor -= 1;
            digits[cursor] = b'0' + (remaining % 10) as u8;
            remaining /= 10;
            if remaining == 0 {
                break;
            }
        }
        self.write_segments(&[label, &digits[cursor..]]);
    }

    // ------------------------=
    // FUNC: write_number_suffix
    // DESC: Writes a labeled integer followed by a short unit or qualifier.
    // ------------------=
    fn write_number_suffix(&mut self, label: &[u8], value: u64, suffix: &[u8]) {
        let mut digits = [0u8; 20];
        let mut cursor = digits.len();
        let mut remaining = value;
        loop {
            cursor -= 1;
            digits[cursor] = b'0' + (remaining % 10) as u8;
            remaining /= 10;
            if remaining == 0 {
                break;
            }
        }
        self.write_segments(&[label, &digits[cursor..], suffix]);
    }

    // ------------------------=
    // FUNC: write_date_time
    // DESC: Renders one validated local installer date and time without allocating memory.
    // ------------------=
    fn write_date_time(&mut self, label: &[u8], value: DateTimeConfiguration) {
        let mut text = *b"0000-00-00 00:00";
        text[0] = b'0' + ((value.year / 1000) % 10) as u8;
        text[1] = b'0' + ((value.year / 100) % 10) as u8;
        text[2] = b'0' + ((value.year / 10) % 10) as u8;
        text[3] = b'0' + (value.year % 10) as u8;
        text[5] = b'0' + value.month / 10;
        text[6] = b'0' + value.month % 10;
        text[8] = b'0' + value.day / 10;
        text[9] = b'0' + value.day % 10;
        text[11] = b'0' + value.hour / 10;
        text[12] = b'0' + value.hour % 10;
        text[14] = b'0' + value.minute / 10;
        text[15] = b'0' + value.minute % 10;
        self.write_segments(&[label, &text]);
    }
}

struct ConsoleRuntime {
    mode: ConsoleMode,
    output: ConsoleOutput,
    command: [u8; COMMAND_CAPACITY],
    command_length: usize,
    command_cursor: usize,
    caret_visible: bool,
    intent: IntentRuntime,
    language_session: crate::runtime::console_language::ConsoleSession,
    navigation_context: crate::runtime::object_navigation::ConsoleNavigationContext,
    system: SystemSnapshot,
    pointer_x: i32,
    pointer_y: i32,
    pointer_x_remainder: i32,
    preference_pointer_remainder: [i32; 2],
    pointer_y_remainder: i32,
    pointer_pressed: bool,
    pointer_buttons: u8,
    continuous_motion_frames: crate::ui::platform::MotionFrameCoalescer,
    presenting_fast_motion_frame: bool,
    installer_step: InstallerStep,
    installer_return_step: InstallerStep,
    storage_device: Option<StorageDevice>,
    storage_plan: Option<StorageProvisioningPlan>,
    installer_error: Option<StorageError>,
    installer_focus: usize,
    installer_choice: usize,
    installer_date_time: DateTimeConfiguration,
    installer_date_time_part: usize,
    system_step: usize,
    system_focus: usize,
    ai_chat_focus: usize,
    shell_menu: usize,
    onboarding_machine: [u8; 48],
    onboarding_machine_length: usize,
    onboarding_handle: [u8; 48],
    onboarding_handle_length: usize,
    onboarding_name: [u8; 48],
    onboarding_name_length: usize,
    onboarding_secret: [u8; 72],
    onboarding_secret_length: usize,
    current_user: crate::runtime::identity::StableId,
    current_session: crate::runtime::identity::StableId,
    settings_editing: bool,
    selected_node_id: Option<crate::runtime::node::types::NodeId>,
    settings_window: SettingsWindowState,
    settings_window_dragging: bool,
    settings_window_resizing: Option<usize>,
    settings_window_drag_offset_x: i32,
    settings_window_drag_offset_y: i32,
    settings_accent_dirty: bool,
    settings_primary_dirty: bool,
    settings_effects_dirty: bool,
    settings_effect_dragging: Option<usize>,
    settings_timeout_dragging: bool,
    settings_scroll_dragging: bool,
    settings_scroll_grab_offset: i32,
    settings_scroll_target: usize,
    launcher_scroll_dragging: bool,
    launcher_tick_ns: Option<u64>,
    launcher_scroll_grab_offset: i32,
    onboarding_validation_error: bool,
    home_window_x: i32,
    home_window_y: i32,
    home_window_width: i32,
    home_window_height: i32,
    home_window_visible: bool,
    home_window_maximized: bool,
    home_window_restore_x: i32,
    home_window_restore_y: i32,
    home_window_restore_width: i32,
    home_window_restore_height: i32,
    home_window_dragging: bool,
    home_window_resizing: Option<usize>,
    home_window_drag_offset_x: i32,
    home_window_drag_offset_y: i32,
    home_location: usize,
    home_previous_location: usize,
    home_selected_item: Option<usize>,
    home_dragging_item: Option<usize>,
    home_drag_from_desktop: bool,
    home_drag_origin_x: i32,
    home_drag_origin_y: i32,
    home_drag_moved: bool,
    desktop_items: u8,
    desktop_item_positions: [[i32; 2]; 7],
    home_note_location: usize,
    home_note_previous_location: usize,
    home_clipboard_note: bool,
    desktop_clock: DateTimeConfiguration,
    desktop_app: DesktopAppKind,
    command_window_suspended: bool,
    app_window_x: i32,
    app_window_y: i32,
    app_window_width: i32,
    app_window_height: i32,
    app_window_maximized: bool,
    app_window_restore_x: i32,
    app_window_restore_y: i32,
    app_window_restore_width: i32,
    app_window_restore_height: i32,
    app_window_dragging: bool,
    app_window_resizing: Option<usize>,
    app_window_drag_offset_x: i32,
    app_window_drag_offset_y: i32,
    editor_document: TextDocument,
    editor_document_path: [u8; crate::ui::text_editor::DOCUMENT_PATH_CAPACITY],
    editor_document_path_length: usize,
    editor_document_name: [u8; crate::ui::text_editor::DOCUMENT_NAME_CAPACITY],
    editor_document_name_length: usize,
    editor_dialog: EditorDialog,
    editor_scroll_row: usize,
    editor_scroll_dragging: bool,
    editor_scroll_grab_offset: i32,
    editor_window: DesktopAppWindowState,
    command_window: DesktopAppWindowState,
    task_manager_window: DesktopAppWindowState,
    task_manager_selected: usize,
    task_manager_scroll: usize,
    session_idle: SessionIdleState,
    locked_desktop_layout: LockedDesktopState,
}

impl ConsoleRuntime {
    // ------------------------=
    // FUNC: new
    // DESC: Creates and initializes a new instance.
    // ------------------=
    fn new(system: SystemSnapshot) -> Self {
        let installer_date_time = firmware_date_time(system.firmware_runtime_services);
        let installer_choice = time_zone_index(installer_date_time.utc_offset_minutes);
        Self {
            mode: ConsoleMode::Startup,
            output: ConsoleOutput::new(),
            command: [0; COMMAND_CAPACITY],
            command_length: 0,
            command_cursor: 0,
            caret_visible: true,
            intent: IntentRuntime::new(),
            language_session: crate::runtime::console_language::ConsoleSession::new(),
            navigation_context: crate::runtime::object_navigation::ConsoleNavigationContext::new(
                1,
                0,
                b"/home/default",
            )
            .unwrap(),
            system,
            pointer_x: 500,
            pointer_y: 500,
            pointer_x_remainder: 0,
            preference_pointer_remainder: [0; 2],
            pointer_y_remainder: 0,
            pointer_pressed: false,
            pointer_buttons: 0,
            continuous_motion_frames: crate::ui::platform::MotionFrameCoalescer::new(),
            presenting_fast_motion_frame: false,
            installer_step: InstallerStep::Welcome,
            installer_return_step: InstallerStep::Welcome,
            storage_device: None,
            storage_plan: None,
            installer_error: None,
            installer_focus: 1,
            installer_choice,
            installer_date_time,
            installer_date_time_part: 0,
            system_step: 0,
            system_focus: 1,
            ai_chat_focus: 0,
            shell_menu: 0,
            onboarding_machine: [0; 48],
            onboarding_machine_length: 0,
            onboarding_handle: [0; 48],
            onboarding_handle_length: 0,
            onboarding_name: [0; 48],
            onboarding_name_length: 0,
            onboarding_secret: [0; 72],
            onboarding_secret_length: 0,
            current_user: crate::runtime::identity::StableId::zero(),
            current_session: crate::runtime::identity::StableId::zero(),
            settings_editing: false,
            selected_node_id: None,
            settings_window: SettingsWindowState {
                x: 160,
                y: 210,
                width: 680,
                height: 620,
                maximized: false,
                expanded_row: None,
                scroll_offset: 0,
                control_focus: 0,
                row_count: 8,
            },
            settings_window_dragging: false,
            settings_window_resizing: None,
            settings_window_drag_offset_x: 0,
            settings_window_drag_offset_y: 0,
            settings_accent_dirty: false,
            settings_primary_dirty: false,
            settings_effects_dirty: false,
            settings_effect_dragging: None,
            settings_timeout_dragging: false,
            settings_scroll_dragging: false,
            settings_scroll_grab_offset: 0,
            settings_scroll_target: 0,
            launcher_scroll_dragging: false,
            launcher_tick_ns: None,
            launcher_scroll_grab_offset: 0,
            onboarding_validation_error: false,
            home_window_x: 110,
            home_window_y: 150,
            home_window_width: 780,
            home_window_height: 660,
            home_window_visible: true,
            home_window_maximized: false,
            home_window_restore_x: 110,
            home_window_restore_y: 150,
            home_window_restore_width: 780,
            home_window_restore_height: 660,
            home_window_dragging: false,
            home_window_resizing: None,
            home_window_drag_offset_x: 0,
            home_window_drag_offset_y: 0,
            home_location: 0,
            home_previous_location: 0,
            home_selected_item: None,
            home_dragging_item: None,
            home_drag_from_desktop: false,
            home_drag_origin_x: 0,
            home_drag_origin_y: 0,
            home_drag_moved: false,
            desktop_items: 0,
            desktop_item_positions: [
                [70, 150],
                [140, 150],
                [210, 150],
                [280, 150],
                [350, 150],
                [420, 150],
                [490, 150],
            ],
            home_note_location: 0,
            home_note_previous_location: 0,
            home_clipboard_note: false,
            desktop_clock: installer_date_time,
            desktop_app: DesktopAppKind::None,
            command_window_suspended: false,
            app_window_x: 190,
            app_window_y: 160,
            app_window_width: 600,
            app_window_height: 620,
            app_window_maximized: false,
            app_window_restore_x: 190,
            app_window_restore_y: 160,
            app_window_restore_width: 600,
            app_window_restore_height: 620,
            app_window_dragging: false,
            app_window_resizing: None,
            app_window_drag_offset_x: 0,
            app_window_drag_offset_y: 0,
            editor_document: TextDocument::new(),
            editor_document_path: [0; crate::ui::text_editor::DOCUMENT_PATH_CAPACITY],
            editor_document_path_length: 0,
            editor_document_name: [0; crate::ui::text_editor::DOCUMENT_NAME_CAPACITY],
            editor_document_name_length: 0,
            editor_dialog: EditorDialog::None,
            editor_scroll_row: 0,
            editor_scroll_dragging: false,
            editor_scroll_grab_offset: 0,
            editor_window: DesktopAppWindowState::new(190, 160, 600, 620),
            command_window: DesktopAppWindowState::new(240, 210, 600, 620),
            task_manager_window: DesktopAppWindowState::new(160, 140, 760, 650),
            task_manager_selected: 0,
            task_manager_scroll: 0,
            session_idle: SessionIdleState::new(),
            locked_desktop_layout: LockedDesktopState::new(),
        }
    }

    // ------------------------=
    // FUNC: context
    // DESC: Implements the context operation.
    // ------------------=
    fn context(&self) -> IntentContext {
        IntentContext {
            console_mode: self.mode,
            architecture: core::str::from_utf8(self.system.architecture).unwrap_or("unknown"),
            known_devices: self.system.devices.len(),
            environment: "boot-console",
        }
    }

    // ------------------------=
    // FUNC: reset_input
    // DESC: Implements the reset input operation.
    // ------------------=
    fn reset_input(&mut self) {
        self.command = [0; COMMAND_CAPACITY];
        self.command_length = 0;
        self.command_cursor = 0;
    }

    // ------------------------=
    // FUNC: text_input_focused
    // DESC: Reports whether the current native surface owns an editable text caret.
    // ------------------=
    fn text_input_focused(&self) -> bool {
        match self.mode {
            ConsoleMode::Onboarding => {
                (1..=4).contains(&self.system_step) && self.system_focus == 1
            }
            ConsoleMode::Authentication | ConsoleMode::Locked => self.system_focus == 1,
            ConsoleMode::AppLauncher => self.system_focus == 0,
            ConsoleMode::Settings => self.settings_editing,
            ConsoleMode::Desktop => {
                self.ai_chat_focus == 2
                    || self.desktop_app == DesktopAppKind::CommandWindow
                    || self.desktop_app == DesktopAppKind::TextEditor
                    || self.editor_dialog == EditorDialog::SaveAs
                    || crate::runtime::with_runtime(|runtime| {
                        runtime
                            .file_navigator
                            .map(|state| state.location_editing || state.rename_editing)
                    })
                    .flatten()
                    .unwrap_or(false)
            }
            ConsoleMode::Console | ConsoleMode::Repair | ConsoleMode::Startup => true,
            _ => false,
        }
    }

    // ------------------------=
    // FUNC: pointer_over_text_input
    // DESC: Resolves text-hover affordance from the same typed hit geometry used for activation.
    // ------------------=
    fn pointer_over_text_input(&self) -> bool {
        let layout = SystemLayout::new(
            self.system.framebuffer_width,
            self.system.framebuffer_height,
        );
        match self.mode {
            ConsoleMode::Onboarding => matches!(
                layout.onboarding_target(self.system_step, self.pointer_x, self.pointer_y),
                Some(OnboardingTarget::Input)
            ),
            ConsoleMode::Authentication | ConsoleMode::Locked => {
                layout.authentication_target(self.pointer_x, self.pointer_y) == Some(1)
            }
            ConsoleMode::AppLauncher => matches!(
                layout.app_launcher_target(
                    self.pointer_x,
                    self.pointer_y,
                    launcher_visible_count(&self.command[..self.command_length])
                ),
                AppLauncherTarget::Search
            ),
            ConsoleMode::Desktop => {
                let chat = crate::runtime::ai::with_ai_runtime(|runtime| runtime.chat);
                if chat.enabled()
                    && matches!(
                        layout.ai_chat_target(self.pointer_x, self.pointer_y, chat.minimized()),
                        Some(AiChatTarget::Composer)
                    )
                {
                    return true;
                }
                if self.home_window_visible
                    && matches!(
                        self.desktop_target(layout),
                        Some(DesktopTarget::HomeLocation)
                    )
                {
                    return true;
                }
                if self.editor_dialog == EditorDialog::SaveAs {
                    return matches!(
                        layout.desktop_editor_dialog_target(
                            self.pointer_x,
                            self.pointer_y,
                            self.app_window_x,
                            self.app_window_y,
                            self.app_window_width,
                            self.app_window_height,
                            self.app_window_maximized,
                            false,
                            0,
                        ),
                        Some(EditorDialogTarget::NameField)
                    );
                }
                let window = if self.desktop_app == DesktopAppKind::TextEditor {
                    self.editor_window
                } else {
                    self.command_window
                };
                let target = layout.desktop_app_window_target(
                    self.pointer_x,
                    self.pointer_y,
                    window.x,
                    window.y,
                    window.width,
                    window.height,
                    window.maximized,
                    self.desktop_app == DesktopAppKind::TextEditor,
                );
                if target != DesktopAppWindowTarget::Content {
                    return false;
                }
                if self.desktop_app == DesktopAppKind::TextEditor {
                    return true;
                }
                let geometry = layout.desktop_app_window_geometry(
                    window.x,
                    window.y,
                    window.width,
                    window.height,
                    window.maximized,
                );
                let pointer_y = self.system.framebuffer_height as i32 * self.pointer_y / 1000;
                pointer_y >= geometry.content.bottom() - (50 * layout.scale()) as i32
            }
            ConsoleMode::Settings if self.settings_editing => matches!(
                layout.settings_target(self.pointer_x, self.pointer_y, self.settings_window),
                Some(SettingsTarget::ExpandedAction | SettingsTarget::ContentRow(_))
            ),
            _ => false,
        }
    }

    // ------------------------=
    // FUNC: publish_text_input_presentation
    // DESC: Publishes focused caret and I-beam state immediately before framebuffer composition.
    // ------------------=
    fn publish_text_input_presentation(&self) {
        let (cursor, kind) = if self.ai_chat_focus == 2 {
            (
                crate::runtime::ai::with_ai_runtime(|runtime| runtime.chat.input_cursor()),
                3,
            )
        } else if self.desktop_app == DesktopAppKind::TextEditor
            && self.editor_dialog == EditorDialog::None
        {
            (self.editor_document.cursor(), 4)
        } else {
            let file_cursor = crate::runtime::with_runtime(|runtime| {
                runtime.file_navigator.and_then(|state| {
                    (state.location_editing || state.rename_editing).then_some(state.editor_cursor)
                })
            })
            .flatten();
            file_cursor
                .map(|cursor| (cursor, 2))
                .unwrap_or((self.command_cursor, 1))
        };
        crate::ui::text_input::set_presentation(
            self.text_input_focused(),
            self.caret_visible,
            cursor,
            kind,
            self.pointer_over_text_input(),
        );
        if let Some(shape) = self.resize_pointer_shape() {
            crate::ui::text_input::set_pointer_shape(shape);
        }
    }

    // ------------------------=
    // FUNC: resize_pointer_shape
    // DESC: Resolves the directional resize pointer for the captured or hovered active window handle.
    // ------------------=
    fn resize_pointer_shape(&self) -> Option<crate::ui::text_input::PointerShape> {
        let layout = crate::ui::system_layout::SystemLayout::new(
            self.system.framebuffer_width as usize,
            self.system.framebuffer_height as usize,
        );
        let handle = match self.mode {
            ConsoleMode::Desktop => self
                .app_window_resizing
                .or(self.home_window_resizing)
                .or_else(|| {
                    let app_target = (self.desktop_app != DesktopAppKind::None)
                        .then(|| {
                            layout.desktop_app_window_target(
                                self.pointer_x,
                                self.pointer_y,
                                self.app_window_x,
                                self.app_window_y,
                                self.app_window_width,
                                self.app_window_height,
                                self.app_window_maximized,
                                self.desktop_app == DesktopAppKind::TextEditor,
                            )
                        })
                        .unwrap_or(DesktopAppWindowTarget::None);
                    match app_target {
                        DesktopAppWindowTarget::Resize(handle) if !self.app_window_maximized => {
                            Some(handle)
                        }
                        DesktopAppWindowTarget::None
                            if self.home_window_visible && !self.home_window_maximized =>
                        {
                            match self.desktop_target(layout) {
                                Some(DesktopTarget::HomeResize(handle)) => Some(handle),
                                _ => None,
                            }
                        }
                        _ => None,
                    }
                }),
            ConsoleMode::Settings => self.settings_window_resizing.or_else(|| {
                (!self.settings_window.maximized)
                    .then(|| {
                        match layout.settings_target(
                            self.pointer_x,
                            self.pointer_y,
                            self.settings_window,
                        ) {
                            Some(SettingsTarget::Resize(handle)) => Some(handle),
                            _ => None,
                        }
                    })
                    .flatten()
            }),
            _ => None,
        }?;
        crate::ui::text_input::pointer_shape_for_resize_handle(handle)
    }

    // ------------------------=
    // FUNC: clicked_caret_index
    // DESC: Converts the current normalized pointer into a nearest insertion point inside a rendered field.
    // ------------------=
    fn clicked_caret_index(
        &self,
        field: crate::ui::geometry::Rect,
        text_inset: usize,
        length: usize,
    ) -> usize {
        let pointer_x = self.system.framebuffer_width as i32 * self.pointer_x / 1000;
        let scale = (self.system.framebuffer_width as usize / 1000).max(1);
        crate::ui::text_input::caret_from_x(
            pointer_x - field.x - text_inset as i32,
            9 * scale,
            length,
        )
    }

    // ------------------------=
    // FUNC: show_startup
    // DESC: Renders show startup to the active display.
    // ------------------=
    fn show_startup(&mut self) {
        self.mode = ConsoleMode::Startup;
        self.reset_input();
        self.output.clear();
        self.output.write_line(b".___ _______  ___________.___ _______  .___________________.___.________    _________");
        self.output.write_line(b"|   |\\      \\ \\_   _____/|   |\\      \\ |   \\__    ___/\\__  |   |\\_____  \\  /   _____/");
        self.output.write_line(b"|   |/   |   \\ |    __)  |   |/   |   \\|   | |    |    /   |   | /   |   \\ \\_____  \\");
        self.output.write_line(b"|   /    |    \\|     \\   |   /    |    \\   | |    |    \\____   |/    |    \\/        \\");
        self.output.write_line(b"|___\\____|__  /\\___  /   |___\\____|__  /___| |____|    / ______|\\_______  /_______  /");
        self.output.write_line(b"            \\/     \\/                \\/                \\/               \\/        \\/");
        self.pointer_x = 500;
        // Start away from actionable rows. VirtualBox's first click may only
        // capture its relative USB mouse; it must not also launch an option.
        self.pointer_y = 500;
        self.pointer_x_remainder = 0;
        self.pointer_y_remainder = 0;
        self.pointer_pressed = false;
        self.pointer_buttons = 0;
        crate::output_text(b" --[ NODE 01 ]-- SYSTEM ONLINE -- SELECT OPERATION -->\n");
        crate::output_text(
            b"[mouse menu] Install InfinityOS | Repair installation | Recovery console\n",
        );
        crate::output_text(b"[ui] startup selection\n");
    }

    // ------------------------=
    // FUNC: enter_console
    // DESC: Implements the enter console operation.
    // ------------------=
    fn enter_console(&mut self) {
        if !self.current_session.is_zero() {
            self.open_command_window();
            return;
        }
        self.mode = ConsoleMode::Console;
        self.reset_input();
        self.output.clear();
        self.output.write_line(b"Infinity Console");
        self.output
            .write_line(b"Type help or describe what you want.");
        crate::output_text(b"[ui] infinity console\n");
    }

    // ------------------------=
    // FUNC: enter_repair
    // DESC: Implements the enter repair operation.
    // ------------------=
    fn enter_repair(&mut self) {
        self.mode = ConsoleMode::Repair;
        self.reset_input();
        self.output.clear();
        self.output.write_line(b"InfinityOS Repair Environment");
        self.output
            .write_line(b"Storage changes are disabled until explicitly confirmed.");
        self.output
            .write_line(b"Type help to inspect devices and system status.");
        crate::output_text(b"[repair] recovery console started\n");
    }

    // ------------------------=
    // FUNC: show_installer
    // DESC: Renders show installer to the active display.
    // ------------------=
    fn show_installer(&mut self) {
        self.mode = ConsoleMode::Installer;
        self.reset_input();
        self.installer_step = InstallerStep::Welcome;
        self.installer_return_step = InstallerStep::Welcome;
        self.storage_device = None;
        self.storage_plan = None;
        self.installer_error = None;
        self.installer_date_time = firmware_date_time(self.system.firmware_runtime_services);
        self.installer_choice = time_zone_index(self.installer_date_time.utc_offset_minutes);
        self.installer_date_time_part = 0;
        self.installer_focus = if self.installer_has_primary() { 1 } else { 0 };
        self.render_installer();
        crate::output_text(b"[installer] wizard started\n");
    }

    // ------------------------=
    // FUNC: prompt
    // DESC: Implements the prompt operation.
    // ------------------=
    fn prompt(&self) -> &'static [u8] {
        if self.mode == ConsoleMode::Desktop && self.desktop_app == DesktopAppKind::CommandWindow {
            return b"inf > ";
        }
        match self.mode {
            // The startup surface draws its infinity mark directly because
            // the compact bitmap font is intentionally ASCII-only.
            ConsoleMode::Startup => b"",
            ConsoleMode::Console => b"inf > ",
            ConsoleMode::Repair => b"repair > ",
            ConsoleMode::Installer => b"",
            ConsoleMode::Onboarding
            | ConsoleMode::Desktop
            | ConsoleMode::AppLauncher
            | ConsoleMode::SystemMenu
            | ConsoleMode::Settings
            | ConsoleMode::Authentication
            | ConsoleMode::Locked => b"",
        }
    }

    // ------------------------=
    // FUNC: redraw
    // DESC: Implements the redraw operation.
    // ------------------=
    fn redraw(&self) {
        self.publish_text_input_presentation();
        if matches!(
            self.mode,
            ConsoleMode::Onboarding
                | ConsoleMode::Desktop
                | ConsoleMode::AppLauncher
                | ConsoleMode::SystemMenu
                | ConsoleMode::Settings
                | ConsoleMode::Authentication
                | ConsoleMode::Locked
        ) {
            let screen = match self.mode {
                ConsoleMode::Onboarding => 1,
                ConsoleMode::Desktop => match self.desktop_app {
                    DesktopAppKind::CommandWindow => 8,
                    DesktopAppKind::TextEditor => 9,
                    DesktopAppKind::TaskManager => 10,
                    DesktopAppKind::None => 2,
                },
                ConsoleMode::AppLauncher => 7,
                ConsoleMode::SystemMenu => 3,
                ConsoleMode::Settings => 4,
                ConsoleMode::Authentication => 5,
                ConsoleMode::Locked => 6,
                _ => 0,
            };
            let mut settings_value = [0u8; 48];
            let mut settings_value_length = 0usize;
            if self.mode == ConsoleMode::Settings
                && !self.settings_editing
                && !matches!(self.system_focus, 6 | 7)
            {
                if self.system_focus == 4 {
                    let minutes = self.user_no_activity_timeout_minutes();
                    settings_value_length = write_minutes_label(&mut settings_value, minutes);
                } else if let Some(machine) =
                    crate::runtime::with_runtime(|runtime| runtime.identity.machine()).flatten()
                {
                    settings_value_length = machine.display_name.as_bytes().len();
                    settings_value[..settings_value_length]
                        .copy_from_slice(machine.display_name.as_bytes());
                }
            }
            let displayed_input = if self.desktop_app == DesktopAppKind::TextEditor {
                self.editor_document.bytes()
            } else if self.mode == ConsoleMode::Settings && !self.settings_editing {
                &settings_value[..settings_value_length]
            } else {
                &self.command[..self.command_length]
            };
            let (editor_window, command_window, task_manager_window) = self.desktop_app_windows();
            crate::bootstrap::system_ui_present(
                screen,
                self.system_step,
                displayed_input,
                self.mode == ConsoleMode::Locked
                    || self.mode == ConsoleMode::Authentication
                    || (self.mode == ConsoleMode::Onboarding
                        && crate::ui::installer_layout::configuration_template_input_variable(
                            self.system_step,
                        ) == crate::ui::installer_template::InstallerTemplateVariable::Password),
                self.system_focus,
                self.pointer_x,
                self.pointer_y,
                self.onboarding_validation_error,
                self.home_window_x,
                self.home_window_y,
                self.home_window_width,
                self.home_window_height,
                self.home_window_visible,
                self.home_window_maximized,
                self.home_location,
                self.home_selected_item,
                self.home_dragging_item,
                self.home_note_location,
                self.desktop_items,
                &self.desktop_item_positions,
                self.desktop_clock,
                self.settings_window,
                self.shell_menu,
                &self.output.lines,
                &self.output.lengths,
                self.output.count,
                self.app_window_x,
                self.app_window_y,
                self.app_window_width,
                self.app_window_height,
                self.app_window_maximized,
                self.editor_document.is_saved(),
                self.editor_document.bytes(),
                &self.command[..self.command_length],
                editor_window,
                command_window,
                task_manager_window,
                self.editor_scroll_row,
                self.editor_dialog as u8,
                &self.command[..self.command_length],
                self.system_focus,
                self.presenting_fast_motion_frame,
            );
            return;
        }
        crate::bootstrap::console_present(
            &self.output.lines,
            &self.output.lengths,
            self.output.count,
            self.prompt(),
            &self.command[..self.command_length],
            self.mode == ConsoleMode::Startup,
            if self.mode == ConsoleMode::Installer {
                self.installer_screen()
            } else {
                0
            },
            self.installer_focus,
            self.installer_choice,
            self.installer_date_time,
            self.installer_date_time_part,
            self.mode == ConsoleMode::Installer && self.installer_has_primary(),
            self.storage_device,
            self.pointer_x,
            self.pointer_y,
            self.pointer_pressed,
        );
    }

    // ------------------------=
    // FUNC: input
    // DESC: Implements the input operation.
    // ------------------=
    fn input(&mut self, key: ConsoleKey) {
        self.session_idle.note_activity();
        self.caret_visible = true;
        if self.mode == ConsoleMode::Desktop {
            if self.ai_chat_focus != 0 && self.input_ai_chat(key) {
                self.redraw();
                return;
            }
            if self.desktop_app == DesktopAppKind::CommandWindow {
                self.input_console(key);
                self.redraw();
                return;
            }
            if self.desktop_app == DesktopAppKind::TextEditor {
                self.input_text_editor(key);
                self.redraw();
                return;
            }
            if self.desktop_app == DesktopAppKind::TaskManager {
                self.input_task_manager(key);
                self.redraw();
                return;
            }
            if self.home_window_visible && self.input_file_navigator(key) {
                self.redraw();
                return;
            }
        }
        match self.mode {
            ConsoleMode::Installer => self.input_installer(key),
            ConsoleMode::Startup => self.input_startup(key),
            ConsoleMode::Console | ConsoleMode::Repair => self.input_console(key),
            ConsoleMode::Onboarding => self.input_onboarding(key),
            ConsoleMode::Desktop
            | ConsoleMode::AppLauncher
            | ConsoleMode::SystemMenu
            | ConsoleMode::Settings => self.input_shell(key),
            ConsoleMode::Authentication | ConsoleMode::Locked => self.input_authentication(key),
        }
        self.redraw();
    }

    // ------------------------=
    // FUNC: input_ai_chat
    // DESC: Handles keyboard navigation, model selection, and bounded chat composer editing.
    // ------------------=
    fn input_ai_chat(&mut self, key: ConsoleKey) -> bool {
        match key {
            ConsoleKey::Character(_)
            | ConsoleKey::Backspace
            | ConsoleKey::Delete
            | ConsoleKey::Left
            | ConsoleKey::Right
            | ConsoleKey::Home
            | ConsoleKey::End
                if self.ai_chat_focus == 2 =>
            {
                if let Some(edit_key) = text_edit_key(key) {
                    crate::runtime::ai::with_ai_runtime(|runtime| {
                        runtime.chat.edit_input(edit_key)
                    });
                }
            }
            ConsoleKey::Tab(reverse) => {
                self.ai_chat_focus = if reverse {
                    if self.ai_chat_focus <= 1 {
                        5
                    } else {
                        self.ai_chat_focus - 1
                    }
                } else if self.ai_chat_focus >= 5 {
                    1
                } else {
                    self.ai_chat_focus + 1
                };
            }
            ConsoleKey::Enter => match self.ai_chat_focus {
                1 => self.select_next_chat_model(),
                2 | 3 => {
                    self.submit_ai_chat_input();
                    self.ai_chat_focus = 2;
                }
                4 => crate::runtime::ai::with_ai_runtime(|runtime| {
                    runtime.chat.set_minimized(!runtime.chat.minimized())
                }),
                5 => self.set_ai_chat_enabled(false),
                _ => {}
            },
            ConsoleKey::Escape => self.ai_chat_focus = 0,
            _ => {}
        }
        true
    }

    // ------------------------=
    // FUNC: input_task_manager
    // DESC: Handles live table selection and full keyboard lifecycle/resource actions.
    // ------------------=
    fn input_task_manager(&mut self, key: ConsoleKey) {
        let count = crate::runtime::with_runtime(|runtime| {
            runtime.task_manager.task_count(&runtime.execution)
        })
        .unwrap_or(0);
        match key {
            ConsoleKey::Up => {
                self.task_manager_selected = self.task_manager_selected.saturating_sub(1)
            }
            ConsoleKey::Down => {
                self.task_manager_selected =
                    (self.task_manager_selected + 1).min(count.saturating_sub(1))
            }
            ConsoleKey::Delete => self.task_manager_action(0),
            ConsoleKey::Character(b'r' | b'R') => self.task_manager_action(1),
            ConsoleKey::Character(b' ') | ConsoleKey::Enter => self.task_manager_action(2),
            ConsoleKey::Character(b't' | b'T') => self.task_manager_action(3),
            ConsoleKey::Character(b'l' | b'L') => {
                let _ = self.open_file_navigator_window(b"/home/default");
            }
            ConsoleKey::Escape if self.shell_menu == 10 => self.shell_menu = 0,
            ConsoleKey::Escape => self.close_desktop_app(),
            _ => {}
        }
        self.refresh_task_manager_output();
    }

    // ------------------------=
    // FUNC: task_manager_action
    // DESC: Applies one selected-row end, relaunch, pause/resume, or throttle transition.
    // ------------------=
    fn task_manager_action(&mut self, action: u8) {
        let selected = self.task_manager_selected;
        let task = crate::runtime::with_runtime(|runtime| {
            runtime.task_manager.task_nth(&runtime.execution, selected)
        })
        .flatten();
        let Some(task) = task else { return };
        let changed = crate::runtime::with_runtime(|runtime| match action {
            0 => runtime
                .task_manager
                .end(&mut runtime.execution, task.handle)
                .ok(),
            1 => runtime
                .task_manager
                .relaunch(&mut runtime.execution, task.handle)
                .ok(),
            2 if task.state == crate::runtime::execution::ContextState::Waiting => runtime
                .task_manager
                .resume(&mut runtime.execution, task.handle)
                .ok(),
            2 => runtime
                .task_manager
                .pause(&mut runtime.execution, task.handle)
                .ok(),
            _ => {
                let mut budget = task.budget;
                budget.cpu_weight = match budget.cpu_weight {
                    1..=24 => 100,
                    25..=99 => 25,
                    _ => 50,
                };
                runtime
                    .task_manager
                    .throttle(
                        &mut runtime.execution,
                        task.handle,
                        budget,
                        crate::runtime::execution::PriorityClass::Background,
                    )
                    .ok()
            }
        })
        .flatten()
        .is_some();
        if action == 0 && changed {
            self.close_task_surface(task);
        } else if action == 1 && changed {
            self.reopen_task_surface(task);
        }
    }

    // ------------------------=
    // FUNC: close_task_surface
    // DESC: Removes the GUI surface owned by a successfully ended application context.
    // ------------------=
    fn close_task_surface(&mut self, task: crate::runtime::task_manager::TaskSnapshot) {
        match task.image_identity {
            crate::runtime::task_manager::IMAGE_FILE_NAVIGATOR => {
                let next = crate::runtime::with_runtime(|runtime| {
                    runtime.file_navigators.close_task(task.handle.0);
                    runtime.file_navigators.active_index()
                })
                .flatten();
                if let Some(index) = next {
                    let _ = self.load_file_navigator_window(index);
                } else {
                    self.home_window_visible = false;
                }
            }
            crate::runtime::task_manager::IMAGE_TEXT_EDITOR => self.editor_window.visible = false,
            crate::runtime::task_manager::IMAGE_COMMAND_WINDOW => {
                self.command_window.visible = false
            }
            crate::runtime::task_manager::IMAGE_TASK_MANAGER => {
                self.task_manager_window.visible = false
            }
            _ => {}
        }
    }

    // ------------------------=
    // FUNC: reopen_task_surface
    // DESC: Restores the visible GUI surface for a successfully relaunched application context.
    // ------------------=
    fn reopen_task_surface(&mut self, task: crate::runtime::task_manager::TaskSnapshot) {
        match task.image_identity {
            crate::runtime::task_manager::IMAGE_FILE_NAVIGATOR => {
                let index = crate::runtime::with_runtime(|runtime| {
                    runtime
                        .file_navigators
                        .launch(b"/home/default", task.handle.0)
                        .ok()
                })
                .flatten();
                if let Some(index) = index {
                    let _ = self.load_file_navigator_window(index);
                }
            }
            crate::runtime::task_manager::IMAGE_TEXT_EDITOR => self.open_text_editor(),
            crate::runtime::task_manager::IMAGE_COMMAND_WINDOW => self.open_command_window(),
            crate::runtime::task_manager::IMAGE_TASK_MANAGER => self.open_task_manager(),
            _ => {}
        }
    }

    // ------------------------=
    // FUNC: activate_task_manager_pointer
    // DESC: Hit-tests the Task menu and visible process rows using shared live window geometry.
    // ------------------=
    fn activate_task_manager_pointer(&mut self) -> bool {
        let layout = SystemLayout::new(
            self.system.framebuffer_width,
            self.system.framebuffer_height,
        );
        let scale = layout.scale().max(1);
        let geometry = layout.desktop_app_window_geometry(
            self.app_window_x,
            self.app_window_y,
            self.app_window_width,
            self.app_window_height,
            self.app_window_maximized,
        );
        let point_x = self.system.framebuffer_width as i32 * self.pointer_x / 1000;
        let point_y = self.system.framebuffer_height as i32 * self.pointer_y / 1000;
        if !geometry.window.contains(crate::ui::geometry::Point {
            x: point_x,
            y: point_y,
        }) {
            return false;
        }
        let toolbar_left = geometry.toolbar.x;
        let toolbar_top = geometry.toolbar.y;
        // Window chrome must never be consumed as a process-row click.
        let point = crate::ui::geometry::Point {
            x: point_x,
            y: point_y,
        };
        if !geometry.toolbar.contains(point) && !geometry.content.contains(point) {
            return false;
        }
        let menu_width = 232 * scale as i32;
        if point_y >= toolbar_top
            && point_y < toolbar_top + geometry.toolbar.height as i32
            && point_x >= toolbar_left + 10 * scale as i32
            && point_x < toolbar_left + 104 * scale as i32
        {
            self.shell_menu = if self.shell_menu == 10 { 0 } else { 10 };
            self.refresh_task_manager_output();
            return true;
        }
        if self.shell_menu == 10 {
            let menu_top = toolbar_top + geometry.toolbar.height as i32 - 2 * scale as i32;
            if point_x >= toolbar_left + 10 * scale as i32
                && point_x < toolbar_left + 10 * scale as i32 + menu_width
                && point_y >= menu_top
                && point_y < menu_top + 214 * scale as i32
            {
                let action = ((point_y - menu_top - 8 * scale as i32) / (32 * scale) as i32)
                    .clamp(0, 5) as u8;
                self.shell_menu = 0;
                match action {
                    0 => {
                        let _ = self.open_file_navigator_window(b"/home/default");
                    }
                    1 => self.task_manager_action(1),
                    2 => self.task_manager_action(2),
                    3 => self.task_manager_action(3),
                    4 => self.task_manager_action(0),
                    _ => self.refresh_task_manager_output(),
                }
                return true;
            }
            self.shell_menu = 0;
        }
        if let Some(row) = layout.task_manager_process_row(self.pointer_x, self.pointer_y, geometry)
        {
            self.task_manager_selected = self.task_manager_scroll + row;
            self.refresh_task_manager_output();
            return true;
        }
        false
    }

    // ------------------------=
    // FUNC: activate_native_app_performance_pointer
    // DESC: Opens and executes the uniform Performance menu at the same header position in every native app.
    // ------------------=
    fn activate_native_app_performance_pointer(&mut self) -> bool {
        let layout = SystemLayout::new(
            self.system.framebuffer_width,
            self.system.framebuffer_height,
        );
        let scale = layout.scale().max(1) as i32;
        let geometry = layout.desktop_app_window_geometry(
            self.app_window_x,
            self.app_window_y,
            self.app_window_width,
            self.app_window_height,
            self.app_window_maximized,
        );
        let point_x = self.system.framebuffer_width as i32 * self.pointer_x / 1000;
        let point_y = self.system.framebuffer_height as i32 * self.pointer_y / 1000;
        let performance_left = geometry.window.x + 220 * scale;
        if point_y >= geometry.window.y
            && point_y < geometry.window.y + 42 * scale
            && point_x >= performance_left
            && point_x < performance_left + 110 * scale
        {
            self.shell_menu = if self.shell_menu == 20 { 0 } else { 20 };
            return true;
        }
        if self.shell_menu != 20 {
            return false;
        }
        let menu_top = geometry.window.y + 42 * scale;
        if point_x >= performance_left
            && point_x < performance_left + 232 * scale
            && point_y >= menu_top
            && point_y < menu_top + 140 * scale
        {
            let row = ((point_y - menu_top - 6 * scale) / (30 * scale)).clamp(0, 3);
            self.shell_menu = 0;
            let image = match self.desktop_app {
                DesktopAppKind::TextEditor => crate::runtime::task_manager::IMAGE_TEXT_EDITOR,
                DesktopAppKind::CommandWindow => crate::runtime::task_manager::IMAGE_COMMAND_WINDOW,
                DesktopAppKind::TaskManager => crate::runtime::task_manager::IMAGE_TASK_MANAGER,
                DesktopAppKind::None => return true,
            };
            match row {
                0 => self.apply_application_resource_mode(
                    image,
                    crate::runtime::resource_policy::ResourceMode::Restricted,
                ),
                1 => self.apply_application_resource_mode(
                    image,
                    crate::runtime::resource_policy::ResourceMode::Balanced,
                ),
                2 => self.apply_application_resource_mode(
                    image,
                    crate::runtime::resource_policy::ResourceMode::Expanded,
                ),
                _ => self.open_settings(0),
            }
            return true;
        }
        self.shell_menu = 0;
        false
    }

    // ------------------------=
    // FUNC: refresh_task_manager_output
    // DESC: Rebuilds the bounded live graphical task table from authoritative snapshots.
    // ------------------=
    fn refresh_task_manager_output(&mut self) {
        self.output.clear();
        self.output
            .write_line(b"ID  TASK                 STATE       CPU WEIGHT");
        let selected = self.task_manager_selected;
        crate::runtime::with_runtime(|runtime| {
            runtime.task_manager.sample_cpu(&runtime.execution);
            let count = runtime.task_manager.task_count(&runtime.execution);
            self.task_manager_selected = selected.min(count.saturating_sub(1));
            self.task_manager_scroll = self.task_manager_selected.saturating_sub(3);
            self.system_focus = self.task_manager_selected;
            for index in self.task_manager_scroll..count.min(self.task_manager_scroll + 5) {
                if let Some(task) = runtime.task_manager.task_nth(&runtime.execution, index) {
                    let id = number_pair(task.handle.0 as usize);
                    self.output.write_segments(&[
                        if index == self.task_manager_selected {
                            b"> ".as_slice()
                        } else {
                            b"  "
                        },
                        &id,
                        b"  ",
                        task_name(task.service_identity, task.image_identity),
                        b"  ",
                        context_state_text(task.state),
                    ]);
                }
            }
        });
    }

    // ------------------------=
    // FUNC: input_file_navigator
    // DESC: Handles native File Navigator location editing, inline rename, selection, and keyboard navigation.
    // ------------------=
    fn input_file_navigator(&mut self, key: ConsoleKey) -> bool {
        let state = crate::runtime::with_runtime(|runtime| runtime.file_navigator).flatten();
        let Some(state) = state else {
            return false;
        };
        if state.location_editing || state.rename_editing {
            match key {
                ConsoleKey::Character(_)
                | ConsoleKey::Backspace
                | ConsoleKey::Delete
                | ConsoleKey::Left
                | ConsoleKey::Right
                | ConsoleKey::Home
                | ConsoleKey::End => {
                    let _ = crate::runtime::with_runtime(|runtime| {
                        runtime.file_navigator.as_mut().map(|navigator| {
                            text_edit_key(key).map(|edit_key| {
                                navigator
                                    .editor_text
                                    .edit(&mut navigator.editor_cursor, edit_key)
                            })
                        })
                    });
                }
                ConsoleKey::Escape => {
                    let _ = crate::runtime::with_runtime(|runtime| {
                        runtime
                            .file_navigator
                            .as_mut()
                            .map(|navigator| navigator.cancel_edit())
                    });
                }
                ConsoleKey::Enter => self.commit_file_navigator_edit(state),
                _ => {}
            }
            return true;
        }
        if let Some(menu) = state.menu_open {
            match key {
                ConsoleKey::Escape => {
                    let _ = crate::runtime::with_runtime(|runtime| {
                        runtime
                            .file_navigator
                            .as_mut()
                            .map(|navigator| navigator.close_overlays())
                    });
                }
                ConsoleKey::Left | ConsoleKey::Right => {
                    let current = menu.index();
                    let next = if matches!(key, ConsoleKey::Left) {
                        if current == 1 {
                            4
                        } else {
                            current - 1
                        }
                    } else if current == 4 {
                        1
                    } else {
                        current + 1
                    };
                    let _ = crate::runtime::with_runtime(|runtime| {
                        runtime.file_navigator.as_mut().map(|navigator| {
                            navigator.open_menu(
                                crate::runtime::object_navigation::FileNavigatorMenu::from_index(
                                    next,
                                )
                                .unwrap(),
                            )
                        })
                    });
                }
                ConsoleKey::Up | ConsoleKey::Down => {
                    let count = menu.item_count();
                    let _ = crate::runtime::with_runtime(|runtime| {
                        runtime.file_navigator.as_mut().map(|navigator| {
                            let selected = navigator.menu_selection as usize;
                            navigator.menu_selection = if matches!(key, ConsoleKey::Up) {
                                if selected == 0 {
                                    count - 1
                                } else {
                                    selected - 1
                                }
                            } else {
                                (selected + 1) % count
                            } as u8;
                        })
                    });
                }
                ConsoleKey::Enter => {
                    self.activate_file_navigator_menu(menu, state.menu_selection as usize);
                }
                _ => {}
            }
            return true;
        }
        if let Some(dialog) = state.dialog_open {
            match key {
                ConsoleKey::Escape => {
                    let _ = crate::runtime::with_runtime(|runtime| {
                        runtime
                            .file_navigator
                            .as_mut()
                            .map(|navigator| navigator.close_overlays())
                    });
                }
                ConsoleKey::Enter
                    if matches!(
                        dialog,
                        crate::runtime::object_navigation::FileNavigatorDialog::About
                            | crate::runtime::object_navigation::FileNavigatorDialog::Help
                    ) =>
                {
                    let _ = crate::runtime::with_runtime(|runtime| {
                        runtime
                            .file_navigator
                            .as_mut()
                            .map(|navigator| navigator.close_overlays())
                    });
                }
                _ => {}
            }
            return true;
        }
        match key {
            ConsoleKey::Up | ConsoleKey::Down => {
                let count = navigator_child_count(state.active_namespace_ref.as_bytes());
                let _ = crate::runtime::with_runtime(|runtime| {
                    runtime.file_navigator.as_mut().map(|navigator| {
                        navigator.move_selection(count, matches!(key, ConsoleKey::Up))
                    })
                });
                true
            }
            ConsoleKey::Enter => {
                self.open_file_navigator_selection();
                true
            }
            ConsoleKey::Backspace => {
                self.navigate_file_navigator_parent();
                true
            }
            ConsoleKey::Left => {
                let _ = crate::runtime::with_runtime(|runtime| {
                    runtime
                        .file_navigator
                        .as_mut()
                        .map(|navigator| navigator.back())
                });
                true
            }
            ConsoleKey::Right => {
                let _ = crate::runtime::with_runtime(|runtime| {
                    runtime
                        .file_navigator
                        .as_mut()
                        .map(|navigator| navigator.forward())
                });
                true
            }
            ConsoleKey::Escape if state.context_menu_open => {
                let _ = crate::runtime::with_runtime(|runtime| {
                    runtime
                        .file_navigator
                        .as_mut()
                        .map(|navigator| navigator.context_menu_open = false)
                });
                true
            }
            _ => false,
        }
    }

    // ------------------------=
    // FUNC: commit_file_navigator_edit
    // DESC: Resolves an edited location or atomically renames the selected NamespaceRef.
    // ------------------=
    fn commit_file_navigator_edit(
        &mut self,
        state: crate::runtime::object_navigation::FileNavigatorState,
    ) {
        if state.location_editing {
            let path = state.editor_text.as_bytes();
            if crate::storage::namespace_resolve(path).is_ok() {
                let _ = crate::runtime::with_runtime(|runtime| {
                    runtime.file_navigator.as_mut().map(|navigator| {
                        let result = navigator.navigate(path);
                        navigator.cancel_edit();
                        navigator.dialog_open = None;
                        result
                    })
                });
            }
            return;
        }
        if state.rename_editing {
            if let Some(entry) = navigator_child_nth(
                state.active_namespace_ref.as_bytes(),
                state.selected_index as usize,
            ) {
                if let Ok(destination) = crate::runtime::object_navigation::namespace_child_path(
                    state.active_namespace_ref.as_bytes(),
                    state.editor_text.as_bytes(),
                ) {
                    let _ = crate::storage::namespace_move(
                        &entry.path[..entry.path_len as usize],
                        destination.as_bytes(),
                    );
                }
            }
            let _ = crate::runtime::with_runtime(|runtime| {
                runtime.file_navigator.as_mut().map(|navigator| {
                    navigator.cancel_edit();
                    navigator.dialog_open = None;
                })
            });
        }
    }

    // ------------------------=
    // FUNC: navigate_file_navigator_parent
    // DESC: Navigates the File Navigator to its explicit parent NamespaceRef.
    // ------------------=
    fn navigate_file_navigator_parent(&mut self) {
        let _ = crate::runtime::with_runtime(|runtime| {
            let navigator = runtime.file_navigator.as_mut()?;
            let parent = crate::runtime::object_navigation::parent_path(
                navigator.active_namespace_ref.as_bytes(),
            )
            .ok()?;
            navigator.navigate(parent.as_bytes()).ok()
        });
    }

    // ------------------------=
    // FUNC: open_file_navigator_selection
    // DESC: Opens a selected namespace node or loads a selected UTF-8 object in Text Editor.
    // ------------------=
    fn open_file_navigator_selection(&mut self) {
        let state = crate::runtime::with_runtime(|runtime| runtime.file_navigator).flatten();
        let Some(state) = state else {
            return;
        };
        if (state.selected_index as usize)
            < crate::runtime::object_navigation::FILE_NAVIGATOR_NAVIGATION_ENTRY_COUNT
        {
            let _ = crate::runtime::with_runtime(|runtime| {
                runtime.file_navigator.as_mut().map(|navigator| {
                    navigator.navigate_navigation_entry(state.selected_index as usize)
                })
            });
            return;
        }
        let Some(entry) = navigator_child_nth(
            state.active_namespace_ref.as_bytes(),
            state.selected_index as usize,
        ) else {
            return;
        };
        let path = &entry.path[..entry.path_len as usize];
        if let Ok((metadata, _)) = crate::storage::object_inspect_path(path) {
            if metadata.kind == crate::storage::object::ObjectType::NamespaceNode {
                let _ = crate::runtime::with_runtime(|runtime| {
                    runtime
                        .file_navigator
                        .as_mut()
                        .map(|navigator| navigator.navigate(path))
                });
            } else if metadata.content_type == crate::storage::object::ContentType::Utf8Text {
                self.open_text_editor_path(path);
            }
        }
    }

    // ------------------------=
    // FUNC: open_text_editor_path
    // DESC: Loads one selected UTF-8 NamespaceRef into the native Text Editor window.
    // ------------------=
    fn open_text_editor_path(&mut self, path: &[u8]) {
        let mut content = [0u8; crate::ui::text_editor::DOCUMENT_CAPACITY];
        let Ok((_, length)) = crate::storage::object_read_path(path, None, &mut content) else {
            return;
        };
        if !self.editor_document.open(&content[..length]) {
            return;
        }
        let path_length = path.len().min(self.editor_document_path.len());
        self.editor_document_path[..path_length].copy_from_slice(&path[..path_length]);
        self.editor_document_path_length = path_length;
        let name = crate::runtime::object_navigation::namespace_basename(path);
        let name_length = name.len().min(self.editor_document_name.len());
        self.editor_document_name[..name_length].copy_from_slice(&name[..name_length]);
        self.editor_document_name_length = name_length;
        self.editor_scroll_row = 0;
        self.open_text_editor();
    }

    // ------------------------=
    // FUNC: input_text_editor
    // DESC: Applies bounded multiline editing and native close keyboard behavior.
    // ------------------=
    fn input_text_editor(&mut self, key: ConsoleKey) {
        if self.editor_dialog != EditorDialog::None {
            if self.editor_dialog == EditorDialog::SaveAs && self.edit_system_text(key) {
                return;
            }
            match (self.editor_dialog, key) {
                (EditorDialog::SaveAs, ConsoleKey::Enter) => self.save_editor_document_as(),
                (EditorDialog::Open, ConsoleKey::Up) => {
                    self.system_focus = self.system_focus.saturating_sub(1)
                }
                (EditorDialog::Open, ConsoleKey::Down) => {
                    let count = self.editor_document_count();
                    self.system_focus = self
                        .system_focus
                        .saturating_add(1)
                        .min(count.saturating_sub(1));
                }
                (EditorDialog::Open, ConsoleKey::Enter) => self.open_selected_editor_document(),
                (_, ConsoleKey::Escape) => self.close_editor_dialog(),
                _ => {}
            }
            return;
        }
        match key {
            ConsoleKey::Character(character) if (32..=126).contains(&character) => {
                let _ = self.editor_document.insert(character);
                self.editor_scroll_row = self.editor_scroll_geometry().maximum_scroll;
            }
            ConsoleKey::Enter => {
                let _ = self.editor_document.insert(b'\n');
                self.editor_scroll_row = self.editor_scroll_geometry().maximum_scroll;
            }
            ConsoleKey::Backspace => {
                let _ = self.editor_document.backspace();
                self.editor_scroll_row = self.editor_scroll_geometry().maximum_scroll;
            }
            ConsoleKey::Delete => {
                let _ = self.editor_document.delete();
            }
            ConsoleKey::Left => {
                let _ = self.editor_document.move_cursor(-1);
            }
            ConsoleKey::Right => {
                let _ = self.editor_document.move_cursor(1);
            }
            ConsoleKey::Home => {
                let _ = self.editor_document.move_cursor_to_line_edge(false);
            }
            ConsoleKey::End => {
                let _ = self.editor_document.move_cursor_to_line_edge(true);
            }
            ConsoleKey::Up => {
                let _ = self.editor_document.move_cursor_vertical(true);
            }
            ConsoleKey::Down => {
                let _ = self.editor_document.move_cursor_vertical(false);
            }
            ConsoleKey::Escape => self.close_desktop_app(),
            _ => {}
        }
    }

    // ------------------------=
    // FUNC: enter_onboarding
    // DESC: Starts or resumes the durable first-boot identity workflow.
    // ------------------=
    fn enter_onboarding(&mut self) {
        self.mode = ConsoleMode::Onboarding;
        self.system_step = 0;
        self.system_focus = 1;
        self.reset_input();
        crate::runtime::with_runtime(|runtime| {
            let _ = runtime.identity.begin_onboarding();
            if runtime.identity.machine().is_some() {
                self.system_step = 2;
            }
            if let Some(user) = runtime.identity.user_nth(0) {
                self.current_user = user.id;
                // A restarted first boot asks for the existing credential again before
                // completing. This reconstructs authority without persisting a password
                // or relying on volatile wizard input from the interrupted session.
                self.system_step = 4;
            }
        });
        let _ = crate::runtime::persist_identity_state();
        crate::output_text(b"[onboarding] first-boot experience started\n");
    }

    // ------------------------=
    // FUNC: enter_desktop
    // DESC: Enters the minimal authenticated graphical shell.
    // ------------------=
    fn enter_desktop(&mut self) {
        self.mode = ConsoleMode::Desktop;
        self.desktop_app = DesktopAppKind::None;
        self.system_focus = 0;
        self.ai_chat_focus = 0;
        self.shell_menu = 0;
        self.settings_editing = false;
        self.settings_accent_dirty = false;
        self.settings_primary_dirty = false;
        self.settings_effects_dirty = false;
        self.settings_effect_dragging = None;
        self.settings_timeout_dragging = false;
        self.settings_scroll_dragging = false;
        self.settings_window_dragging = false;
        self.settings_window_resizing = None;
        self.home_window_dragging = false;
        self.home_window_resizing = None;
        self.app_window_dragging = false;
        self.app_window_resizing = None;
        self.home_dragging_item = None;
        self.sync_icon_theme();
        self.sync_accent();
        self.sync_primary();
        self.sync_background_effects();
        self.sync_ai_chat_preferences();
        self.refresh_desktop_items();
        self.reset_input();
        crate::output_text(b"[shell] top bar ready\n[shell] Infinity menu ready\n[settings] graphical settings ready\n");
    }

    // ------------------------=
    // FUNC: sync_ai_chat_preferences
    // DESC: Applies the authenticated user's persistent chat visibility and model choice.
    // ------------------=
    fn sync_ai_chat_preferences(&mut self) {
        let preferences =
            crate::runtime::with_runtime(|runtime| runtime.identity.ai_profile(self.current_user))
                .flatten();
        if let Some(profile) = preferences {
            let memory = crate::runtime::with_runtime(|runtime| {
                runtime
                    .identity
                    .read_ai_memory(self.current_user, self.current_user)
                    .ok()
            })
            .flatten();
            crate::runtime::ai::with_ai_runtime(|runtime| {
                runtime.chat.set_enabled(profile.chat_enabled);
                runtime
                    .chat
                    .select_model_index(profile.chat_model_index as usize);
                if let Some(memory) = memory {
                    runtime.chat.set_memory(memory);
                }
            });
        }
    }

    // ------------------------=
    // FUNC: submit_ai_chat_input
    // DESC: Submits one chat turn and durably checkpoints any semantic-memory change.
    // ------------------=
    fn submit_ai_chat_input(&mut self) {
        let memory = crate::runtime::ai::with_ai_runtime(|runtime| {
            if !runtime.chat.submit_input() {
                return None;
            }
            runtime.chat.take_memory_update()
        });
        let Some(memory) = memory else {
            return;
        };
        let user = self.current_user;
        let updated = crate::runtime::with_runtime(|runtime| {
            runtime.identity.update_ai_memory(user, user, memory)
        })
        .transpose()
        .is_ok();
        if updated {
            let _ = crate::runtime::persist_identity_state();
        }
    }

    // ------------------------=
    // FUNC: persist_ai_chat_preferences
    // DESC: Commits desktop chat visibility and selected model to the user's native identity object.
    // ------------------=
    fn persist_ai_chat_preferences(&mut self) {
        let (enabled, model_index) = crate::runtime::ai::with_ai_runtime(|runtime| {
            (
                runtime.chat.enabled(),
                runtime.chat.selected_model_index() as u8,
            )
        });
        let user = self.current_user;
        let updated = crate::runtime::with_runtime(|runtime| {
            runtime
                .identity
                .update_ai_chat_preferences(user, user, enabled, model_index)
        })
        .transpose()
        .is_ok();
        if updated {
            let _ = crate::runtime::persist_identity_state();
        }
    }

    // ------------------------=
    // FUNC: set_ai_chat_enabled
    // DESC: Enables or disables the desktop AI surface and persists the authenticated preference.
    // ------------------=
    fn set_ai_chat_enabled(&mut self, enabled: bool) {
        crate::runtime::ai::with_ai_runtime(|runtime| runtime.chat.set_enabled(enabled));
        self.ai_chat_focus = 0;
        self.persist_ai_chat_preferences();
    }

    // ------------------------=
    // FUNC: select_next_chat_model
    // DESC: Selects the next installed chat model and persists the choice.
    // ------------------=
    fn select_next_chat_model(&mut self) {
        crate::runtime::ai::with_ai_runtime(|runtime| runtime.chat.select_next_model());
        self.persist_ai_chat_preferences();
    }

    // ------------------------=
    // FUNC: ensure_app_task
    // DESC: Registers one singleton GUI application's live execution context when needed.
    // ------------------=
    fn ensure_app_task(&mut self, image_identity: u32) {
        let _ = crate::runtime::with_runtime(|runtime| {
            let running = (0..runtime.execution.count()).any(|index| {
                runtime
                    .execution
                    .nth(index)
                    .map(|task| {
                        task.image_identity == image_identity
                            && task.state != crate::runtime::execution::ContextState::Stopped
                    })
                    .unwrap_or(false)
            });
            if !running {
                let _ = runtime
                    .task_manager
                    .launch(&mut runtime.execution, image_identity);
            }
        });
    }

    // ------------------------=
    // FUNC: open_text_editor
    // DESC: Opens the native multiline Text Editor as an authenticated desktop window.
    // ------------------=
    fn open_text_editor(&mut self) {
        self.ensure_app_task(crate::runtime::task_manager::IMAGE_TEXT_EDITOR);
        if self.mode != ConsoleMode::Desktop {
            self.enter_desktop();
        }
        self.store_active_app_window();
        self.desktop_app = DesktopAppKind::TextEditor;
        self.editor_window.visible = true;
        self.load_active_app_window();
        self.app_window_dragging = false;
        self.app_window_resizing = None;
        let _ = self.checkpoint_desktop_layout();
    }

    // ------------------------=
    // FUNC: open_command_window
    // DESC: Opens the native Infinity Console language inside a desktop command window.
    // ------------------=
    fn open_command_window(&mut self) {
        self.ensure_app_task(crate::runtime::task_manager::IMAGE_COMMAND_WINDOW);
        if self.mode != ConsoleMode::Desktop {
            self.enter_desktop();
        }
        self.store_active_app_window();
        self.desktop_app = DesktopAppKind::CommandWindow;
        self.command_window.visible = true;
        self.load_active_app_window();
        self.app_window_dragging = false;
        self.app_window_resizing = None;
        if !self.command_window_suspended {
            self.reset_input();
            self.output.clear();
            self.output.write_line(b"Infinity Command Window");
            self.output
                .write_line(b"Type help or describe what you want.");
        }
        self.command_window_suspended = false;
        crate::output_text(b"[ui] desktop command window opened\n");
        let _ = self.checkpoint_desktop_layout();
    }

    // ------------------------=
    // FUNC: open_task_manager
    // DESC: Opens the live graphical Task Manager and registers its application context.
    // ------------------=
    fn open_task_manager(&mut self) {
        self.ensure_app_task(crate::runtime::task_manager::IMAGE_TASK_MANAGER);
        if self.mode != ConsoleMode::Desktop {
            self.enter_desktop();
        }
        self.store_active_app_window();
        self.desktop_app = DesktopAppKind::TaskManager;
        self.task_manager_window.visible = true;
        self.load_active_app_window();
        self.refresh_task_manager_output();
    }

    // ------------------------=
    // FUNC: checkpoint_active_file_navigator
    // DESC: Commits the active File Navigator state and geometry before another layer is raised.
    // ------------------=
    fn checkpoint_active_file_navigator(&mut self) {
        let state = crate::runtime::with_runtime(|runtime| runtime.file_navigator).flatten();
        let Some(state) = state else { return };
        let window = crate::runtime::object_navigation::FileNavigatorWindow {
            state,
            task_handle: crate::runtime::with_runtime(|runtime| {
                runtime
                    .file_navigators
                    .active_index()
                    .and_then(|index| runtime.file_navigators.window(index))
                    .map(|window| window.task_handle)
            })
            .flatten()
            .unwrap_or(0),
            x: self.home_window_x,
            y: self.home_window_y,
            width: self.home_window_width,
            height: self.home_window_height,
            maximized: self.home_window_maximized,
            visible: self.home_window_visible,
            z_order: crate::runtime::with_runtime(|runtime| {
                runtime
                    .file_navigators
                    .active_index()
                    .and_then(|index| runtime.file_navigators.window(index))
                    .map(|window| window.z_order)
            })
            .flatten()
            .unwrap_or(1),
        };
        let _ =
            crate::runtime::with_runtime(|runtime| runtime.file_navigators.update_active(window));
    }

    // ------------------------=
    // FUNC: load_file_navigator_window
    // DESC: Raises one navigator layer and loads its independent state into the active window engine.
    // ------------------=
    fn load_file_navigator_window(&mut self, index: usize) -> bool {
        // Close/minimize/restore may already have selected the destination slot.
        // Never overwrite that slot with the previously displayed window's state.
        let active = crate::runtime::with_runtime(|runtime| runtime.file_navigators.active_index())
            .flatten();
        if active != Some(index) {
            self.checkpoint_active_file_navigator();
        }
        let window = crate::runtime::with_runtime(|runtime| {
            let window = runtime.file_navigators.raise(index)?;
            runtime.file_navigator = Some(window.state);
            Some(window)
        })
        .flatten();
        let Some(window) = window else { return false };
        self.store_active_app_window();
        self.desktop_app = DesktopAppKind::None;
        self.home_window_x = window.x;
        self.home_window_y = window.y;
        self.home_window_width = window.width;
        self.home_window_height = window.height;
        self.home_window_maximized = window.maximized;
        self.home_window_visible = window.visible;
        true
    }

    // ------------------------=
    // FUNC: open_file_navigator_window
    // DESC: Launches a new independently functional File Navigator layer at the requested namespace.
    // ------------------=
    fn open_file_navigator_window(&mut self, path: &[u8]) -> Option<u16> {
        self.checkpoint_active_file_navigator();
        let launched = crate::runtime::with_runtime(|runtime| {
            let handle = runtime
                .task_manager
                .launch(
                    &mut runtime.execution,
                    crate::runtime::task_manager::IMAGE_FILE_NAVIGATOR,
                )
                .ok()?;
            let index = runtime.file_navigators.launch(path, handle.0).ok()?;
            let window = runtime.file_navigators.window(index)?;
            runtime.file_navigator = Some(window.state);
            Some(window)
        })
        .flatten();
        let Some(window) = launched else { return None };
        self.store_active_app_window();
        self.desktop_app = DesktopAppKind::None;
        self.home_window_x = window.x;
        self.home_window_y = window.y;
        self.home_window_width = window.width;
        self.home_window_height = window.height;
        self.home_window_maximized = window.maximized;
        self.home_window_visible = true;
        self.home_selected_item = None;
        self.enter_desktop();
        Some(window.task_handle)
    }

    // ------------------------=
    // FUNC: inactive_file_navigator_at_pointer
    // DESC: Finds the topmost non-active navigator whose complete window contains the pointer.
    // ------------------=
    fn inactive_file_navigator_at_pointer(&self) -> Option<usize> {
        crate::runtime::with_runtime(|runtime| {
            let active = runtime.file_navigators.active_index();
            runtime
                .file_navigators
                .topmost_at(self.pointer_x, self.pointer_y)
                .filter(|index| Some(*index) != active)
        })
        .flatten()
    }

    // ------------------------=
    // FUNC: minimize_desktop_app
    // DESC: Hides a native app without clearing its document, command output, or interaction state.
    // ------------------=
    fn minimize_desktop_app(&mut self) {
        self.store_active_app_window();
        match self.desktop_app {
            DesktopAppKind::TextEditor => self.editor_window.visible = false,
            DesktopAppKind::CommandWindow => {
                self.command_window.visible = false;
                self.command_window_suspended = true;
            }
            DesktopAppKind::TaskManager => self.task_manager_window.visible = false,
            DesktopAppKind::None => return,
        }
        self.desktop_app = DesktopAppKind::None;
        self.app_window_dragging = false;
        self.app_window_resizing = None;
        self.shell_menu = 0;
        let _ = self.checkpoint_desktop_layout();
    }

    // ------------------------=
    // FUNC: close_desktop_app
    // DESC: Dismisses the active desktop application without changing session or desktop state.
    // ------------------=
    fn close_desktop_app(&mut self) {
        self.store_active_app_window();
        if self.desktop_app == DesktopAppKind::CommandWindow {
            self.command_window_suspended = false;
        }
        match self.desktop_app {
            DesktopAppKind::TextEditor => self.editor_window.visible = false,
            DesktopAppKind::CommandWindow => self.command_window.visible = false,
            DesktopAppKind::TaskManager => self.task_manager_window.visible = false,
            DesktopAppKind::None => {}
        }
        self.desktop_app = DesktopAppKind::None;
        self.app_window_dragging = false;
        self.app_window_resizing = None;
        self.editor_scroll_dragging = false;
        self.editor_dialog = EditorDialog::None;
        self.reset_input();
        let _ = self.checkpoint_desktop_layout();
    }

    // ------------------------=
    // FUNC: store_active_app_window
    // DESC: Saves the focused application geometry without disturbing other open windows.
    // ------------------=
    fn store_active_app_window(&mut self) {
        let state = DesktopAppWindowState {
            x: self.app_window_x,
            y: self.app_window_y,
            width: self.app_window_width,
            height: self.app_window_height,
            maximized: self.app_window_maximized,
            visible: true,
        };
        match self.desktop_app {
            DesktopAppKind::TextEditor => self.editor_window = state,
            DesktopAppKind::CommandWindow => self.command_window = state,
            DesktopAppKind::TaskManager => self.task_manager_window = state,
            DesktopAppKind::None => {}
        }
    }

    // ------------------------=
    // FUNC: load_active_app_window
    // DESC: Loads the focused application geometry for independent manipulation.
    // ------------------=
    fn load_active_app_window(&mut self) {
        let state = match self.desktop_app {
            DesktopAppKind::TextEditor => self.editor_window,
            DesktopAppKind::CommandWindow => self.command_window,
            DesktopAppKind::TaskManager => self.task_manager_window,
            DesktopAppKind::None => return,
        };
        self.app_window_x = state.x;
        self.app_window_y = state.y;
        self.app_window_width = state.width;
        self.app_window_height = state.height;
        self.app_window_maximized = state.maximized;
        self.app_window_restore_x = state.x;
        self.app_window_restore_y = state.y;
        self.app_window_restore_width = state.width;
        self.app_window_restore_height = state.height;
    }

    // ------------------------=
    // FUNC: focus_desktop_app
    // DESC: Raises one already-open application window while preserving the previous window state.
    // ------------------=
    fn focus_desktop_app(&mut self, app: DesktopAppKind) {
        if app == self.desktop_app {
            return;
        }
        self.store_active_app_window();
        self.desktop_app = app;
        self.load_active_app_window();
        self.app_window_dragging = false;
        self.app_window_resizing = None;
        let _ = self.checkpoint_desktop_layout();
    }

    // ------------------------=
    // FUNC: desktop_app_windows
    // DESC: Returns both open window states with the focused window's live geometry applied.
    // ------------------=
    fn desktop_app_windows(
        &self,
    ) -> (
        DesktopAppWindowState,
        DesktopAppWindowState,
        DesktopAppWindowState,
    ) {
        let mut editor = self.editor_window;
        let mut command = self.command_window;
        let mut task_manager = self.task_manager_window;
        let active = DesktopAppWindowState {
            x: self.app_window_x,
            y: self.app_window_y,
            width: self.app_window_width,
            height: self.app_window_height,
            maximized: self.app_window_maximized,
            visible: true,
        };
        match self.desktop_app {
            DesktopAppKind::TextEditor => editor = active,
            DesktopAppKind::CommandWindow => command = active,
            DesktopAppKind::TaskManager => task_manager = active,
            DesktopAppKind::None => {}
        }
        (editor, command, task_manager)
    }

    // ------------------------=
    // FUNC: capture_desktop_layout
    // DESC: Captures every desktop window and object position before the authenticated surface is hidden.
    // ------------------=
    fn capture_desktop_layout(&mut self) -> DesktopSessionLayout {
        self.store_active_app_window();
        let focused_surface = if self.mode == ConsoleMode::Settings {
            DesktopResumeSurface::Settings
        } else {
            match self.desktop_app {
                DesktopAppKind::TextEditor => DesktopResumeSurface::TextEditor,
                DesktopAppKind::CommandWindow => DesktopResumeSurface::CommandWindow,
                DesktopAppKind::TaskManager => DesktopResumeSurface::TaskManager,
                DesktopAppKind::None => DesktopResumeSurface::Workspace,
            }
        };
        DesktopSessionLayout {
            home: WindowPlacement::new(
                self.home_window_x,
                self.home_window_y,
                self.home_window_width,
                self.home_window_height,
                self.home_window_maximized,
                self.home_window_visible,
            ),
            settings: WindowPlacement::new(
                self.settings_window.x,
                self.settings_window.y,
                self.settings_window.width,
                self.settings_window.height,
                self.settings_window.maximized,
                self.mode == ConsoleMode::Settings,
            ),
            editor: WindowPlacement::new(
                self.editor_window.x,
                self.editor_window.y,
                self.editor_window.width,
                self.editor_window.height,
                self.editor_window.maximized,
                self.editor_window.visible,
            ),
            command: WindowPlacement::new(
                self.command_window.x,
                self.command_window.y,
                self.command_window.width,
                self.command_window.height,
                self.command_window.maximized,
                self.command_window.visible,
            ),
            task_manager: WindowPlacement::new(
                self.task_manager_window.x,
                self.task_manager_window.y,
                self.task_manager_window.width,
                self.task_manager_window.height,
                self.task_manager_window.maximized,
                self.task_manager_window.visible,
            ),
            desktop_item_positions: self.desktop_item_positions,
            focused_surface,
            settings_section: self.system_focus,
            settings_expanded_row: self.settings_window.expanded_row,
            settings_scroll_offset: self.settings_window.scroll_offset,
            input_preferences: crate::ui::input_preferences::current().encode(),
        }
    }

    // ------------------------=
    // FUNC: restore_desktop_layout
    // DESC: Restores the exact pre-lock window geometry, desktop positions, visibility, and focus.
    // ------------------=
    fn restore_desktop_layout(&mut self, layout: DesktopSessionLayout) {
        crate::ui::input_preferences::apply(crate::ui::input_preferences::Preferences::decode(
            layout.input_preferences,
        ));
        self.home_window_x = layout.home.x;
        self.home_window_y = layout.home.y;
        self.home_window_width = layout.home.width;
        self.home_window_height = layout.home.height;
        self.home_window_maximized = layout.home.maximized;
        self.home_window_visible = layout.home.visible;
        self.home_window_restore_x = layout.home.x;
        self.home_window_restore_y = layout.home.y;
        self.home_window_restore_width = layout.home.width;
        self.home_window_restore_height = layout.home.height;
        self.settings_window.x = layout.settings.x;
        self.settings_window.y = layout.settings.y;
        self.settings_window.width = layout.settings.width;
        self.settings_window.height = layout.settings.height;
        self.settings_window.maximized = layout.settings.maximized;
        self.settings_window.expanded_row = layout.settings_expanded_row;
        self.settings_window.scroll_offset = layout.settings_scroll_offset;
        self.settings_scroll_target = layout.settings_scroll_offset;
        self.editor_window = DesktopAppWindowState {
            x: layout.editor.x,
            y: layout.editor.y,
            width: layout.editor.width,
            height: layout.editor.height,
            maximized: layout.editor.maximized,
            visible: layout.editor.visible,
        };
        self.command_window = DesktopAppWindowState {
            x: layout.command.x,
            y: layout.command.y,
            width: layout.command.width,
            height: layout.command.height,
            maximized: layout.command.maximized,
            visible: layout.command.visible,
        };
        self.task_manager_window = DesktopAppWindowState {
            x: layout.task_manager.x,
            y: layout.task_manager.y,
            width: layout.task_manager.width,
            height: layout.task_manager.height,
            maximized: layout.task_manager.maximized,
            visible: layout.task_manager.visible,
        };
        self.desktop_item_positions = layout.desktop_item_positions;
        match layout.focused_surface {
            DesktopResumeSurface::Workspace => self.desktop_app = DesktopAppKind::None,
            DesktopResumeSurface::Settings => {
                self.desktop_app = DesktopAppKind::None;
                self.mode = ConsoleMode::Settings;
                self.system_focus = layout.settings_section;
            }
            DesktopResumeSurface::TextEditor => {
                self.desktop_app = DesktopAppKind::TextEditor;
                self.load_active_app_window();
            }
            DesktopResumeSurface::CommandWindow => {
                self.desktop_app = DesktopAppKind::CommandWindow;
                self.load_active_app_window();
            }
            DesktopResumeSurface::TaskManager => {
                self.desktop_app = DesktopAppKind::TaskManager;
                self.load_active_app_window();
                self.refresh_task_manager_output();
            }
        }
    }

    // ------------------------=
    // FUNC: restore_persisted_desktop_layout
    // DESC: Restores the authenticated user's last durable cross-session desktop layout.
    // ------------------=
    fn restore_persisted_desktop_layout(&mut self) -> bool {
        crate::ui::input_preferences::apply(crate::ui::input_preferences::Preferences::defaults());
        if self.current_user.is_zero() {
            return false;
        }
        let Some(layout) = crate::runtime::with_runtime(|runtime| {
            runtime.identity.user_desktop_layout(self.current_user)
        })
        .flatten() else {
            return false;
        };
        self.restore_desktop_layout(layout);
        true
    }

    // ------------------------=
    // FUNC: checkpoint_desktop_layout
    // DESC: Updates the authenticated user's in-memory desktop record after a settled layout change.
    // ------------------=
    fn checkpoint_desktop_layout(&mut self) -> bool {
        if self.current_user.is_zero() {
            return false;
        }
        let layout = self.capture_desktop_layout();
        let user = self.current_user;
        crate::runtime::with_runtime(|runtime| {
            runtime
                .identity
                .update_user_desktop_layout(user, user, layout)
        })
        .transpose()
        .is_ok()
    }

    // ------------------------=
    // FUNC: persist_desktop_layout
    // DESC: Checkpoints and durably commits the user's complete layout at a session boundary.
    // ------------------=
    fn persist_desktop_layout(&mut self) -> bool {
        self.checkpoint_desktop_layout() && crate::runtime::persist_identity_state()
    }

    // ------------------------=
    // FUNC: lock_session_preserving_desktop
    // DESC: Locks the active identity session only after retaining its complete desktop layout.
    // ------------------=
    fn lock_session_preserving_desktop(&mut self, inactive: bool) -> bool {
        if self.current_session.is_zero() {
            return false;
        }
        let layout = self.capture_desktop_layout();
        let locked = crate::runtime::with_runtime(|runtime| {
            runtime
                .identity
                .lock_session(self.current_session, self.current_user)
        })
        .unwrap_or(Err(crate::runtime::identity::IdentityError::InvalidState))
        .is_ok();
        if locked {
            let _ = self.persist_desktop_layout();
            self.locked_desktop_layout.save(layout);
            self.mode = ConsoleMode::Locked;
            self.session_idle.note_activity();
            self.reset_input();
            crate::output_text(if inactive {
                b"[session] locked after inactivity; desktop layout retained\n"
            } else {
                b"[session] locked; desktop layout retained\n"
            });
        }
        locked
    }

    // ------------------------=
    // FUNC: inactive_app_at_pointer
    // DESC: Hit-tests visible non-focused application windows for click-to-raise behavior.
    // ------------------=
    fn inactive_app_at_pointer(&self, layout: SystemLayout) -> Option<DesktopAppKind> {
        if self.desktop_app == DesktopAppKind::None && self.home_window_visible {
            let (left, top, width, height) = layout.home_window_geometry_sized(
                self.home_window_x,
                self.home_window_y,
                self.home_window_width,
                self.home_window_height,
                self.home_window_maximized,
            );
            let pointer_x = self.system.framebuffer_width as i32 * self.pointer_x / 1000;
            let pointer_y = self.system.framebuffer_height as i32 * self.pointer_y / 1000;
            if pointer_x >= left as i32
                && pointer_x < left.saturating_add(width) as i32
                && pointer_y >= top as i32
                && pointer_y < top.saturating_add(height) as i32
            {
                return None;
            }
        }
        let (editor, command, task_manager) = self.desktop_app_windows();
        for (app, state, is_editor) in [
            (DesktopAppKind::TaskManager, task_manager, false),
            (DesktopAppKind::TextEditor, editor, true),
            (DesktopAppKind::CommandWindow, command, false),
        ] {
            if app == self.desktop_app || !state.visible {
                continue;
            }
            if layout.desktop_app_window_target(
                self.pointer_x,
                self.pointer_y,
                state.x,
                state.y,
                state.width,
                state.height,
                state.maximized,
                is_editor,
            ) != DesktopAppWindowTarget::None
            {
                return Some(app);
            }
        }
        None
    }

    // ------------------------=
    // FUNC: save_editor_document
    // DESC: Creates or updates the Text Editor document as a native Personal-space object.
    // ------------------=
    fn save_editor_document(&mut self) {
        if self.editor_document_path_length == 0 {
            self.open_editor_save_as_dialog();
            return;
        }
        let content = self.editor_document.bytes();
        let saved = crate::storage::object_write_path(
            &self.editor_document_path[..self.editor_document_path_length],
            content,
        )
        .is_ok();
        if saved {
            self.editor_document.save();
            crate::output_text(b"[editor] document persisted\n");
        } else {
            crate::output_text(b"[editor] save failed\n");
        }
    }

    // ------------------------=
    // FUNC: open_editor_document
    // DESC: Opens the persisted Personal-space Text Editor document into the active buffer.
    // ------------------=
    fn open_editor_document(&mut self) {
        self.editor_dialog = EditorDialog::Open;
        self.system_focus = 0;
        self.reset_input();
        self.refresh_editor_open_list();
    }

    // ------------------------=
    // FUNC: open_selected_editor_document
    // DESC: Loads the selected native Text object and retains its stable namespace reference for later versioned saves.
    // ------------------=
    fn open_selected_editor_document(&mut self) {
        let entry = match crate::storage::namespace_list_nth(
            crate::ui::text_editor::DOCUMENT_NAMESPACE,
            self.system_focus,
        ) {
            Ok(Some(entry)) => entry,
            _ => return,
        };
        let mut content = [0u8; crate::ui::text_editor::DOCUMENT_CAPACITY];
        let path_length = entry.path_len as usize;
        if let Ok((_, length)) =
            crate::storage::object_read_path(&entry.path[..path_length], None, &mut content)
        {
            if self.editor_document.open(&content[..length]) {
                self.editor_document_path[..path_length]
                    .copy_from_slice(&entry.path[..path_length]);
                self.editor_document_path_length = path_length;
                let name =
                    &entry.path[crate::ui::text_editor::DOCUMENT_NAMESPACE.len()..path_length];
                let name_length = name.len().min(self.editor_document_name.len());
                self.editor_document_name[..name_length].copy_from_slice(&name[..name_length]);
                self.editor_document_name_length = name_length;
                self.editor_scroll_row = 0;
                self.close_editor_dialog();
                crate::output_text(b"[editor] document opened\n");
                return;
            }
        }
        crate::output_text(b"[editor] open failed\n");
    }

    // ------------------------=
    // FUNC: delete_editor_document
    // DESC: Removes the persisted Personal-space document and resets the editor buffer.
    // ------------------=
    fn delete_editor_document(&mut self) {
        if self.editor_document_path_length != 0
            && crate::storage::object_remove_path(
                &self.editor_document_path[..self.editor_document_path_length],
            )
            .is_ok()
        {
            self.editor_document.clear();
            self.editor_document_path_length = 0;
            self.editor_document_name_length = 0;
            self.editor_scroll_row = 0;
            crate::output_text(b"[editor] document deleted\n");
        } else {
            crate::output_text(b"[editor] delete failed\n");
        }
    }

    // ------------------------=
    // FUNC: open_editor_save_as_dialog
    // DESC: Opens a bounded native-object naming sheet without changing the active document.
    // ------------------=
    fn open_editor_save_as_dialog(&mut self) {
        self.editor_dialog = EditorDialog::SaveAs;
        self.reset_input();
    }

    // ------------------------=
    // FUNC: save_editor_document_as
    // DESC: Creates a uniquely named Text object and binds the editor to its new stable Object ID through a namespace reference.
    // ------------------=
    fn save_editor_document_as(&mut self) {
        let name = &self.command[..self.command_length];
        let mut path = [0u8; crate::ui::text_editor::DOCUMENT_PATH_CAPACITY];
        let Some(path_length) = crate::ui::text_editor::document_path(name, &mut path) else {
            return;
        };
        if crate::storage::namespace_resolve(&path[..path_length]).is_ok() {
            return;
        }
        if crate::storage::object_create_note_at(
            name,
            self.editor_document.bytes(),
            &path[..path_length],
        )
        .is_ok()
        {
            self.editor_document_path[..path_length].copy_from_slice(&path[..path_length]);
            self.editor_document_path_length = path_length;
            self.editor_document_name[..name.len()].copy_from_slice(name);
            self.editor_document_name_length = name.len();
            self.editor_document.save();
            self.close_editor_dialog();
        }
    }

    // ------------------------=
    // FUNC: close_editor_dialog
    // DESC: Closes the editor naming or object-picker sheet and clears its transient input.
    // ------------------=
    fn close_editor_dialog(&mut self) {
        self.editor_dialog = EditorDialog::None;
        self.system_focus = 0;
        self.reset_input();
        self.output.clear();
    }

    // ------------------------=
    // FUNC: editor_document_count
    // DESC: Counts discoverable Personal document namespace references for bounded picker navigation.
    // ------------------=
    fn editor_document_count(&self) -> usize {
        let mut count = 0usize;
        while count < OUTPUT_ROWS
            && matches!(
                crate::storage::namespace_list_nth(
                    crate::ui::text_editor::DOCUMENT_NAMESPACE,
                    count,
                ),
                Ok(Some(_))
            )
        {
            count += 1;
        }
        count
    }

    // ------------------------=
    // FUNC: refresh_editor_open_list
    // DESC: Projects discoverable native document names into the bounded picker without parsing rendered text for behavior.
    // ------------------=
    fn refresh_editor_open_list(&mut self) {
        self.output.clear();
        for index in 0..OUTPUT_ROWS {
            let entry = match crate::storage::namespace_list_nth(
                crate::ui::text_editor::DOCUMENT_NAMESPACE,
                index,
            ) {
                Ok(Some(entry)) => entry,
                _ => break,
            };
            let path_length = entry.path_len as usize;
            self.output.write_line(
                &entry.path[crate::ui::text_editor::DOCUMENT_NAMESPACE.len()..path_length],
            );
        }
    }

    // ------------------------=
    // FUNC: desktop_reference_paths
    // DESC: Maps one Home presentation item to its native source and Desktop namespace projections.
    // ------------------=
    fn desktop_reference_paths(item: usize) -> Option<(&'static [u8], &'static [u8])> {
        match item {
            0 => Some((
                b"/home/default/documents",
                b"/home/default/desktop/documents",
            )),
            1 => Some((
                b"/home/default/downloads",
                b"/home/default/desktop/downloads",
            )),
            2 => Some((b"/home/default/pictures", b"/home/default/desktop/pictures")),
            3 => Some((b"/home/default/media", b"/home/default/desktop/music")),
            4 => Some((b"/home/default/media", b"/home/default/desktop/videos")),
            5 => Some((b"/home/default/projects", b"/home/default/desktop/projects")),
            _ => None,
        }
    }

    // ------------------------=
    // FUNC: refresh_desktop_items
    // DESC: Reconstructs visible Desktop references from the persistent human namespace.
    // ------------------=
    fn refresh_desktop_items(&mut self) {
        let mut items = 0u8;
        for item in 0..6usize {
            if let Some((_, target)) = Self::desktop_reference_paths(item) {
                if crate::storage::namespace_resolve(target).is_ok() {
                    items |= 1u8 << item;
                }
            }
        }
        self.desktop_items = items;
    }

    // ------------------------=
    // FUNC: place_desktop_reference
    // DESC: Persists a second namespace reference and records its current Desktop presentation point.
    // ------------------=
    fn place_desktop_reference(&mut self, item: usize) {
        let Some((source, target)) = Self::desktop_reference_paths(item) else {
            return;
        };
        if crate::storage::namespace_ensure_link(source, target).is_ok() {
            self.desktop_items |= 1u8 << item;
            self.desktop_item_positions[item] =
                [self.pointer_x.clamp(35, 950), self.pointer_y.clamp(90, 880)];
            crate::output_text(b"[objects] Desktop reference committed\n");
        } else {
            crate::output_text(b"[objects] Desktop reference could not be committed\n");
        }
    }

    // ------------------------=
    // FUNC: desktop_item_at_pointer
    // DESC: Hit-tests persistent Desktop references using their normalized presentation bounds.
    // ------------------=
    fn desktop_item_at_pointer(&self) -> Option<usize> {
        for item in (0..7usize).rev() {
            if self.desktop_items & (1u8 << item) == 0 {
                continue;
            }
            let [x, y] = self.desktop_item_positions[item];
            if (x - 34..=x + 34).contains(&self.pointer_x)
                && (y - 32..=y + 48).contains(&self.pointer_y)
            {
                return Some(item);
            }
        }
        None
    }

    // ------------------------=
    // FUNC: desktop_target
    // DESC: Resolves a pointer target from the same live Home bounds used by the renderer.
    // ------------------=
    fn desktop_target(&self, layout: SystemLayout) -> Option<DesktopTarget> {
        if self.home_window_visible {
            let overlay = crate::runtime::with_runtime(|runtime| {
                let state = runtime.file_navigator?;
                let menu = state.menu_open.map(|value| value.index()).unwrap_or(0);
                let menu_items = state.menu_open.map(|value| value.item_count()).unwrap_or(0);
                let dialog = match state.dialog_open {
                    Some(crate::runtime::object_navigation::FileNavigatorDialog::EmptyTrash) => 1,
                    Some(crate::runtime::object_navigation::FileNavigatorDialog::About) => 2,
                    Some(crate::runtime::object_navigation::FileNavigatorDialog::Help) => 3,
                    Some(crate::runtime::object_navigation::FileNavigatorDialog::Location) => 4,
                    None => 0,
                };
                layout.file_navigator_overlay_target(
                    self.pointer_x,
                    self.pointer_y,
                    self.home_window_x,
                    self.home_window_y,
                    self.home_window_width,
                    self.home_window_height,
                    self.home_window_maximized,
                    menu,
                    menu_items,
                    dialog,
                )
            })
            .flatten();
            if overlay.is_some() {
                return overlay;
            }
        }
        let target = layout.desktop_target_sized(
            self.pointer_x,
            self.pointer_y,
            self.home_window_x,
            self.home_window_y,
            self.home_window_width,
            self.home_window_height,
            self.home_window_visible,
            self.home_window_maximized,
        );
        if matches!(
            target,
            Some(DesktopTarget::HomeItem(_)) | Some(DesktopTarget::HomeContent)
        ) {
            return self
                .file_navigator_item_at_pointer(layout)
                .map(DesktopTarget::HomeItem)
                .or(Some(DesktopTarget::HomeContent));
        }
        target
    }

    // ------------------------=
    // FUNC: file_navigator_item_at_pointer
    // DESC: Maps live list or grid geometry to a direct-child index in the active NamespaceRef.
    // ------------------=
    fn file_navigator_item_at_pointer(&self, layout: SystemLayout) -> Option<usize> {
        let state = crate::runtime::with_runtime(|runtime| runtime.file_navigator).flatten()?;
        let (left, top, width, height) = layout.home_window_geometry_sized(
            self.home_window_x,
            self.home_window_y,
            self.home_window_width,
            self.home_window_height,
            self.home_window_maximized,
        );
        let scale = (self.system.framebuffer_width as usize / 1000).max(1);
        let point_x =
            self.system.framebuffer_width as usize * self.pointer_x.max(0) as usize / 1000;
        let point_y =
            self.system.framebuffer_height as usize * self.pointer_y.max(0) as usize / 1000;
        let sidebar = width * 27 / 100;
        let content_left = left + sidebar;
        let content_top = top + 34 * scale + 38 * scale;
        if point_x < content_left
            || point_x >= left + width
            || point_y < content_top
            || point_y >= top + height
        {
            return None;
        }
        let grid_x = left + sidebar + 28 * scale;
        let grid_y = top + 34 * scale + 58 * scale;
        if point_x < grid_x.saturating_sub(18 * scale) || point_y < grid_y.saturating_sub(8 * scale)
        {
            return None;
        }
        let count = navigator_child_count(state.active_namespace_ref.as_bytes());
        let index = if state.view_mode == crate::runtime::object_navigation::ViewMode::List {
            state.scroll_offset / (34 * scale) + point_y.saturating_sub(grid_y) / (34 * scale)
        } else {
            let gap = width.saturating_sub(sidebar + 55 * scale) / 4;
            let tile_step = (self.system.framebuffer_height as usize / 23).max(34) + 40 * scale;
            let column = ((point_x - grid_x) / gap.max(1)).min(3);
            let row = (point_y - grid_y) / tile_step.max(1);
            state.scroll_offset / tile_step.max(1) * 4 + row * 4 + column
        };
        (index < count).then_some(index)
    }

    // ------------------------=
    // FUNC: open_settings
    // DESC: Opens one Settings section without accidentally activating its first value.
    // ------------------=
    fn open_settings(&mut self, section: usize) {
        self.store_active_app_window();
        self.mode = ConsoleMode::Settings;
        self.system_focus = section.min(10);
        self.settings_window.row_count = if matches!(self.system_focus, 1 | 6 | 7 | 10) {
            8
        } else if self.system_focus == 3 {
            7
        } else {
            5
        };
        self.settings_editing = false;
        self.settings_window.expanded_row = matches!(self.system_focus, 6 | 7).then_some(0);
        self.settings_window.scroll_offset = 0;
        self.settings_scroll_target = 0;
        self.settings_window.control_focus = 0;
        self.settings_window_dragging = false;
        self.settings_window_resizing = None;
        self.settings_accent_dirty = false;
        self.settings_primary_dirty = false;
        self.settings_effects_dirty = false;
        self.settings_effect_dragging = None;
        self.settings_timeout_dragging = false;
        self.settings_scroll_dragging = false;
        self.reset_input();
        let _ = self.checkpoint_desktop_layout();
    }

    // ------------------------=
    // FUNC: toggle_settings_row
    // DESC: Opens one inline Settings detail well, closes it on a second activation, and reveals lower rows safely.
    // ------------------=
    fn toggle_settings_row(&mut self, row: usize) {
        if self.system_focus == 10 {
            let mut preferences = crate::ui::input_preferences::current();
            preferences.cycle(row);
            crate::ui::input_preferences::apply(preferences);
            let _ = self.persist_desktop_layout();
            return;
        }
        self.settings_window.expanded_row = if self.settings_window.expanded_row == Some(row) {
            None
        } else {
            Some(row.min(self.settings_window.row_count.saturating_sub(1)))
        };
        let layout = SystemLayout::new(
            self.system.framebuffer_width,
            self.system.framebuffer_height,
        );
        let mut unscrolled = self.settings_window;
        unscrolled.scroll_offset = 0;
        let window = layout.settings_window_geometry_for_section(unscrolled, self.system_focus);
        let desired_scroll = self
            .settings_window
            .expanded_row
            .map(|expanded| {
                layout
                    .settings_row_geometry(unscrolled, expanded)
                    .detail
                    .bottom()
                    .saturating_sub(window.viewport.bottom())
                    .max(0) as usize
                    / layout.scale().max(1)
            })
            .unwrap_or(0);
        let maximum_scroll = window.maximum_scroll;
        self.settings_scroll_target = desired_scroll.min(maximum_scroll);
        self.settings_window.scroll_offset = self.settings_window.scroll_offset.min(maximum_scroll);
    }

    // ------------------------=
    // FUNC: scroll_settings
    // DESC: Moves all Settings sections toward bounded logical scroll targets while leaving navigation focus unchanged.
    // ------------------=
    fn scroll_settings(&mut self, direction: i8) {
        let amount = crate::ui::input_preferences::current().wheel(direction);
        let distance = amount.unsigned_abs() as usize * 20;
        let maximum_scroll = SystemLayout::new(
            self.system.framebuffer_width,
            self.system.framebuffer_height,
        )
        .settings_window_geometry_for_section(self.settings_window, self.system_focus)
        .maximum_scroll;
        if amount < 0 {
            self.settings_scroll_target = self.settings_scroll_target.saturating_sub(distance);
        } else {
            self.settings_scroll_target = self
                .settings_scroll_target
                .saturating_add(distance)
                .min(maximum_scroll);
        }
        self.settings_window.scroll_offset = self.settings_scroll_target;
    }

    // ------------------------=
    // FUNC: editor_scroll_geometry
    // DESC: Computes live wrapped-row and proportional scrollbar geometry for the active Text Editor.
    // ------------------=
    fn editor_scroll_geometry(&self) -> crate::ui::system_layout::EditorScrollGeometry {
        let layout = SystemLayout::new(
            self.system.framebuffer_width,
            self.system.framebuffer_height,
        );
        let window = layout.desktop_app_window_geometry(
            self.app_window_x,
            self.app_window_y,
            self.app_window_width,
            self.app_window_height,
            self.app_window_maximized,
        );
        let columns = (window.content.width as usize).saturating_sub(52 * layout.scale())
            / (9 * layout.scale()).max(1);
        let rows =
            crate::ui::text_editor::visual_line_count(self.editor_document.bytes(), columns.max(1));
        layout.desktop_editor_scroll_geometry(
            self.app_window_x,
            self.app_window_y,
            self.app_window_width,
            self.app_window_height,
            self.app_window_maximized,
            rows,
            self.editor_scroll_row,
        )
    }

    // ------------------------=
    // FUNC: scroll_editor
    // DESC: Scrolls the native Text Editor by bounded visual rows while preserving object content and selection state.
    // ------------------=
    fn scroll_editor(&mut self, direction: i8) {
        let maximum = self.editor_scroll_geometry().maximum_scroll;
        let amount = crate::ui::input_preferences::current().wheel(direction);
        let distance = amount.unsigned_abs() as usize;
        if amount < 0 {
            self.editor_scroll_row = self.editor_scroll_row.saturating_sub(distance);
        } else {
            self.editor_scroll_row = self.editor_scroll_row.saturating_add(distance).min(maximum);
        }
    }

    // ------------------------=
    // FUNC: sync_icon_theme
    // DESC: Applies the authenticated user's durable icon selection to every live UI surface.
    // ------------------=
    fn sync_icon_theme(&mut self) {
        let user = self.current_user;
        let _ = crate::runtime::with_runtime(|runtime| {
            let selection = runtime
                .identity
                .user_profile(user)
                .map(|profile| profile.icon_theme)
                .unwrap_or(0);
            runtime.ui.icons.activate(selection)
        });
    }

    // ------------------------=
    // FUNC: sync_accent
    // DESC: Applies the authenticated user's durable accent to every live semantic UI surface.
    // ------------------=
    fn sync_accent(&mut self) {
        let user = self.current_user;
        let _ = crate::runtime::with_runtime(|runtime| {
            let accent = runtime
                .identity
                .user_profile(user)
                .map(|profile| profile.accent_rgb)
                .unwrap_or(crate::runtime::identity::DEFAULT_ACCENT_RGB);
            runtime
                .ui
                .skins
                .set_accent(accent, crate::ui::skin::AppearanceScope::User)
        });
    }

    // ------------------------=
    // FUNC: sync_primary
    // DESC: Applies the durable machine primary color to every live frosted OS surface.
    // ------------------=
    fn sync_primary(&mut self) {
        let _ = crate::runtime::with_runtime(|runtime| {
            runtime.ui.skins.set_primary(
                runtime.identity.primary_rgb(),
                crate::ui::skin::AppearanceScope::Machine,
            )
        });
    }

    // ------------------------=
    // FUNC: sync_background_effects
    // DESC: Applies durable machine glass opacity and blur to every live semantic panel.
    // ------------------=
    fn sync_background_effects(&mut self) {
        let _ = crate::runtime::with_runtime(|runtime| {
            let (opacity, blur) = runtime.identity.background_effects();
            runtime.ui.skins.set_background_effects(
                opacity,
                blur,
                crate::ui::skin::AppearanceScope::Machine,
            )
        });
    }

    // ------------------------=
    // FUNC: preview_background_effect
    // DESC: Applies one slider value live without persisting every captured pointer sample.
    // ------------------=
    fn preview_background_effect(&mut self, row: usize, value: u8) {
        let changed = crate::runtime::with_runtime(|runtime| {
            let (mut opacity, mut blur) = runtime.ui.skins.background_effects();
            if row == 4 {
                opacity = value.clamp(40, 100);
            } else if row == 5 {
                blur = value.min(8);
            } else {
                return Err(crate::ui::skin::SkinError::InvalidAccent);
            }
            runtime.ui.skins.set_background_effects(
                opacity,
                blur,
                crate::ui::skin::AppearanceScope::Machine,
            )
        })
        .transpose()
        .is_ok();
        if changed {
            self.settings_effects_dirty = true;
        }
    }

    // ------------------------=
    // FUNC: commit_background_effects
    // DESC: Commits the previewed glass effects once to durable machine appearance state.
    // ------------------=
    fn commit_background_effects(&mut self) {
        if !self.settings_effects_dirty {
            return;
        }
        let user = self.current_user;
        let changed = crate::runtime::with_runtime(|runtime| {
            let (opacity, blur) = runtime.ui.skins.background_effects();
            runtime
                .identity
                .update_background_effects(user, opacity, blur)
        })
        .transpose()
        .is_ok();
        if changed {
            let _ = crate::runtime::persist_identity_state();
        }
        self.settings_effects_dirty = false;
    }

    // ------------------------=
    // FUNC: cycle_background_effect
    // DESC: Provides keyboard stepping for the same typed opacity and blur values used by pointer dragging.
    // ------------------=
    fn cycle_background_effect(&mut self, row: usize) {
        let (opacity, blur) =
            crate::runtime::with_runtime(|runtime| runtime.ui.skins.background_effects())
                .unwrap_or((88, 4));
        if row == 4 {
            self.preview_background_effect(row, if opacity >= 100 { 40 } else { opacity + 4 });
        } else if row == 5 {
            self.preview_background_effect(row, if blur >= 8 { 0 } else { blur + 1 });
        }
        self.commit_background_effects();
    }

    // ------------------------=
    // FUNC: user_no_activity_timeout_minutes
    // DESC: Reads the authenticated user's effective inactivity-lock deadline.
    // ------------------=
    fn user_no_activity_timeout_minutes(&self) -> u8 {
        crate::runtime::with_runtime(|runtime| {
            runtime
                .identity
                .user_profile(self.current_user)
                .map(|profile| profile.no_activity_timeout_minutes)
        })
        .flatten()
        .unwrap_or(crate::runtime::identity::DEFAULT_NO_ACTIVITY_TIMEOUT_MINUTES)
    }

    // ------------------------=
    // FUNC: preview_user_no_activity_timeout
    // DESC: Applies a bounded slider value to the live user profile without a durable write per pointer sample.
    // ------------------=
    fn preview_user_no_activity_timeout(&mut self, minutes: u8) {
        let _ = crate::runtime::with_runtime(|runtime| {
            runtime.identity.update_user_no_activity_timeout(
                self.current_user,
                self.current_user,
                minutes,
            )
        });
    }

    // ------------------------=
    // FUNC: commit_user_no_activity_timeout
    // DESC: Commits the selected user inactivity-lock deadline once after slider capture ends.
    // ------------------=
    fn commit_user_no_activity_timeout(&mut self) {
        let _ = crate::runtime::persist_identity_state();
    }

    // ------------------------=
    // FUNC: cycle_user_no_activity_timeout
    // DESC: Advances the inactivity deadline through practical keyboard-accessible presets.
    // ------------------=
    fn cycle_user_no_activity_timeout(&mut self) {
        const PRESETS: [u8; 8] = [1, 5, 10, 15, 30, 45, 60, 120];
        let current = self.user_no_activity_timeout_minutes();
        let next = PRESETS
            .iter()
            .position(|value| *value >= current)
            .map(|index| PRESETS[(index + 1) % PRESETS.len()])
            .unwrap_or(PRESETS[0]);
        self.preview_user_no_activity_timeout(next);
        self.commit_user_no_activity_timeout();
        self.session_idle.note_activity();
    }

    // ------------------------=
    // FUNC: preview_primary
    // DESC: Applies one primary picker color live without writing every pointer sample.
    // ------------------=
    fn preview_primary(&mut self, primary_rgb: u32) {
        let changed = crate::runtime::with_runtime(|runtime| {
            runtime
                .ui
                .skins
                .set_primary(primary_rgb, crate::ui::skin::AppearanceScope::Machine)
        })
        .transpose()
        .is_ok();
        if changed {
            self.settings_primary_dirty = true;
        }
    }

    // ------------------------=
    // FUNC: commit_primary
    // DESC: Commits the previewed primary color once to durable machine appearance state.
    // ------------------=
    fn commit_primary(&mut self) {
        if !self.settings_primary_dirty {
            return;
        }
        let user = self.current_user;
        let changed = crate::runtime::with_runtime(|runtime| {
            runtime
                .identity
                .update_primary(user, runtime.ui.skins.primary_rgb())
        })
        .transpose()
        .is_ok();
        if changed {
            let _ = crate::runtime::persist_identity_state();
        }
        self.settings_primary_dirty = false;
    }

    // ------------------------=
    // FUNC: adjust_primary
    // DESC: Adjusts the primary theme color from typed picker coordinates and previews it live.
    // ------------------=
    fn adjust_primary(&mut self, target: SettingsAccentTarget) {
        let current = crate::runtime::with_runtime(|runtime| runtime.ui.skins.primary_rgb())
            .unwrap_or(crate::runtime::identity::DEFAULT_PRIMARY_RGB);
        let (mut hue, mut saturation, mut value) = crate::ui::skin::rgb_to_hsv(current);
        match target {
            SettingsAccentTarget::Spectrum {
                saturation: next_saturation,
                value: next_value,
            } => {
                saturation = next_saturation;
                value = next_value.max(20);
            }
            SettingsAccentTarget::Hue(next_hue) => hue = next_hue,
        }
        self.preview_primary(crate::ui::skin::hsv_to_rgb(hue, saturation, value));
    }

    // ------------------------=
    // FUNC: cycle_primary
    // DESC: Provides keyboard-only primary surface selection from restrained frosted presets.
    // ------------------=
    fn cycle_primary(&mut self) {
        const PRESETS: [u32; 6] = [0x0d2238, 0x162a46, 0x251f42, 0x142f36, 0x35233d, 0x273041];
        let current = crate::runtime::with_runtime(|runtime| runtime.ui.skins.primary_rgb())
            .unwrap_or(PRESETS[0]);
        let next = PRESETS
            .iter()
            .position(|value| *value == current)
            .map(|index| PRESETS[(index + 1) % PRESETS.len()])
            .unwrap_or(PRESETS[0]);
        self.preview_primary(next);
        self.commit_primary();
    }

    // ------------------------=
    // FUNC: preview_accent
    // DESC: Applies one picker color immediately without performing a durable write for every mouse sample.
    // ------------------=
    fn preview_accent(&mut self, accent_rgb: u32) {
        let changed = crate::runtime::with_runtime(|runtime| {
            runtime
                .ui
                .skins
                .set_accent(accent_rgb, crate::ui::skin::AppearanceScope::User)
        })
        .transpose()
        .is_ok();
        if changed {
            self.settings_accent_dirty = true;
        }
    }

    // ------------------------=
    // FUNC: commit_accent
    // DESC: Commits the previewed accent once to the authenticated user profile and durable identity state.
    // ------------------=
    fn commit_accent(&mut self) {
        if !self.settings_accent_dirty {
            return;
        }
        let user = self.current_user;
        let changed = crate::runtime::with_runtime(|runtime| {
            let accent = runtime.ui.skins.accent_rgb();
            runtime.identity.update_user_accent(user, user, accent)
        })
        .transpose()
        .is_ok();
        if changed {
            let _ = crate::runtime::persist_identity_state();
        }
        self.settings_accent_dirty = false;
    }

    // ------------------------=
    // FUNC: adjust_accent
    // DESC: Adjusts the active accent from a typed picker coordinate and previews it live.
    // ------------------=
    fn adjust_accent(&mut self, target: SettingsAccentTarget) {
        let current = crate::runtime::with_runtime(|runtime| runtime.ui.skins.accent_rgb())
            .unwrap_or(crate::runtime::identity::DEFAULT_ACCENT_RGB);
        let (mut hue, mut saturation, mut value) = crate::ui::skin::rgb_to_hsv(current);
        match target {
            SettingsAccentTarget::Spectrum {
                saturation: next_saturation,
                value: next_value,
            } => {
                saturation = next_saturation;
                value = next_value.max(32);
            }
            SettingsAccentTarget::Hue(next_hue) => hue = next_hue,
        }
        self.preview_accent(crate::ui::skin::hsv_to_rgb(hue, saturation, value));
    }

    // ------------------------=
    // FUNC: cycle_accent
    // DESC: Provides keyboard-only accent selection by advancing through a polished preset palette.
    // ------------------=
    fn cycle_accent(&mut self) {
        const PRESETS: [u32; 8] = [
            0x4da3ff, 0x6f8cff, 0xa56dff, 0xe65cc8, 0xff6b78, 0xffa62b, 0x33d69f, 0x38d8ff,
        ];
        let current = crate::runtime::with_runtime(|runtime| runtime.ui.skins.accent_rgb())
            .unwrap_or(PRESETS[0]);
        let next = PRESETS
            .iter()
            .position(|value| *value == current)
            .map(|index| PRESETS[(index + 1) % PRESETS.len()])
            .unwrap_or(PRESETS[0]);
        self.preview_accent(next);
        self.commit_accent();
    }

    // ------------------------=
    // FUNC: cycle_icon_theme
    // DESC: Selects, persists, and immediately applies the next complete desktop icon family.
    // ------------------=
    fn cycle_icon_theme(&mut self) {
        let next = crate::runtime::with_runtime(|runtime| {
            let current = runtime
                .identity
                .user_profile(self.current_user)
                .map(|profile| profile.icon_theme)
                .unwrap_or(runtime.ui.icons.active() as u8);
            crate::ui::icon_theme::IconThemeId::from_u8(current)
                .unwrap_or(crate::ui::icon_theme::IconThemeId::CrystalBlueGlass)
                .next() as u8
        })
        .unwrap_or(0);
        self.select_icon_theme(next);
    }

    // ------------------------=
    // FUNC: select_icon_theme
    // DESC: Persists and immediately applies one explicitly selected installed icon family.
    // ------------------=
    fn select_icon_theme(&mut self, selection: u8) {
        let user = self.current_user;
        let changed = crate::runtime::with_runtime(|runtime| {
            runtime
                .identity
                .update_user_icon_theme(user, user, selection)?;
            runtime
                .ui
                .icons
                .activate(selection)
                .map_err(|_| crate::runtime::identity::IdentityError::InvalidInput)?;
            Ok::<(), crate::runtime::identity::IdentityError>(())
        })
        .transpose()
        .is_ok();
        if changed {
            let _ = crate::runtime::persist_identity_state();
            crate::output_text(b"[appearance] icon family transaction committed\n");
        }
    }

    // ------------------------=
    // FUNC: network_page
    // DESC: Returns the selected Network settings page independently from sidebar focus.
    // ------------------=
    fn network_page(&self) -> usize {
        self.settings_window.expanded_row.unwrap_or(0).min(6)
    }

    // ------------------------=
    // FUNC: node_settings_page
    // DESC: Returns the selected node-management page independently from sidebar focus.
    // ------------------=
    fn node_settings_page(&self) -> usize {
        self.settings_window.expanded_row.unwrap_or(0).min(4)
    }

    // ------------------------=
    // FUNC: network_static_configuration
    // DESC: Reads the current typed static IPv4 transaction as defaults for incremental editing.
    // ------------------=
    fn network_static_configuration(&self) -> ([u8; 4], u8, Option<[u8; 4]>, u32) {
        crate::runtime::with_runtime(|runtime| {
            let address = (0..runtime.network.interfaces.address_count())
                .filter_map(|index| runtime.network.interfaces.address_nth(index))
                .find(|value| {
                    value.interface_id == 2
                        && value.source == crate::runtime::network::types::AddressSource::Static
                });
            let route = (0..runtime.network.interfaces.route_count())
                .filter_map(|index| runtime.network.interfaces.route_nth(index))
                .find(|value| value.interface_id == 2 && value.prefix_length == 0);
            let octets = match address.map(|value| value.address) {
                Some(crate::runtime::network::types::IpAddress::V4(value)) => value,
                _ => [10, 0, 2, 15],
            };
            let gateway = match route.and_then(|value| value.next_hop) {
                Some(crate::runtime::network::types::IpAddress::V4(value)) => Some(value),
                _ => None,
            };
            (
                octets,
                address.map(|value| value.prefix_length).unwrap_or(24),
                gateway,
                route.map(|value| value.metric).unwrap_or(100),
            )
        })
        .unwrap_or(([10, 0, 2, 15], 24, None, 100))
    }

    // ------------------------=
    // FUNC: commit_network_edit
    // DESC: Validates and commits one edited address or resolver field through typed Settings authority.
    // ------------------=
    fn commit_network_edit(&mut self) -> bool {
        let page = self.network_page();
        let control = self.settings_window.control_focus.min(5);
        let input = &self.command[..self.command_length];
        if page == 2 {
            let (mut address, mut prefix, mut gateway, mut metric) =
                self.network_static_configuration();
            match control {
                1 => {
                    address = match parse_ipv4(input) {
                        Some(value) => value,
                        None => return false,
                    }
                }
                2 => {
                    prefix = match parse_bounded_number(input, 32) {
                        Some(value) => value as u8,
                        None => return false,
                    }
                }
                3 => {
                    gateway = match parse_ipv4(input) {
                        Some(value) => Some(value),
                        None => return false,
                    }
                }
                4 => {
                    metric = match parse_bounded_number(input, 65_535) {
                        Some(value) => value,
                        None => return false,
                    }
                }
                _ => return false,
            }
            return crate::runtime::configure_static_ipv4_from_settings(
                address, prefix, gateway, metric, 0,
            );
        }
        if page == 3 && matches!(control, 1 | 2) {
            let Some(server) = parse_ipv4(input) else {
                return false;
            };
            let (enabled, mut primary, mut secondary) = crate::runtime::with_runtime(|runtime| {
                let to_v4 = |value| match value {
                    Some(crate::runtime::network::types::IpAddress::V4(octets)) => Some(octets),
                    _ => None,
                };
                (
                    runtime.network.resolver.enabled(),
                    to_v4(runtime.network.resolver.server(0)),
                    to_v4(runtime.network.resolver.server(1)),
                )
            })
            .unwrap_or((true, None, None));
            if control == 1 {
                primary = Some(server);
            } else {
                secondary = Some(server);
            }
            return crate::runtime::configure_resolver_from_settings(
                enabled, primary, secondary, 0,
            );
        }
        false
    }

    // ------------------------=
    // FUNC: activate_network_control
    // DESC: Executes the focused Network page control through typed runtime configuration operations.
    // ------------------=
    fn activate_network_control(&mut self, control: usize) {
        self.settings_window.control_focus = control.min(5);
        match (self.network_page(), control.min(5)) {
            (0, index @ 0..=3) => {
                let mode = [
                    crate::runtime::network::types::NetworkSetupMode::Automatic,
                    crate::runtime::network::types::NetworkSetupMode::Wired,
                    crate::runtime::network::types::NetworkSetupMode::Wireless,
                    crate::runtime::network::types::NetworkSetupMode::Offline,
                ][index];
                let _ =
                    crate::runtime::reconfigure_network_from_settings(mode, 0, index as u64 + 1);
            }
            (1, 1) => {
                let enabled = crate::runtime::with_runtime(|runtime| {
                    runtime
                        .network
                        .interfaces
                        .interface(2)
                        .map(|value| value.enabled)
                })
                .flatten()
                .unwrap_or(false);
                let _ = crate::runtime::set_network_interface_from_settings(!enabled, 0);
            }
            (2, 0) => {
                let _ = crate::runtime::configure_dynamic_ipv4_from_settings(0);
            }
            (2, 1..=4) | (3, 1..=2) => {
                self.settings_editing = true;
                self.reset_input();
            }
            (3, 0) => {
                let (enabled, primary, secondary) = crate::runtime::with_runtime(|runtime| {
                    let to_v4 = |value| match value {
                        Some(crate::runtime::network::types::IpAddress::V4(octets)) => Some(octets),
                        _ => None,
                    };
                    (
                        runtime.network.resolver.enabled(),
                        to_v4(runtime.network.resolver.server(0)),
                        to_v4(runtime.network.resolver.server(1)),
                    )
                })
                .unwrap_or((true, None, None));
                let _ = crate::runtime::configure_resolver_from_settings(
                    !enabled, primary, secondary, 0,
                );
            }
            (3, 3) => {
                let _ = crate::runtime::configure_resolver_from_settings(true, None, None, 0);
            }
            (4, 5) => {
                let _ = crate::runtime::remove_default_network_route_from_settings(0);
            }
            (5, index @ 0..=4) => self.activate_settings_content_row(index),
            (5, 5) => self.activate_settings_content_row(0),
            (6, 0) => {
                let current =
                    crate::runtime::with_runtime(|runtime| runtime.network.policy.default_action())
                        .unwrap_or(crate::runtime::network::types::PolicyAction::Deny);
                let next = match current {
                    crate::runtime::network::types::PolicyAction::Deny => {
                        crate::runtime::network::types::PolicyAction::Ask
                    }
                    crate::runtime::network::types::PolicyAction::Ask => {
                        crate::runtime::network::types::PolicyAction::Allow
                    }
                    _ => crate::runtime::network::types::PolicyAction::Deny,
                };
                let _ = crate::runtime::set_network_default_policy_from_settings(next, 0);
            }
            (6, 5) => {
                let _ = crate::runtime::set_network_default_policy_from_settings(
                    crate::runtime::network::types::PolicyAction::Deny,
                    0,
                );
            }
            _ => {}
        }
    }

    // ------------------------=
    // FUNC: select_next_node
    // DESC: Changes the operator selection by stable NodeId only after an explicit selection action.
    // ------------------=
    fn select_next_node(&mut self) {
        self.selected_node_id = crate::runtime::with_runtime(|runtime| {
            let mut found_current = self.selected_node_id.is_none();
            for node in runtime.nodes.discovered_nodes().iter().flatten() {
                if found_current {
                    return Some(node.id);
                }
                if Some(node.id) == self.selected_node_id {
                    found_current = true;
                }
            }
            runtime
                .nodes
                .discovered_nodes()
                .iter()
                .flatten()
                .next()
                .map(|node| node.id)
        })
        .flatten();
    }

    // ------------------------=
    // FUNC: confirm_selected_node_pairing
    // DESC: Confirms the selected pairing only from six digits manually entered through protected Trusted UI.
    // ------------------=
    fn confirm_selected_node_pairing(&mut self) -> bool {
        if self.command_length != 6
            || self.command[..self.command_length]
                .iter()
                .any(|byte| !byte.is_ascii_digit())
        {
            return false;
        }
        let mut code = 0u32;
        for byte in &self.command[..self.command_length] {
            code = code.saturating_mul(10).saturating_add((byte - b'0') as u32);
        }
        let selected = self.selected_node_id;
        let changed = crate::runtime::with_runtime(|runtime| {
            let now = runtime
                .nodes
                .audit_records()
                .iter()
                .flatten()
                .map(|record| record.timestamp)
                .max()
                .unwrap_or(0)
                .saturating_add(1);
            let Some(pairing) = runtime
                .nodes
                .pairings()
                .iter()
                .flatten()
                .find(|pairing| {
                    Some(pairing.peer) == selected
                        && pairing.state
                            == crate::runtime::node::types::PairingState::AwaitingConfirmation
                })
                .copied()
            else {
                return false;
            };
            let Ok(lease) = runtime.ui.trusted.acquire_secure_input(
                true,
                1,
                crate::ui::trusted::TrustedSurface::NodePairing,
                now.saturating_add(60),
            ) else {
                return false;
            };
            let confirmed = runtime
                .nodes
                .confirm_pairing(pairing.id, code, true, now, now)
                .is_ok();
            let _ = runtime.ui.trusted.release_secure_input(lease);
            confirmed
        })
        .unwrap_or(false);
        if changed {
            let committed = crate::runtime::persist_node_state();
            if committed {
                if let Some(node_id) = selected {
                    let _ = crate::runtime::publish_node_state_event(
                        crate::runtime::EVENT_NODE_PAIRED,
                        node_id,
                        code as u64,
                        code as u64,
                    );
                }
            }
        }
        changed
    }

    // ------------------------=
    // FUNC: activate_node_control
    // DESC: Executes one node-management control against the explicitly selected stable NodeId.
    // ------------------=
    fn activate_node_control(&mut self, control: usize) {
        use crate::runtime::node::types::{MeshRole, NodeTrustPolicy, PolicyDecision, TrustState};

        let page = self.node_settings_page();
        let control = control.min(5);
        self.settings_window.control_focus = control;
        if matches!((page, control), (0, 0) | (1, 0) | (2, 0)) {
            self.select_next_node();
            return;
        }
        if page == 1 && control == 3 {
            let has_pending = crate::runtime::with_runtime(|runtime| {
                runtime.nodes.pairings().iter().flatten().any(|pairing| {
                    Some(pairing.peer) == self.selected_node_id
                        && pairing.state
                            == crate::runtime::node::types::PairingState::AwaitingConfirmation
                })
            })
            .unwrap_or(false);
            if has_pending {
                self.settings_editing = true;
                self.onboarding_validation_error = false;
                self.reset_input();
            }
            return;
        }
        let selected = self.selected_node_id;
        let changed = crate::runtime::with_runtime(|runtime| {
            let now = runtime
                .nodes
                .audit_records()
                .iter()
                .flatten()
                .map(|record| record.timestamp)
                .max()
                .unwrap_or(0)
                .saturating_add(1);
            let peer = selected.and_then(|id| {
                runtime
                    .nodes
                    .discovered_nodes()
                    .iter()
                    .flatten()
                    .find(|node| node.id == id)
                    .copied()
            });
            match (page, control, peer) {
                (0, 1, Some(node))
                    if matches!(node.trust, TrustState::Trusted | TrustState::Restricted) =>
                {
                    runtime.nodes.revoke_trust(node.id, now, now).is_ok()
                }
                (0, 4, Some(node)) => runtime
                    .nodes
                    .set_trust(node.id, TrustState::Blocked, now, now)
                    .is_ok(),
                (0, 5, _) => {
                    runtime.nodes.sweep(now);
                    false
                }
                (1, 1, Some(node))
                    if matches!(node.trust, TrustState::Untrusted | TrustState::Discovered) =>
                {
                    runtime.nodes.begin_pairing(node.id, now).is_ok()
                }
                (1, 4, _) => runtime
                    .nodes
                    .pairings()
                    .iter()
                    .flatten()
                    .find(|pairing| {
                        Some(pairing.peer) == selected
                            && pairing.state
                                == crate::runtime::node::types::PairingState::AwaitingConfirmation
                    })
                    .copied()
                    .map(|pairing| runtime.nodes.cancel_pairing(pairing.id).is_ok())
                    .unwrap_or(false),
                (2, 1, Some(node)) if node.trust == TrustState::Trusted => runtime
                    .nodes
                    .join_mesh(node.id, MeshRole::Member, now, now)
                    .is_ok(),
                (2, 4, Some(node)) => runtime.nodes.leave_mesh(node.id, now, now).is_ok(),
                (3, category @ 0..=4, Some(node)) => {
                    let mut policy = node.policy;
                    policy.categories[category] = match policy.categories[category] {
                        PolicyDecision::Deny => PolicyDecision::SessionOnly,
                        PolicyDecision::SessionOnly => PolicyDecision::Leased,
                        _ => PolicyDecision::Deny,
                    };
                    policy.version = policy.version.saturating_add(1);
                    runtime
                        .nodes
                        .update_policy(node.id, policy, now, now)
                        .is_ok()
                }
                (3, 5, Some(node)) => {
                    let mut policy = NodeTrustPolicy::deny_all();
                    policy.version = node.policy.version.saturating_add(1);
                    runtime
                        .nodes
                        .update_policy(node.id, policy, now, now)
                        .is_ok()
                }
                _ => false,
            }
        })
        .unwrap_or(false);
        if changed {
            let committed = crate::runtime::persist_node_state();
            if committed {
                let event_type = match (page, control) {
                    (0, 1) => crate::runtime::EVENT_NODE_TRUST_REVOKED,
                    (0, 4) => crate::runtime::EVENT_NODE_BLOCKED,
                    (1, 1) => crate::runtime::EVENT_NODE_PAIRING_REQUESTED,
                    (1, 4) => crate::runtime::EVENT_NODE_PAIRING_REJECTED,
                    (2, 1) => crate::runtime::EVENT_NODE_JOINED,
                    (2, 4) => crate::runtime::EVENT_NODE_LEFT,
                    (3, _) => crate::runtime::EVENT_NODE_TRUST_CHANGED,
                    _ => 0,
                };
                if event_type != 0 {
                    if let Some(node_id) = selected {
                        let _ = crate::runtime::publish_node_state_event(
                            event_type,
                            node_id,
                            page as u64 + 1,
                            page as u64 + 1,
                        );
                    }
                }
            }
        }
    }

    // ------------------------=
    // FUNC: activate_settings_content_row
    // DESC: Executes the selected typed settings row without coupling row and navigation focus.
    // ------------------=
    fn activate_settings_content_row(&mut self, row: usize) {
        match (self.system_focus, row) {
            (0, 0) => {
                self.settings_editing = true;
                self.reset_input();
            }
            (1, 0) => {
                let _ = crate::runtime::with_runtime(|runtime| {
                    let dark = crate::ui::skin::SkinId::from_bytes(b"infinity.default.dark");
                    let alternate =
                        crate::ui::skin::SkinId::from_bytes(b"infinity.diagnostic.light");
                    let next = if runtime.ui.skins.active().id == dark {
                        alternate
                    } else {
                        dark
                    };
                    runtime
                        .ui
                        .skins
                        .activate(next, crate::ui::skin::AppearanceScope::User)
                });
            }
            (1, 1) => self.cycle_icon_theme(),
            (1, 2) => self.cycle_primary(),
            (1, 3) => self.cycle_accent(),
            (1, 4 | 5) => self.cycle_background_effect(row),
            (3, 0) => {
                let current = crate::runtime::with_runtime(|runtime| {
                    runtime.identity.ai_profile(self.current_user)
                })
                .flatten();
                if let Some(profile) = current {
                    let next = if profile.provider_policy
                        == crate::runtime::identity::AiProviderPolicy::LocalOnly
                    {
                        crate::runtime::identity::AiProviderPolicy::PreferLocal
                    } else {
                        crate::runtime::identity::AiProviderPolicy::LocalOnly
                    };
                    let _ = crate::runtime::with_runtime(|runtime| {
                        runtime.identity.update_ai_profile(
                            self.current_user,
                            self.current_user,
                            next,
                        )
                    });
                    let _ = crate::runtime::persist_identity_state();
                }
            }
            (3, 1) => {
                let enabled = crate::runtime::ai::with_ai_runtime(|runtime| runtime.chat.enabled());
                self.set_ai_chat_enabled(!enabled);
            }
            (3, 2) => self.select_next_chat_model(),
            (4, 3) => self.cycle_user_no_activity_timeout(),
            (6, profile @ 0..=4) => {
                let profile_id = profile as u32 + 1;
                if crate::runtime::activate_network_profile_from_settings(
                    profile_id,
                    0,
                    profile_id as u64,
                ) {
                    self.settings_window.expanded_row = Some(profile);
                }
            }
            _ => {}
        }
    }

    // ------------------------=
    // FUNC: open_shell_menu
    // DESC: Opens one native top-bar menu and selects its first actionable row.
    // ------------------=
    fn open_shell_menu(&mut self, menu: usize) {
        self.mode = ConsoleMode::SystemMenu;
        self.shell_menu = menu.min(16);
        self.system_focus = 0;
    }

    // ------------------------=
    // FUNC: open_app_launcher
    // DESC: Opens the native installed application launcher with an empty live search query.
    // ------------------=
    fn open_app_launcher(&mut self) {
        self.store_active_app_window();
        crate::ui::app_launcher::launcher_open();
        self.launcher_tick_ns = crate::ui::performance::monotonic_ns();
        self.launcher_scroll_dragging = false;
        self.mode = ConsoleMode::AppLauncher;
        self.system_focus = 0;
        self.reset_input();
    }

    // ------------------------=
    // FUNC: close_app_launcher
    // DESC: Begins the native launcher exit transition while retaining its composited surface.
    // ------------------=
    fn close_app_launcher(&mut self) {
        crate::ui::app_launcher::launcher_begin_close();
    }

    // ------------------------=
    // FUNC: activate_launcher_action
    // DESC: Routes one typed launcher entry into a real Home, Settings, Text Editor, or Command surface.
    // ------------------=
    fn activate_launcher_action(&mut self, action: LauncherAction) {
        match action {
            LauncherAction::Home(location) => {
                self.home_previous_location = self.home_location;
                self.home_location = location.min(8);
                let path = home_location_path(self.home_location);
                let _ = self.open_file_navigator_window(path);
            }
            LauncherAction::Settings(section) => self.open_settings(section),
            LauncherAction::TextEditor => self.open_text_editor(),
            LauncherAction::CommandWindow => self.open_command_window(),
            LauncherAction::TaskManager => self.open_task_manager(),
        }
    }

    // ------------------------=
    // FUNC: activate_launcher_focus
    // DESC: Activates the currently focused filtered application or persistent category card.
    // ------------------=
    fn activate_launcher_focus(&mut self) {
        let query = &self.command[..self.command_length];
        let visible = launcher_visible_count(query);
        if (1..=visible).contains(&self.system_focus) {
            if let Some(entry) = launcher_visible_entry(query, self.system_focus - 1) {
                self.activate_launcher_action(entry.action);
            }
            return;
        }
        let category = self.system_focus.saturating_sub(visible + 1);
        if let Some(entry) = LAUNCHER_CATEGORIES.get(category) {
            self.activate_launcher_action(entry.action);
        }
    }

    // ------------------------=
    // FUNC: reveal_launcher_focus
    // DESC: Smoothly reveals a keyboard-focused application row inside the bounded grid viewport.
    // ------------------=
    fn reveal_launcher_focus(&self, visible: usize) {
        if !(1..=visible).contains(&self.system_focus) {
            return;
        }
        let layout = SystemLayout::new(
            self.system.framebuffer_width,
            self.system.framebuffer_height,
        );
        let geometry = layout.app_launcher_geometry();
        let scroll = layout.app_launcher_scroll_geometry(visible);
        let row = (self.system_focus - 1) / crate::ui::app_launcher::LAUNCHER_COLUMNS;
        let top = row.saturating_mul(geometry.grid_row_height);
        let bottom = top.saturating_add(geometry.grid_row_height);
        let current = crate::ui::app_launcher::launcher_presentation().scroll;
        let viewport = geometry.grid_viewport.height as usize;
        let desired = if top < current {
            top
        } else if bottom > current.saturating_add(viewport) {
            bottom.saturating_sub(viewport)
        } else {
            current
        };
        let _ = crate::ui::app_launcher::launcher_scroll_to(desired, scroll.maximum_scroll);
    }

    // ------------------------=
    // FUNC: shell_menu_item_count
    // DESC: Returns the bounded row count for the currently open native menu.
    // ------------------=
    fn shell_menu_item_count(&self) -> usize {
        if self.shell_menu >= 8 {
            return crate::ui::status_menu::items(self.shell_menu).len();
        }
        match self.shell_menu {
            1 => 5,
            2 => 6,
            3 => 5,
            4 | 5 => 4,
            _ => 10,
        }
    }

    // ------------------------=
    // FUNC: show_shell_notice
    // DESC: Opens Infinity Console with a visible explanation for a context-sensitive menu action.
    // ------------------=
    fn show_shell_notice(&mut self, notice: &[u8]) {
        self.enter_console();
        self.output.write_line(notice);
    }

    // ------------------------=
    // FUNC: activate_shell_menu_item
    // DESC: Executes the selected menu command through real shell, settings, session, or firmware behavior.
    // ------------------=
    fn activate_shell_menu_item(&mut self) {
        if self.shell_menu >= 8 {
            use crate::ui::status_menu::Action;
            if let Some((_, action)) =
                crate::ui::status_menu::items(self.shell_menu).get(self.system_focus)
            {
                match *action {
                    Action::Settings(section) => self.open_settings(section),
                    Action::Devices(row) => {
                        self.open_settings(5);
                        self.toggle_settings_row(row);
                    }
                    Action::Launcher => self.open_app_launcher(),
                    Action::Files => {
                        self.enter_desktop();
                        let _ = self.open_file_navigator_window(b"/home/default");
                    }
                    Action::Lock => {
                        let _ = self.lock_session_preserving_desktop(false);
                    }
                    Action::Restart | Action::Shutdown => {
                        self.shell_menu = 0;
                        self.system_focus = if *action == Action::Restart { 8 } else { 9 };
                        self.activate_shell_menu_item();
                    }
                    Action::PreviousMonth | Action::Today | Action::NextMonth => {
                        crate::ui::status_menu::navigate(match action {
                            Action::PreviousMonth => -1,
                            Action::NextMonth => 1,
                            _ => 0,
                        });
                        self.shell_menu = if self.shell_menu == 15 { 16 } else { 15 };
                    }
                }
            }
            return;
        }
        match (self.shell_menu, self.system_focus) {
            (0, 0) => self.open_settings(8),
            (0, 1) => self.open_settings(0),
            (0, 2) => self.open_settings(2),
            (0, 3) => self.open_settings(3),
            (0, 4) => self.open_settings(6),
            (0, 5) => self.open_settings(4),
            (0, 6) => {
                let _ = self.lock_session_preserving_desktop(false);
            }
            (0, 7) => {
                let _ = self.persist_desktop_layout();
                let _ = crate::runtime::with_runtime(|runtime| {
                    runtime
                        .identity
                        .end_session(self.current_session, self.current_user)
                });
                self.current_session = crate::runtime::identity::StableId::zero();
                self.mode = ConsoleMode::Authentication;
                self.system_focus = 0;
                self.reset_input();
                crate::output_text(b"[session] signed out; authentication surface ready\n");
            }
            (0, 8) => {
                let _ = self.persist_desktop_layout();
                crate::output_text(b"[shell] restart requested\n");
                reboot(self.system.firmware_runtime_services);
            }
            (0, 9) => {
                let _ = self.persist_desktop_layout();
                crate::output_text(b"[shell] shutdown requested\n");
                shutdown(self.system.firmware_runtime_services);
            }
            (1, 0) => self.enter_console(),
            (1, 1) => {
                self.home_window_x = 30;
                self.home_window_y = 400;
                self.home_window_visible = true;
                self.enter_desktop();
            }
            (1, 2) => {
                self.home_window_visible = true;
                self.home_previous_location = self.home_location;
                self.home_location = 7;
                self.home_selected_item = Some(5);
                self.enter_desktop();
                crate::output_text(b"[objects] Projects collection opened for new project\n");
            }
            (1, 3) => self.open_settings(0),
            (1, 4) => {
                self.home_window_visible = false;
                self.enter_desktop();
            }
            (2, 0 | 1) => {
                core::mem::swap(
                    &mut self.home_note_location,
                    &mut self.home_note_previous_location,
                );
                self.enter_desktop();
            }
            (2, 2) => {
                if self.home_selected_item == Some(6) {
                    self.home_clipboard_note = true;
                    self.home_note_previous_location = self.home_note_location;
                    self.home_note_location = 9;
                }
                self.enter_desktop();
            }
            (2, 3) => {
                self.home_clipboard_note = self.home_selected_item == Some(6);
                self.enter_desktop();
            }
            (2, 4) => {
                if self.home_clipboard_note {
                    self.home_note_previous_location = self.home_note_location;
                    self.home_note_location = self.home_location;
                    self.home_selected_item = Some(6);
                }
                self.enter_desktop();
            }
            (2, 5) => {
                self.home_selected_item =
                    (self.home_note_location == self.home_location).then_some(6);
                self.enter_desktop();
            }
            (3, 0) => {
                self.home_window_visible = true;
                self.enter_desktop();
            }
            (3, 1) => {
                self.home_window_x = 30;
                self.home_window_y = 400;
                self.home_window_visible = true;
                self.home_window_maximized = false;
                self.enter_desktop();
            }
            (3, 2) => self.enter_desktop(),
            (3, 3) => {
                self.desktop_clock = firmware_date_time(self.system.firmware_runtime_services);
                self.enter_desktop();
            }
            (3, 4) => self.open_settings(1),
            (4, 0) => {
                self.home_window_visible = false;
                self.enter_desktop();
            }
            (4, 1) => {
                self.home_window_visible = true;
                self.home_window_maximized = false;
                self.home_window_x = self.home_window_restore_x;
                self.home_window_y = self.home_window_restore_y;
                self.enter_desktop();
            }
            (4, 2) => {
                self.home_window_visible = true;
                self.home_window_maximized = false;
                self.home_window_x = 30;
                self.home_window_y = 400;
                self.enter_desktop();
            }
            (4, 3) => self.open_settings(0),
            (5, 0 | 3) => self.open_settings(8),
            (5, 1) => self.show_shell_notice(
                b"Keyboard: Tab and arrows move focus. Enter selects. Escape closes.",
            ),
            (5, 2) => {
                self.enter_console();
                self.system_status();
            }
            _ => self.enter_desktop(),
        }
    }

    // ------------------------=
    // FUNC: activate_status_item
    // DESC: Routes one top-bar status control to truthful settings, search, or menu functionality.
    // ------------------=
    fn activate_status_item(&mut self, item: usize) {
        if item == 7 {
            crate::ui::status_menu::navigate(0);
        }
        let menu = 8 + item.min(7);
        if self.mode == ConsoleMode::SystemMenu
            && (self.shell_menu == menu || item == 7 && self.shell_menu == 16)
        {
            self.enter_desktop();
        } else {
            self.open_shell_menu(menu);
        }
    }

    // ------------------------=
    // FUNC: edit_system_text
    // DESC: Applies bounded private text editing without sending credentials to logs or events.
    // ------------------=
    fn edit_system_text(&mut self, key: ConsoleKey) -> bool {
        match key {
            ConsoleKey::Character(character) => crate::ui::text_input::insert_ascii(
                &mut self.command,
                &mut self.command_length,
                &mut self.command_cursor,
                character,
            ),
            ConsoleKey::Backspace => crate::ui::text_input::backspace(
                &mut self.command,
                &mut self.command_length,
                &mut self.command_cursor,
            ),
            ConsoleKey::Delete => crate::ui::text_input::delete(
                &mut self.command,
                &mut self.command_length,
                &mut self.command_cursor,
            ),
            ConsoleKey::Left => {
                crate::ui::text_input::move_caret(&mut self.command_cursor, self.command_length, -1)
            }
            ConsoleKey::Right => {
                crate::ui::text_input::move_caret(&mut self.command_cursor, self.command_length, 1)
            }
            ConsoleKey::Home => {
                crate::ui::text_input::move_caret(&mut self.command_cursor, self.command_length, -2)
            }
            ConsoleKey::End => {
                crate::ui::text_input::move_caret(&mut self.command_cursor, self.command_length, 2)
            }
            _ => false,
        }
    }

    // ------------------------=
    // FUNC: select_onboarding_network
    // DESC: Stages one pointer or keyboard-selected first-boot network mode through the native runtime.
    // ------------------=
    fn select_onboarding_network(&mut self, index: usize) -> bool {
        let mode = match index {
            0 => crate::runtime::network::types::NetworkSetupMode::Wired,
            1 => crate::runtime::network::types::NetworkSetupMode::Wireless,
            _ => crate::runtime::network::types::NetworkSetupMode::Offline,
        };
        crate::runtime::select_network_mode_from_onboarding(mode)
    }

    // ------------------------=
    // FUNC: input_onboarding
    // DESC: Advances modular first-boot steps and commits identity and connectivity changes.
    // ------------------=
    fn input_onboarding(&mut self, key: ConsoleKey) {
        if self.system_step == 6 {
            if matches!(key, ConsoleKey::Tab(_)) {
                let reverse = matches!(key, ConsoleKey::Tab(true));
                self.system_focus = match (reverse, self.system_focus) {
                    (false, 0) => 2,
                    (false, 2) => 3,
                    (false, 3) => 4,
                    (false, 4) => 1,
                    (false, _) => 0,
                    (true, 0) => 1,
                    (true, 1) => 4,
                    (true, 4) => 3,
                    (true, 3) => 2,
                    (true, _) => 0,
                };
                return;
            }
            if matches!(key, ConsoleKey::Up | ConsoleKey::Down) {
                self.system_focus = match (matches!(key, ConsoleKey::Up), self.system_focus) {
                    (true, 2) => 4,
                    (true, 3) => 2,
                    (true, _) => 3,
                    (false, 2) => 3,
                    (false, 3) => 4,
                    (false, _) => 2,
                };
                return;
            }
            if matches!(key, ConsoleKey::Left | ConsoleKey::Right) {
                self.system_focus = if matches!(key, ConsoleKey::Left) {
                    0
                } else {
                    1
                };
                return;
            }
            if matches!(key, ConsoleKey::Enter) && (2..=4).contains(&self.system_focus) {
                self.onboarding_validation_error =
                    !self.select_onboarding_network(self.system_focus - 2);
                if !self.onboarding_validation_error {
                    self.system_focus = 1;
                }
                return;
            }
        }
        if crate::ui::installer_layout::configuration_template_input_variable(self.system_step)
            != crate::ui::installer_template::InstallerTemplateVariable::None
            && self.system_focus == 1
            && self.edit_system_text(key)
        {
            self.onboarding_validation_error = false;
            return;
        }
        if matches!(
            key,
            ConsoleKey::Tab(_) | ConsoleKey::Left | ConsoleKey::Right
        ) && self.system_step > 0
            && self.system_step != 6
        {
            let reverse = matches!(key, ConsoleKey::Tab(true) | ConsoleKey::Left);
            self.system_focus = if reverse {
                if self.system_focus == 0 {
                    1
                } else {
                    0
                }
            } else if self.system_focus == 1 {
                0
            } else {
                1
            };
            return;
        }
        if matches!(key, ConsoleKey::Escape) && self.system_step > 0 {
            self.system_step -= 1;
            self.system_focus = 1;
            self.onboarding_validation_error = false;
            self.restore_onboarding_input();
            return;
        }
        if !matches!(key, ConsoleKey::Enter) {
            return;
        }
        if self.system_step > 0 && self.system_focus == 0 {
            self.system_step -= 1;
            self.system_focus = 1;
            self.onboarding_validation_error = false;
            self.restore_onboarding_input();
            return;
        }
        match self.system_step {
            0 => self.system_step = 1,
            1..=4 => {
                let variable = crate::ui::installer_layout::configuration_template_input_variable(
                    self.system_step,
                );
                if !self.commit_onboarding_input(variable)
                    || !self.materialize_onboarding_identity()
                {
                    self.onboarding_validation_error = true;
                    return;
                }
                self.system_step += 1;
            }
            5 => {
                let selected = crate::runtime::with_runtime(|runtime| {
                    let state = runtime.network.setup_snapshot();
                    if state.wired_available {
                        crate::runtime::network::types::NetworkSetupMode::Wired
                    } else if state.wireless_available {
                        crate::runtime::network::types::NetworkSetupMode::Wireless
                    } else {
                        crate::runtime::network::types::NetworkSetupMode::Offline
                    }
                })
                .unwrap_or(crate::runtime::network::types::NetworkSetupMode::Offline);
                let _ = crate::runtime::select_network_mode_from_onboarding(selected);
                self.system_step = 6;
            }
            6 => {
                if !crate::runtime::apply_network_mode_from_onboarding(0, 0x4f4e_424f_4152_4401) {
                    self.onboarding_validation_error = true;
                    return;
                }
                self.system_step = 7;
            }
            _ => {
                if !self.materialize_onboarding_identity() {
                    self.onboarding_validation_error = true;
                    return;
                }
                if let Some(variable) = self.missing_onboarding_variable() {
                    self.system_step = crate::ui::installer_layout::configuration_template_step_for_input_variable(variable)
                        .unwrap_or(1);
                    self.system_focus = 1;
                    self.onboarding_validation_error = true;
                    self.restore_onboarding_input();
                    return;
                }
                let session = crate::runtime::with_runtime(|runtime| {
                    runtime.identity.complete_onboarding()?;
                    runtime.identity.create_session(
                        self.current_user,
                        &self.onboarding_secret[..self.onboarding_secret_length],
                        4,
                    )
                })
                .unwrap_or(Err(crate::runtime::identity::IdentityError::InvalidState));
                let Ok(session) = session else {
                    return;
                };
                self.current_session = session.id;
                for byte in &mut self.onboarding_secret {
                    *byte = 0;
                }
                self.onboarding_secret_length = 0;
                let _ = crate::runtime::persist_identity_state();
                crate::output_text(
                    b"[onboarding] complete\n[session] authenticated session active\n",
                );
                self.enter_desktop();
                return;
            }
        }
        self.reset_input();
        self.system_focus = if self.system_step == 6 { 2 } else { 1 };
        self.onboarding_validation_error = false;
        let _ = crate::runtime::persist_identity_state();
    }

    // ------------------------=
    // FUNC: commit_onboarding_input
    // DESC: Stores the current field bytes in the runtime variable selected by the saved UI template.
    // ------------------=
    fn commit_onboarding_input(
        &mut self,
        variable: crate::ui::installer_template::InstallerTemplateVariable,
    ) -> bool {
        use crate::ui::installer_template::InstallerTemplateVariable;
        match variable {
            InstallerTemplateVariable::MachineNodeName => {
                if self.command_length == 0 || self.command_length > self.onboarding_machine.len() {
                    return false;
                }
                self.onboarding_machine_length = self.command_length;
                self.onboarding_machine[..self.onboarding_machine_length]
                    .copy_from_slice(&self.command[..self.onboarding_machine_length]);
            }
            InstallerTemplateVariable::ProfileName => {
                if self.command_length == 0 || self.command_length > self.onboarding_handle.len() {
                    return false;
                }
                self.onboarding_handle_length = self.command_length;
                self.onboarding_handle[..self.onboarding_handle_length]
                    .copy_from_slice(&self.command[..self.onboarding_handle_length]);
            }
            InstallerTemplateVariable::DisplayName => {
                if self.command_length == 0 || self.command_length > self.onboarding_name.len() {
                    return false;
                }
                self.onboarding_name_length = self.command_length;
                self.onboarding_name[..self.onboarding_name_length]
                    .copy_from_slice(&self.command[..self.onboarding_name_length]);
            }
            InstallerTemplateVariable::Password => {
                if self.command_length < 8 || self.command_length > self.onboarding_secret.len() {
                    return false;
                }
                self.onboarding_secret_length = self.command_length;
                self.onboarding_secret[..self.onboarding_secret_length]
                    .copy_from_slice(&self.command[..self.onboarding_secret_length]);
            }
            InstallerTemplateVariable::None => return false,
        }
        true
    }

    // ------------------------=
    // FUNC: materialize_onboarding_identity
    // DESC: Creates ready identity objects whenever their template-bound variables have been collected.
    // ------------------=
    fn materialize_onboarding_identity(&mut self) -> bool {
        if crate::runtime::with_runtime(|runtime| runtime.identity.machine().is_none())
            .unwrap_or(true)
            && self.onboarding_machine_length > 0
        {
            let architecture = if self.system.architecture == b"x86_64" {
                2
            } else if self.system.architecture == b"AArch64" {
                3
            } else {
                1
            };
            let created = crate::runtime::with_runtime(|runtime| {
                runtime.identity.create_machine(
                    &self.onboarding_machine[..self.onboarding_machine_length],
                    architecture,
                    1,
                    1,
                )
            })
            .is_some_and(|result| result.is_ok());
            if !created {
                return false;
            }
        }
        if self.current_user.is_zero()
            && self.onboarding_handle_length > 0
            && self.onboarding_name_length > 0
        {
            let created = crate::runtime::with_runtime(|runtime| {
                runtime.identity.create_user(
                    &self.onboarding_handle[..self.onboarding_handle_length],
                    &self.onboarding_name[..self.onboarding_name_length],
                    2,
                )
            });
            let Some(Ok(user)) = created else {
                return false;
            };
            self.current_user = user.id;
        }
        if !self.current_user.is_zero() && self.onboarding_secret_length >= 8 {
            let credential = crate::runtime::with_runtime(|runtime| {
                if runtime.identity.has_active_credential(self.current_user) {
                    runtime
                        .identity
                        .authenticate(
                            self.current_user,
                            &self.onboarding_secret[..self.onboarding_secret_length],
                            3,
                        )
                        .map(|_| ())
                } else {
                    runtime
                        .identity
                        .create_password(
                            self.current_user,
                            &self.onboarding_secret[..self.onboarding_secret_length],
                            3,
                        )
                        .map(|_| ())
                }
            })
            .is_some_and(|result| result.is_ok());
            if !credential {
                return false;
            }
        }
        true
    }

    // ------------------------=
    // FUNC: missing_onboarding_variable
    // DESC: Returns the first required template variable whose durable identity state is incomplete.
    // ------------------=
    fn missing_onboarding_variable(
        &self,
    ) -> Option<crate::ui::installer_template::InstallerTemplateVariable> {
        use crate::ui::installer_template::InstallerTemplateVariable;
        if self.onboarding_machine_length == 0 {
            Some(InstallerTemplateVariable::MachineNodeName)
        } else if self.onboarding_handle_length == 0 {
            Some(InstallerTemplateVariable::ProfileName)
        } else if self.onboarding_name_length == 0 {
            Some(InstallerTemplateVariable::DisplayName)
        } else if self.onboarding_secret_length < 8 {
            Some(InstallerTemplateVariable::Password)
        } else {
            None
        }
    }

    // ------------------------=
    // FUNC: restore_onboarding_input
    // DESC: Restores previously committed first-boot values when navigating backward without exposing secrets externally.
    // ------------------=
    fn restore_onboarding_input(&mut self) {
        self.reset_input();
        let (source, length): (&[u8], usize) =
            match crate::ui::installer_layout::configuration_template_input_variable(
                self.system_step,
            ) {
                crate::ui::installer_template::InstallerTemplateVariable::MachineNodeName => {
                    (&self.onboarding_machine, self.onboarding_machine_length)
                }
                crate::ui::installer_template::InstallerTemplateVariable::ProfileName => {
                    (&self.onboarding_handle, self.onboarding_handle_length)
                }
                crate::ui::installer_template::InstallerTemplateVariable::DisplayName => {
                    (&self.onboarding_name, self.onboarding_name_length)
                }
                crate::ui::installer_template::InstallerTemplateVariable::Password => {
                    (&self.onboarding_secret, self.onboarding_secret_length)
                }
                _ => (&[], 0),
            };
        let copied = length.min(COMMAND_CAPACITY).min(source.len());
        self.command[..copied].copy_from_slice(&source[..copied]);
        self.command_length = copied;
        self.command_cursor = copied;
    }

    // ------------------------=
    // FUNC: input_authentication
    // DESC: Authenticates login or unlock secrets with useful but non-revealing failure behavior.
    // ------------------=
    fn input_authentication(&mut self, key: ConsoleKey) {
        if matches!(key, ConsoleKey::Tab(true)) {
            self.system_focus = (self.system_focus + 10) % 11;
            return;
        }
        if matches!(key, ConsoleKey::Tab(false)) {
            self.system_focus = (self.system_focus + 1) % 11;
            return;
        }
        if self.mode == ConsoleMode::Authentication
            && self.system_focus == 0
            && matches!(
                key,
                ConsoleKey::Up | ConsoleKey::Left | ConsoleKey::Down | ConsoleKey::Right
            )
        {
            let count = crate::runtime::with_runtime(|runtime| runtime.identity.user_count())
                .unwrap_or(0)
                .max(1);
            if matches!(key, ConsoleKey::Up | ConsoleKey::Left) {
                self.system_step = (self.system_step + count - 1) % count;
            } else {
                self.system_step = (self.system_step + 1) % count;
            }
            self.reset_input();
            return;
        }
        if self.system_focus == 1 && self.edit_system_text(key) {
            return;
        }
        if !matches!(key, ConsoleKey::Enter) {
            return;
        }
        match self.system_focus {
            0 => {
                let count = crate::runtime::with_runtime(|runtime| runtime.identity.user_count())
                    .unwrap_or(0)
                    .max(1);
                self.system_step = (self.system_step + 1) % count;
                self.reset_input();
                return;
            }
            3 => {
                crate::output_text(b"[authentication] security key provider unavailable\n");
                return;
            }
            4 if self.mode == ConsoleMode::Authentication => {
                self.enter_onboarding();
                return;
            }
            5 => {
                crate::output_text(
                    b"[authentication] recovery requires a trusted recovery capability\n",
                );
                return;
            }
            6 => {
                crate::output_text(b"[authentication] options available after sign in\n");
                return;
            }
            7 => {
                shutdown(self.system.firmware_runtime_services);
            }
            8 => {
                reboot(self.system.firmware_runtime_services);
            }
            9 => {
                crate::output_text(b"[network] local network settings requested\n");
                return;
            }
            10 => {
                crate::output_text(
                    b"[accessibility] high contrast and keyboard navigation available\n",
                );
                return;
            }
            _ => {}
        }
        if self.command_length == 0 || !matches!(self.system_focus, 1 | 2) {
            self.system_focus = 1;
            return;
        }
        if self.mode == ConsoleMode::Locked {
            let result = crate::runtime::with_runtime(|runtime| {
                runtime.identity.unlock_session(
                    self.current_session,
                    &self.command[..self.command_length],
                    10,
                )
            })
            .unwrap_or(Err(crate::runtime::identity::IdentityError::InvalidState));
            self.reset_input();
            if result.is_ok() {
                self.enter_desktop();
                if let Some(layout) = self.locked_desktop_layout.restore() {
                    self.restore_desktop_layout(layout);
                }
            } else {
                crate::output_text(b"[authentication] verification failed\n");
            }
            return;
        }
        let user =
            crate::runtime::with_runtime(|runtime| runtime.identity.user_nth(self.system_step))
                .flatten();
        let Some(user) = user else {
            self.reset_input();
            return;
        };
        let result = crate::runtime::with_runtime(|runtime| {
            runtime
                .identity
                .create_session(user.id, &self.command[..self.command_length], 10)
        })
        .unwrap_or(Err(crate::runtime::identity::IdentityError::InvalidState));
        self.reset_input();
        if let Ok(session) = result {
            self.current_user = user.id;
            self.current_session = session.id;
            self.enter_desktop();
            let _ = self.restore_persisted_desktop_layout();
        } else {
            crate::output_text(b"[authentication] verification failed\n");
        }
    }

    // ------------------------=
    // FUNC: input_shell
    // DESC: Navigates the top-bar menu and functional Settings pages using shared typed services.
    // ------------------=
    fn input_shell(&mut self, key: ConsoleKey) {
        if self.mode == ConsoleMode::AppLauncher {
            if matches!(key, ConsoleKey::Escape) {
                self.close_app_launcher();
                return;
            }
            if matches!(key, ConsoleKey::Character(b'/')) && self.command_length == 0 {
                self.system_focus = 0;
                return;
            }
            if matches!(
                key,
                ConsoleKey::Character(_)
                    | ConsoleKey::Backspace
                    | ConsoleKey::Delete
                    | ConsoleKey::Home
                    | ConsoleKey::End
            ) {
                if self.edit_system_text(key) {
                    self.system_focus =
                        if launcher_visible_count(&self.command[..self.command_length]) > 0 {
                            1
                        } else {
                            0
                        };
                    let visible = launcher_visible_count(&self.command[..self.command_length]);
                    let _ = crate::ui::app_launcher::launcher_scroll_to(0, 0);
                    self.reveal_launcher_focus(visible);
                }
                return;
            }
            let visible = launcher_visible_count(&self.command[..self.command_length]);
            let focus_count = visible + LAUNCHER_CATEGORIES.len() + 1;
            if matches!(
                key,
                ConsoleKey::Up | ConsoleKey::Left | ConsoleKey::Tab(true)
            ) {
                self.system_focus = (self.system_focus + focus_count - 1) % focus_count;
                self.reveal_launcher_focus(visible);
                return;
            }
            if matches!(
                key,
                ConsoleKey::Down | ConsoleKey::Right | ConsoleKey::Tab(false)
            ) {
                self.system_focus = (self.system_focus + 1) % focus_count;
                self.reveal_launcher_focus(visible);
                return;
            }
            if matches!(key, ConsoleKey::Enter) {
                self.activate_launcher_focus();
            }
            return;
        }
        if self.mode == ConsoleMode::Desktop {
            if matches!(key, ConsoleKey::Character(b'/')) {
                self.open_app_launcher();
            } else if matches!(key, ConsoleKey::Enter | ConsoleKey::Tab(_)) {
                self.open_shell_menu(0);
            }
            return;
        }
        if self.mode == ConsoleMode::Settings && self.system_focus == 6 && self.settings_editing {
            if matches!(key, ConsoleKey::Escape) {
                self.settings_editing = false;
                self.reset_input();
                return;
            }
            if matches!(key, ConsoleKey::Enter) {
                if self.command_length > 0 && self.commit_network_edit() {
                    self.settings_editing = false;
                    self.onboarding_validation_error = false;
                    self.reset_input();
                } else {
                    self.onboarding_validation_error = true;
                }
                return;
            }
            let _ = self.edit_system_text(key);
            return;
        }
        if self.mode == ConsoleMode::Settings && self.system_focus == 7 && self.settings_editing {
            if matches!(key, ConsoleKey::Escape) {
                self.settings_editing = false;
                self.onboarding_validation_error = false;
                self.reset_input();
                return;
            }
            if matches!(key, ConsoleKey::Enter) {
                if self.confirm_selected_node_pairing() {
                    self.settings_editing = false;
                    self.onboarding_validation_error = false;
                    self.reset_input();
                } else {
                    self.onboarding_validation_error = true;
                }
                return;
            }
            if matches!(key, ConsoleKey::Character(byte) if byte.is_ascii_digit())
                || matches!(
                    key,
                    ConsoleKey::Backspace
                        | ConsoleKey::Delete
                        | ConsoleKey::Left
                        | ConsoleKey::Right
                )
            {
                let _ = self.edit_system_text(key);
                if self.command_length > 6 {
                    self.command_length = 6;
                    self.command_cursor = self.command_cursor.min(6);
                }
            }
            return;
        }
        if self.mode == ConsoleMode::Settings && self.settings_editing {
            if self.edit_system_text(key) {
                return;
            }
            if matches!(key, ConsoleKey::Escape) {
                self.settings_editing = false;
                self.reset_input();
                return;
            }
            if matches!(key, ConsoleKey::Enter) && self.command_length > 0 {
                let _ = crate::runtime::with_runtime(|runtime| {
                    runtime
                        .identity
                        .update_machine_name(&self.command[..self.command_length])
                });
                let _ = crate::runtime::persist_identity_state();
                self.settings_editing = false;
                self.reset_input();
            }
            return;
        }
        if self.mode == ConsoleMode::Settings && self.system_focus == 6 {
            match key {
                ConsoleKey::Escape => self.enter_desktop(),
                ConsoleKey::Left => {
                    self.settings_window.expanded_row = Some((self.network_page() + 6) % 7);
                    self.settings_window.scroll_offset = 0;
                    self.settings_scroll_target = 0;
                    self.settings_window.control_focus = 0;
                }
                ConsoleKey::Right => {
                    self.settings_window.expanded_row = Some((self.network_page() + 1) % 7);
                    self.settings_window.scroll_offset = 0;
                    self.settings_scroll_target = 0;
                    self.settings_window.control_focus = 0;
                }
                ConsoleKey::Up | ConsoleKey::Tab(true) => {
                    self.settings_window.control_focus =
                        (self.settings_window.control_focus + 5) % 6;
                }
                ConsoleKey::Down | ConsoleKey::Tab(false) => {
                    self.settings_window.control_focus =
                        (self.settings_window.control_focus + 1) % 6;
                }
                ConsoleKey::Enter => {
                    self.activate_network_control(self.settings_window.control_focus)
                }
                _ => {}
            }
            return;
        }
        if self.mode == ConsoleMode::Settings && self.system_focus == 7 {
            match key {
                ConsoleKey::Escape => self.enter_desktop(),
                ConsoleKey::Left => {
                    self.settings_window.expanded_row = Some((self.node_settings_page() + 4) % 5);
                    self.settings_window.scroll_offset = 0;
                    self.settings_scroll_target = 0;
                    self.settings_window.control_focus = 0;
                }
                ConsoleKey::Right => {
                    self.settings_window.expanded_row = Some((self.node_settings_page() + 1) % 5);
                    self.settings_window.scroll_offset = 0;
                    self.settings_scroll_target = 0;
                    self.settings_window.control_focus = 0;
                }
                ConsoleKey::Up | ConsoleKey::Tab(true) => {
                    self.settings_window.control_focus =
                        (self.settings_window.control_focus + 5) % 6;
                }
                ConsoleKey::Down | ConsoleKey::Tab(false) => {
                    self.settings_window.control_focus =
                        (self.settings_window.control_focus + 1) % 6;
                }
                ConsoleKey::Enter => self.activate_node_control(self.settings_window.control_focus),
                _ => {}
            }
            return;
        }
        if matches!(key, ConsoleKey::Escape) {
            self.enter_desktop();
            return;
        }
        if matches!(
            key,
            ConsoleKey::Up | ConsoleKey::Left | ConsoleKey::Tab(true)
        ) {
            let count = if self.mode == ConsoleMode::SystemMenu {
                self.shell_menu_item_count()
            } else {
                11
            };
            self.system_focus = (self.system_focus + count - 1) % count;
            if self.mode == ConsoleMode::Settings {
                self.settings_window.row_count = if matches!(self.system_focus, 1 | 6 | 7 | 10) {
                    8
                } else if self.system_focus == 3 {
                    7
                } else {
                    5
                };
                self.settings_window.expanded_row = matches!(self.system_focus, 6 | 7).then_some(0);
                self.settings_window.scroll_offset = 0;
                self.settings_scroll_target = 0;
            }
            return;
        }
        if matches!(
            key,
            ConsoleKey::Down | ConsoleKey::Right | ConsoleKey::Tab(false)
        ) {
            let count = if self.mode == ConsoleMode::SystemMenu {
                self.shell_menu_item_count()
            } else {
                11
            };
            self.system_focus = (self.system_focus + 1) % count;
            if self.mode == ConsoleMode::Settings {
                self.settings_window.row_count = if matches!(self.system_focus, 1 | 6 | 7 | 10) {
                    8
                } else if self.system_focus == 3 {
                    7
                } else {
                    5
                };
                self.settings_window.expanded_row = matches!(self.system_focus, 6 | 7).then_some(0);
                self.settings_window.scroll_offset = 0;
                self.settings_scroll_target = 0;
            }
            return;
        }
        if !matches!(key, ConsoleKey::Enter) {
            return;
        }
        if self.mode == ConsoleMode::Settings {
            if self.system_focus == 0 {
                self.activate_settings_content_row(0);
            }
            if self.system_focus == 1 {
                self.activate_settings_content_row(0);
                crate::output_text(b"[appearance] skin transaction committed\n");
            }
            if self.system_focus == 3 {
                self.activate_settings_content_row(0);
            }
            if self.system_focus == 4 {
                if self.settings_window.expanded_row == Some(3) {
                    self.activate_settings_content_row(3);
                } else {
                    self.toggle_settings_row(3);
                }
            }
            return;
        }
        self.activate_shell_menu_item();
    }

    // ------------------------=
    // FUNC: input_installer
    // DESC: Handles input installer input or state transitions.
    // ------------------=
    fn input_installer(&mut self, key: ConsoleKey) {
        if matches!(key, ConsoleKey::Help) {
            if self.installer_step == InstallerStep::Help {
                self.installer_step = self.installer_return_step;
            } else if self.installer_step != InstallerStep::Provisioning {
                self.installer_return_step = self.installer_step;
                self.installer_step = InstallerStep::Help;
            }
            self.installer_focus = self.default_installer_focus();
            self.render_installer();
            crate::output_text(b"[installer] help toggled\n");
            return;
        }
        if self.installer_step == InstallerStep::Confirm && matches!(key, ConsoleKey::Escape) {
            self.installer_back();
            self.render_installer();
            return;
        }
        if matches!(key, ConsoleKey::Escape) && self.installer_step != InstallerStep::Provisioning {
            self.show_startup();
            return;
        }
        if self.installer_step == InstallerStep::DateTime
            && matches!(
                key,
                ConsoleKey::Up | ConsoleKey::Down | ConsoleKey::Left | ConsoleKey::Right
            )
            && self.installer_focus >= 2
        {
            if self.installer_focus == 4 {
                self.adjust_installer_time_zone(matches!(key, ConsoleKey::Left | ConsoleKey::Down));
            } else if matches!(key, ConsoleKey::Left | ConsoleKey::Right) {
                self.move_installer_date_time_part(matches!(key, ConsoleKey::Left));
            } else {
                self.adjust_installer_date_time(matches!(key, ConsoleKey::Down));
            }
            crate::output_text(b"[installer] date-time configuration changed\n");
            return;
        }
        if matches!(
            key,
            ConsoleKey::Tab(_)
                | ConsoleKey::Up
                | ConsoleKey::Down
                | ConsoleKey::Left
                | ConsoleKey::Right
        ) {
            let reverse = matches!(
                key,
                ConsoleKey::Tab(true) | ConsoleKey::Up | ConsoleKey::Left
            );
            self.move_installer_focus(reverse);
            return;
        }
        if !matches!(key, ConsoleKey::Enter) {
            return;
        }
        self.activate_installer_focus();
    }

    // ------------------------=
    // FUNC: activate_installer_focus
    // DESC: Initializes activate installer focus state.
    // ------------------=
    fn activate_installer_focus(&mut self) {
        if self.installer_step == InstallerStep::Provisioning {
            return;
        }
        if self.installer_focus == 0 && self.installer_has_back() {
            self.installer_back();
            if self.mode == ConsoleMode::Installer {
                self.render_installer();
            }
            return;
        }
        if self.installer_step == InstallerStep::DateTime && self.installer_focus >= 2 {
            self.installer_focus = if self.installer_focus >= 4 {
                1
            } else {
                self.installer_focus + 1
            };
            crate::output_text(b"[installer] date-time field accepted\n");
            return;
        }
        match self.installer_step {
            InstallerStep::Welcome => self.installer_step = InstallerStep::Hierarchy,
            InstallerStep::Hierarchy => {
                self.storage_device = StorageManager::discover();
                self.installer_step = InstallerStep::Discovery;
            }
            InstallerStep::Discovery if self.storage_device.is_some() => {
                self.installer_step = InstallerStep::Details
            }
            InstallerStep::Details => self.installer_step = InstallerStep::DateTime,
            InstallerStep::DateTime => {
                match self.storage_device.map(|device| {
                    StorageManager::plan_entire_disk(
                        device,
                        StorageProfile::SharedDynamic,
                        self.installer_date_time,
                    )
                }) {
                    Some(Ok(plan)) => {
                        self.storage_plan = Some(plan);
                        self.installer_step = InstallerStep::Preview;
                    }
                    Some(Err(error)) => {
                        self.installer_error = Some(error);
                        report_storage_error(error);
                        self.installer_step = InstallerStep::Failed;
                    }
                    None => {
                        self.installer_error = Some(StorageError::NoDevice);
                        report_storage_error(StorageError::NoDevice);
                        self.installer_step = InstallerStep::Failed;
                    }
                }
            }
            InstallerStep::Preview => {
                self.installer_step = InstallerStep::Confirm;
                crate::output_text(b"[installer] confirmation popup opened\n");
            }
            InstallerStep::Confirm => {
                let authorized =
                    crate::runtime::with_runtime(|runtime| runtime.installer_authorized(0))
                        .unwrap_or(false);
                if !authorized {
                    self.installer_error = Some(StorageError::InvalidPlan);
                    self.installer_step = InstallerStep::Failed;
                    self.output
                        .write_line(b"Installer capability lease is unavailable.");
                    return;
                }
                crate::output_text(b"[installer] destructive confirmation accepted\n");
                crate::runtime::with_runtime(|runtime| {
                    runtime.installer_record(crate::runtime::EVENT_INSTALLER_PLAN_CONFIRMED, 0);
                });
                self.installer_step = InstallerStep::Provisioning;
                self.render_installer();
                self.redraw();
                crate::bootstrap::installer_progress_update(0, b"PREPARING INSTALLATION");
                let mut report_progress = |percent, label: &[u8]| {
                    crate::bootstrap::installer_progress_update(percent, label);
                };
                let result = self
                    .storage_plan
                    .as_ref()
                    .map(|plan| StorageManager::provision_with_progress(plan, &mut report_progress))
                    .unwrap_or(Err(StorageError::NoDevice));
                match result {
                    Ok(()) => {
                        self.installer_step = InstallerStep::Complete;
                        crate::runtime::with_runtime(|runtime| {
                            runtime.installer_record(crate::runtime::EVENT_INSTALLER_COMPLETED, 0);
                            runtime.revoke_installer_authority();
                        });
                    }
                    Err(error) => {
                        self.installer_error = Some(error);
                        report_storage_error(error);
                        self.installer_step = InstallerStep::Failed;
                    }
                }
            }
            InstallerStep::Complete => {
                crate::output_text(b"[install] reboot countdown started\n");
                crate::bootstrap::installer_reboot_countdown();
                crate::output_text(b"[install] firmware reboot requested\n");
                reboot(self.system.firmware_runtime_services);
            }
            InstallerStep::Failed => self.show_startup(),
            _ => {}
        }
        self.installer_focus = self.default_installer_focus();
        self.render_installer();
    }

    // ------------------------=
    // FUNC: installer_has_back
    // DESC: Handles installer has back input or state transitions.
    // ------------------=
    fn installer_has_back(&self) -> bool {
        !matches!(
            self.installer_step,
            InstallerStep::Provisioning | InstallerStep::Complete
        )
    }

    // ------------------------=
    // FUNC: installer_has_primary
    // DESC: Handles installer has primary input or state transitions.
    // ------------------=
    fn installer_has_primary(&self) -> bool {
        !matches!(
            self.installer_step,
            InstallerStep::Provisioning | InstallerStep::Help
        ) && !(self.installer_step == InstallerStep::Discovery && self.storage_device.is_none())
    }

    // ------------------------=
    // FUNC: move_installer_focus
    // DESC: Implements the move installer focus operation.
    // ------------------=
    fn move_installer_focus(&mut self, reverse: bool) {
        if self.installer_step == InstallerStep::DateTime {
            const ORDER: [usize; 5] = [2, 3, 4, 1, 0];
            let current = ORDER
                .iter()
                .position(|value| *value == self.installer_focus)
                .unwrap_or(0);
            let next = if reverse {
                (current + ORDER.len() - 1) % ORDER.len()
            } else {
                (current + 1) % ORDER.len()
            };
            self.installer_focus = ORDER[next];
            if self.installer_focus == 2 && self.installer_date_time_part > 2 {
                self.installer_date_time_part = 0;
            } else if self.installer_focus == 3 && !(3..=4).contains(&self.installer_date_time_part)
            {
                self.installer_date_time_part = 3;
            }
            crate::output_text(b"[installer] focus moved\n");
            return;
        }
        let back = self.installer_has_back();
        let primary = self.installer_has_primary();
        if back && primary {
            self.installer_focus = 1 - self.installer_focus.min(1);
        } else if back {
            self.installer_focus = 0;
        } else if primary {
            self.installer_focus = 1;
        }
        crate::output_text(if self.installer_focus == 0 {
            if reverse {
                b"[installer] focus=back reverse\n"
            } else {
                b"[installer] focus=back\n"
            }
        } else if reverse {
            b"[installer] focus=primary reverse\n"
        } else {
            b"[installer] focus=primary\n"
        });
    }

    // ------------------------=
    // FUNC: default_installer_focus
    // DESC: Implements the default installer focus operation.
    // ------------------=
    fn default_installer_focus(&self) -> usize {
        if self.installer_step == InstallerStep::Confirm {
            // A destructive action must never receive default focus. The user
            // must deliberately move from Cancel to Erase & Install.
            0
        } else if self.installer_step == InstallerStep::DateTime {
            2
        } else if self.installer_has_primary() {
            1
        } else {
            0
        }
    }

    // ------------------------=
    // FUNC: time_zone_choice
    // DESC: Returns the currently selected typed time-zone choice.
    // ------------------=
    fn time_zone_choice(&self) -> TimeZoneChoice {
        TIME_ZONES[self.installer_choice.min(TIME_ZONES.len() - 1)]
    }

    // ------------------------=
    // FUNC: adjust_installer_time_zone
    // DESC: Cycles time-zone choices and updates the persisted typed ID and UTC offset.
    // ------------------=
    fn adjust_installer_time_zone(&mut self, reverse: bool) {
        self.installer_choice = if reverse {
            (self.installer_choice + TIME_ZONES.len() - 1) % TIME_ZONES.len()
        } else {
            (self.installer_choice + 1) % TIME_ZONES.len()
        };
        let zone = self.time_zone_choice();
        self.installer_date_time.time_zone_id = zone.id;
        self.installer_date_time.utc_offset_minutes = zone.offset_minutes;
    }

    // ------------------------=
    // FUNC: select_installer_time_zone_from_map
    // DESC: Resolves a map longitude to the closest supported typed time zone.
    // ------------------=
    fn select_installer_time_zone_from_map(&mut self, normalized_x: i32) {
        let bounded_x = normalized_x.clamp(505, 895);
        let longitude = -180 + (bounded_x - 505) * 360 / 390;
        let mut selected = 0usize;
        let mut distance = i32::MAX;
        for (index, zone) in TIME_ZONES.iter().enumerate() {
            let candidate = (zone.longitude_degrees as i32 - longitude).abs();
            if candidate < distance {
                selected = index;
                distance = candidate;
            }
        }
        self.installer_choice = selected;
        let zone = self.time_zone_choice();
        self.installer_date_time.time_zone_id = zone.id;
        self.installer_date_time.utc_offset_minutes = zone.offset_minutes;
    }

    // ------------------------=
    // FUNC: move_installer_date_time_part
    // DESC: Moves between date or clock components without changing their values.
    // ------------------=
    fn move_installer_date_time_part(&mut self, reverse: bool) {
        if self.installer_focus == 2 {
            let part = self.installer_date_time_part.min(2) as i32;
            self.installer_date_time_part = wrap_installer_value(part, 0, 2, reverse) as usize;
        } else if self.installer_focus == 3 {
            let part = if (3..=4).contains(&self.installer_date_time_part) {
                self.installer_date_time_part as i32
            } else {
                3
            };
            self.installer_date_time_part = wrap_installer_value(part, 3, 4, reverse) as usize;
        }
    }

    // ------------------------=
    // FUNC: adjust_installer_date_time
    // DESC: Adjusts the active calendar or clock component while preserving a valid Gregorian date.
    // ------------------=
    fn adjust_installer_date_time(&mut self, reverse: bool) {
        if self.installer_focus == 2 {
            self.installer_date_time_part = self.installer_date_time_part.min(2);
            match self.installer_date_time_part {
                0 => {
                    self.installer_date_time.year = wrap_installer_value(
                        self.installer_date_time.year as i32,
                        2020,
                        2199,
                        reverse,
                    ) as u16;
                }
                1 => {
                    self.installer_date_time.month =
                        wrap_installer_value(self.installer_date_time.month as i32, 1, 12, reverse)
                            as u8;
                }
                _ => {
                    let maximum = crate::storage::days_in_month(
                        self.installer_date_time.year,
                        self.installer_date_time.month,
                    );
                    self.installer_date_time.day = wrap_installer_value(
                        self.installer_date_time.day as i32,
                        1,
                        maximum as i32,
                        reverse,
                    ) as u8;
                }
            }
            let maximum = crate::storage::days_in_month(
                self.installer_date_time.year,
                self.installer_date_time.month,
            );
            self.installer_date_time.day = self.installer_date_time.day.min(maximum);
        } else if self.installer_focus == 3 {
            if !(3..=4).contains(&self.installer_date_time_part) {
                self.installer_date_time_part = 3;
            }
            if self.installer_date_time_part == 3 {
                self.installer_date_time.hour =
                    wrap_installer_value(self.installer_date_time.hour as i32, 0, 23, reverse)
                        as u8;
            } else {
                self.installer_date_time.minute =
                    wrap_installer_value(self.installer_date_time.minute as i32, 0, 59, reverse)
                        as u8;
            }
            self.installer_date_time.second = 0;
        }
    }

    // ------------------------=
    // FUNC: installer_back
    // DESC: Handles installer back input or state transitions.
    // ------------------=
    fn installer_back(&mut self) {
        self.installer_step = match self.installer_step {
            InstallerStep::Welcome | InstallerStep::Failed => {
                self.show_startup();
                return;
            }
            InstallerStep::Hierarchy => InstallerStep::Welcome,
            InstallerStep::Discovery => InstallerStep::Hierarchy,
            InstallerStep::Details => InstallerStep::Discovery,
            InstallerStep::DateTime => InstallerStep::Details,
            InstallerStep::Preview => InstallerStep::DateTime,
            InstallerStep::Confirm => {
                crate::output_text(b"[installer] confirmation popup cancelled\n");
                InstallerStep::Preview
            }
            InstallerStep::Help => self.installer_return_step,
            step => step,
        };
        self.installer_focus = self.default_installer_focus();
        crate::output_text(b"[installer] action=back\n");
    }

    // ------------------------=
    // FUNC: installer_screen
    // DESC: Handles installer screen input or state transitions.
    // ------------------=
    fn installer_screen(&self) -> u8 {
        match self.installer_step {
            InstallerStep::Welcome => 1,
            InstallerStep::Hierarchy => 2,
            InstallerStep::Discovery => 3,
            InstallerStep::Details => 4,
            InstallerStep::DateTime => 5,
            InstallerStep::Preview => 6,
            InstallerStep::Confirm => 7,
            InstallerStep::Provisioning => 8,
            InstallerStep::Complete => 9,
            InstallerStep::Failed => 10,
            InstallerStep::Help => 11,
        }
    }

    // ------------------------=
    // FUNC: render_installer
    // DESC: Renders render installer to the active display.
    // ------------------=
    fn render_installer(&mut self) {
        self.output.clear();
        match self.installer_step {
            InstallerStep::Welcome => {
                self.output.write_line(b"YOUR NEW SYSTEM STARTS HERE.");
                self.output.write_line(b"LET'S CREATE YOUR INFINITY POOL.");
                self.output.write_line(b"ONE SIMPLE HOME FOR YOUR SYSTEM,");
                self.output
                    .write_line(b"YOUR APPS, AND EVERYTHING YOU CREATE.");
                self.output
                    .write_line(b"WE'LL GUIDE YOU THROUGH EVERY CHOICE.");
                self.output
                    .write_line(b"NOTHING CHANGES UNTIL YOU APPROVE IT.");
            }
            InstallerStep::Help => {
                self.output.write_line(b"SETUP HELP");
                self.output
                    .write_line(b"TAB or ARROWS moves the highlighted choice.");
                self.output
                    .write_line(b"ENTER activates the highlighted choice.");
                self.output
                    .write_line(b"ESC returns to the startup screen.");
                self.output
                    .write_line(b"F1 returns to your previous setup step.");
                self.output
                    .write_line(b"No disk changes occur before final approval.");
            }
            InstallerStep::Hierarchy => {
                self.output
                    .write_line(b"ONE DISK BECOMES ONE INFINITY POOL.");
                self.output
                    .write_line(b"THE POOL ORGANIZES FOUR PROTECTED AREAS:");
                self.output
                    .write_line(b"SYSTEM | PERSONAL | APPLICATIONS | RECOVERY");
            }
            InstallerStep::Discovery => {
                self.output.write_line(b"CHOOSE A DISK");
                if let Some(device) = self.storage_device {
                    self.output.write_segments(&[b"Disk: ", device.model()]);
                    self.output
                        .write_number_suffix(b"Size: ", device.capacity_mib(), b" MiB");
                    self.output.write_segments(&[b"Connection: ", device.bus]);
                    self.output.write_line(if device.has_gpt {
                        b"Contents: Existing partitions found"
                    } else {
                        b"Contents: Empty disk"
                    });
                    self.output.write_line(b"ENTER: Use this disk");
                } else {
                    self.output
                        .write_line(b"No compatible disk found. ESC to go back.");
                }
            }
            InstallerStep::Details => {
                if let Some(device) = self.storage_device {
                    self.output.write_line(b"DISK SUMMARY");
                    self.output.write_segments(&[b"Name: ", device.model()]);
                    self.output
                        .write_number(b"Size in MiB: ", device.capacity_mib());
                    self.output.write_segments(&[b"Connection: ", device.bus]);
                    self.output.write_line(if device.has_gpt {
                        b"Current contents: Existing partitions"
                    } else {
                        b"Current contents: Empty"
                    });
                    self.output.write_line(b"ENTER: Continue to date and time");
                }
            }
            InstallerStep::DateTime => {
                self.output.write_line(b"SET DATE, TIME, AND TIME ZONE");
                self.output
                    .write_line(b"THESE SETTINGS ARE SAVED INTO THE INSTALLED SYSTEM.");
            }
            InstallerStep::Preview => {
                if let Some(plan) = self.storage_plan {
                    self.output
                        .write_line(b"REVIEW INSTALLATION - THIS ERASES THE DISK");
                    self.output
                        .write_segments(&[b"Disk: ", plan.target.model()]);
                    self.output
                        .write_line(b"Creates: EFI boot area + Infinity Container");
                    self.output
                        .write_line(b"Pool areas: System | Personal | Applications | Recovery");
                    self.output.write_date_time(b"Local time: ", plan.date_time);
                    self.output
                        .write_segments(&[b"Time zone: ", self.time_zone_choice().label]);
                }
            }
            InstallerStep::Confirm => {
                if let Some(plan) = self.storage_plan {
                    // Keep the reviewed plan visible behind the modal confirmation.
                    self.output
                        .write_line(b"REVIEW INSTALLATION - THIS ERASES THE DISK");
                    self.output
                        .write_segments(&[b"Disk: ", plan.target.model()]);
                    self.output
                        .write_line(b"Creates: EFI boot area + Infinity Container");
                    self.output
                        .write_line(b"Pool areas: System | Personal | Applications | Recovery");
                    self.output.write_date_time(b"Local time: ", plan.date_time);
                    self.output
                        .write_segments(&[b"Time zone: ", self.time_zone_choice().label]);
                }
            }
            InstallerStep::Provisioning => {
                self.output.write_line(b"PROVISIONING STORAGE...");
                self.output
                    .write_line(b"Writing GPT, EFI boot environment, and container");
                self.output
                    .write_line(b"Creating pool and four shared-capacity Spaces");
                self.output
                    .write_line(b"Installing and verifying InfinityOS components");
            }
            InstallerStep::Complete => {
                self.output.write_line(b"INSTALLATION VERIFIED");
                self.output
                    .write_line(b"[OK] EFI boot + Infinity Container metadata");
                self.output
                    .write_line(b"[OK] Pool: System | Personal | Applications | Recovery");
                self.output.write_line(b"[OK] System Generation 1 ACTIVE");
                self.output
                    .write_line(b"[OK] Kernel + required components + bootloader");
                self.output
                    .write_line(b"ENTER: Reboot into installed InfinityOS");
            }
            InstallerStep::Failed => {
                self.output
                    .write_line(b"ERROR: INSTALLATION COULD NOT BE COMPLETED");
                if let Some(error) = self.installer_error {
                    self.output.write_line(storage_error_text(error));
                }
                self.output
                    .write_line(b"No complete-state marker was written.");
                self.output
                    .write_line(b"Check serial diagnostics. ENTER or ESC to return.");
            }
        }
    }

    // ------------------------=
    // FUNC: edit_input
    // DESC: Implements the edit input operation.
    // ------------------=
    fn edit_input(&mut self, key: ConsoleKey) -> bool {
        self.edit_system_text(key)
    }

    // ------------------------=
    // FUNC: input_startup
    // DESC: Handles input startup input or state transitions.
    // ------------------=
    fn input_startup(&mut self, key: ConsoleKey) {
        if self.edit_input(key) {
            return;
        }
        match key {
            ConsoleKey::Enter => {
                if self.command_length == 1 && self.command[0] == b'1' {
                    self.show_installer();
                } else if self.command_length == 1 && self.command[0] == b'2' {
                    self.enter_repair();
                } else if self.command_length == 1 && self.command[0] == b'3' {
                    self.enter_console();
                } else if &self.command[..self.command_length] == b"install"
                    || (self.command_length == 0 && self.pointer_y < 760)
                {
                    self.show_installer();
                } else if &self.command[..self.command_length] == b"repair"
                    || (self.command_length == 0 && self.pointer_y < 810)
                {
                    self.enter_repair();
                } else if &self.command[..self.command_length] == b"console"
                    || &self.command[..self.command_length] == b"recovery"
                    || self.command_length == 0
                {
                    self.enter_console();
                } else {
                    self.output
                        .write_line(b"Type install, repair, or console; or select an option.");
                    self.reset_input();
                }
            }
            ConsoleKey::Tab(reverse) => self.move_startup_focus(reverse),
            ConsoleKey::Up | ConsoleKey::Left => self.move_startup_focus(true),
            ConsoleKey::Down | ConsoleKey::Right => self.move_startup_focus(false),
            ConsoleKey::Escape => self.reset_input(),
            _ => {}
        }
    }

    // ------------------------=
    // FUNC: move_startup_focus
    // DESC: Implements the move startup focus operation.
    // ------------------=
    fn move_startup_focus(&mut self, reverse: bool) {
        self.pointer_x = 500;
        self.pointer_y = if reverse {
            if self.pointer_y < 760 {
                835
            } else if self.pointer_y < 810 {
                735
            } else {
                785
            }
        } else if self.pointer_y < 760 {
            785
        } else if self.pointer_y < 810 {
            835
        } else {
            735
        };
        crate::output_text(if self.pointer_y < 760 {
            b"[startup] focus=installer\n"
        } else if self.pointer_y < 810 {
            b"[startup] focus=repair\n"
        } else {
            b"[startup] focus=console\n"
        });
    }

    // ------------------------=
    // FUNC: pointer
    // DESC: Implements the pointer operation.
    // ------------------=
    fn pointer(&mut self, delta_x: i16, delta_y: i16, buttons: u8) {
        if matches!(self.mode, ConsoleMode::Console | ConsoleMode::Repair) {
            return;
        }
        let button_changed =
            crate::ui::input_preferences::current().buttons(buttons) != self.pointer_buttons;
        if delta_x == 0 && delta_y == 0 && !button_changed {
            return;
        }
        self.session_idle.note_activity();
        if delta_x != 0 || delta_y != 0 || button_changed {
            crate::bootstrap::note_pointer_activity();
        }
        // Pointer positions use a square 0..1000 coordinate space while the
        // framebuffer is 16:9. One raw count on each physical axis should
        // travel approximately the same number of screen pixels.
        #[cfg(target_arch = "aarch64")]
        let (accelerated_x, accelerated_y) = {
            // Keep one-count movements precise and accelerate only sustained
            // gestures. The previous 6x top gain amplified a drained USB burst
            // into a large jump. Numerators use a /4 fixed-point gain.
            let magnitude = core::cmp::max((delta_x as i32).abs(), (delta_y as i32).abs());
            let gain_numerator = if magnitude <= 2 {
                6
            } else if magnitude <= 6 {
                8
            } else if magnitude <= 16 {
                10
            } else if magnitude <= 32 {
                12
            } else {
                14
            };
            let fixed_x = delta_x as i32 * gain_numerator + self.pointer_x_remainder;
            self.pointer_x_remainder = fixed_x % 4;
            // The normalized pointer space is square but the display is 16:9;
            // the 16/9 term keeps physical X/Y travel isotropic.
            let fixed_y = delta_y as i32 * gain_numerator * 16 + self.pointer_y_remainder;
            self.pointer_y_remainder = fixed_y % 36;
            // USB polling coalesces a short burst of HID reports before this
            // point. Clamping the coalesced result discarded most of a fast
            // gesture, so the guest cursor fell farther behind the host the
            // faster the mouse moved. Preserve the complete reported travel;
            // the final screen-coordinate clamp below still keeps the cursor
            // safely inside the framebuffer.
            (fixed_x / 4, fixed_y / 36)
        };
        #[cfg(not(target_arch = "aarch64"))]
        let accelerated_x = delta_x as i32 * 3;
        #[cfg(not(target_arch = "aarch64"))]
        let accelerated_y = delta_y as i32 * 3;
        let preferences = crate::ui::input_preferences::current();
        let dx = if preferences.acceleration {
            accelerated_x
        } else {
            delta_x as i32 * 2
        };
        let dy = if preferences.acceleration {
            accelerated_y
        } else {
            delta_y as i32 * 2
        };
        let x = dx * preferences.speed as i32 + self.preference_pointer_remainder[0];
        let y = dy * preferences.speed as i32 + self.preference_pointer_remainder[1];
        self.preference_pointer_remainder = [x % 4, y % 4];
        let (accelerated_x, accelerated_y) = (x / 4, y / 4);
        // The pointer is a screen-level device, not a console-panel device.
        // Keep only a small edge inset so the cursor remains visible.
        self.pointer_x = (self.pointer_x + accelerated_x).clamp(8, 992);
        self.pointer_y = (self.pointer_y + accelerated_y).clamp(8, 992);
        self.pointer_interaction(buttons);
    }

    // ------------------------=
    // FUNC: pointer_scroll
    // DESC: Consumes wheel motion inside Settings as content scrolling instead of changing the selected navigation section.
    // ------------------=
    fn pointer_scroll(&mut self, vertical: i8) -> bool {
        if vertical == 0 {
            return false;
        }
        self.session_idle.note_activity();
        if self.mode == ConsoleMode::AppLauncher {
            let layout = SystemLayout::new(
                self.system.framebuffer_width,
                self.system.framebuffer_height,
            );
            let visible = launcher_visible_count(&self.command[..self.command_length]);
            let maximum = layout.app_launcher_scroll_geometry(visible).maximum_scroll;
            let distance = crate::ui::input_preferences::current().wheel(vertical)
                * (24 * layout.scale()) as i32;
            if crate::ui::app_launcher::launcher_scroll_by(distance, maximum) {
                self.redraw();
            }
            return true;
        }
        if self.mode == ConsoleMode::Settings {
            self.scroll_settings(vertical);
            self.redraw();
            return true;
        }
        if self.mode == ConsoleMode::Desktop && self.desktop_app == DesktopAppKind::TextEditor {
            self.scroll_editor(vertical);
            self.redraw();
            return true;
        }
        if self.mode == ConsoleMode::Desktop && self.home_window_visible {
            let state = crate::runtime::with_runtime(|runtime| runtime.file_navigator).flatten();
            let Some(state) = state else {
                return false;
            };
            let scale = (self.system.framebuffer_width as usize / 1000).max(1);
            let (_, _, _, height) = crate::ui::system_layout::SystemLayout::new(
                self.system.framebuffer_width,
                self.system.framebuffer_height,
            )
            .home_window_geometry_sized(
                self.home_window_x,
                self.home_window_y,
                self.home_window_width,
                self.home_window_height,
                self.home_window_maximized,
            );
            let total = navigator_child_count(state.active_namespace_ref.as_bytes());
            let extent = if state.view_mode == crate::runtime::object_navigation::ViewMode::List {
                34 * scale
            } else {
                (self.system.framebuffer_height / 23).max(34) + 40 * scale
            };
            let viewport = height.saturating_sub(150 * scale);
            let _ = crate::runtime::with_runtime(|runtime| {
                runtime.file_navigator.as_mut().map(|navigator| {
                    navigator.scroll_by(
                        crate::ui::input_preferences::current().wheel(vertical) as isize * 12,
                        total,
                        viewport,
                        extent,
                    );
                })
            });
            self.redraw();
            return true;
        }
        false
    }

    // ------------------------=
    // FUNC: activate_file_navigator_context
    // DESC: Dispatches object and background context actions through typed storage operations.
    // ------------------=
    fn activate_file_navigator_context(
        &mut self,
        action: usize,
        state: crate::runtime::object_navigation::FileNavigatorState,
    ) {
        let selected =
            state.context_item != crate::runtime::object_navigation::FILE_NAVIGATOR_NO_SELECTION;
        if selected
            && (state.context_item as usize)
                < crate::runtime::object_navigation::FILE_NAVIGATOR_NAVIGATION_ENTRY_COUNT
        {
            if action == 0 {
                self.open_file_navigator_selection();
            }
            let _ = crate::runtime::with_runtime(|runtime| {
                runtime
                    .file_navigator
                    .as_mut()
                    .map(|navigator| navigator.context_menu_open = false)
            });
            return;
        }
        let entry = if selected {
            navigator_child_nth(
                state.active_namespace_ref.as_bytes(),
                state.context_item as usize,
            )
        } else {
            None
        };
        if let Some(entry) = entry {
            let path = &entry.path[..entry.path_len as usize];
            match action {
                0 => self.open_file_navigator_selection(),
                1 => {
                    let name = crate::runtime::object_navigation::namespace_basename(path);
                    let _ = crate::runtime::with_runtime(|runtime| {
                        runtime
                            .file_navigator
                            .as_mut()
                            .map(|navigator| navigator.begin_rename(name))
                    });
                    return;
                }
                2 => self.copy_file_navigator_entry(path, state.active_namespace_ref.as_bytes()),
                3 => {
                    let _ = crate::storage::trash_move(path);
                }
                _ => {}
            }
        } else {
            match action {
                0 => self.create_file_navigator_folder(state.active_namespace_ref.as_bytes()),
                1 => {
                    let _ = crate::runtime::with_runtime(|runtime| {
                        runtime.file_navigator.as_mut().map(|navigator| {
                            navigator.view_mode = crate::runtime::object_navigation::ViewMode::List
                        })
                    });
                }
                2 => {
                    let _ = crate::runtime::with_runtime(|runtime| {
                        runtime.file_navigator.as_mut().map(|navigator| {
                            navigator.view_mode = crate::runtime::object_navigation::ViewMode::Grid
                        })
                    });
                }
                3 => {
                    let _ = crate::runtime::with_runtime(|runtime| {
                        runtime.file_navigator.as_mut().map(|navigator| {
                            navigator.sort_key = 0;
                            navigator.sort_descending = !navigator.sort_descending;
                        })
                    });
                }
                _ => {}
            }
        }
        let _ = crate::runtime::with_runtime(|runtime| {
            runtime
                .file_navigator
                .as_mut()
                .map(|navigator| navigator.context_menu_open = false)
        });
    }

    // ------------------------=
    // FUNC: activate_file_navigator_menu
    // DESC: Executes one typed app-local File Navigator menu action against live navigator or storage state.
    // ------------------=
    fn activate_file_navigator_menu(
        &mut self,
        menu: crate::runtime::object_navigation::FileNavigatorMenu,
        item: usize,
    ) {
        use crate::runtime::object_navigation::{FileNavigatorAction, FileNavigatorDialog};

        match menu.action(item) {
            Some(FileNavigatorAction::NewWindow) => {
                let path = crate::runtime::with_runtime(|runtime| {
                    runtime
                        .file_navigator
                        .map(|navigator| navigator.active_namespace_ref)
                })
                .flatten()
                .unwrap_or_else(|| {
                    crate::runtime::object_navigation::ByteText::new(b"/home/default").unwrap()
                });
                let _ = self.open_file_navigator_window(path.as_bytes());
            }
            Some(FileNavigatorAction::Settings) => {
                self.checkpoint_active_file_navigator();
                self.open_settings(0);
            }
            Some(FileNavigatorAction::EmptyTrash) => {
                let _ = crate::runtime::with_runtime(|runtime| {
                    runtime
                        .file_navigator
                        .as_mut()
                        .map(|navigator| navigator.open_dialog(FileNavigatorDialog::EmptyTrash))
                });
            }
            Some(FileNavigatorAction::About) => {
                let _ = crate::runtime::with_runtime(|runtime| {
                    runtime
                        .file_navigator
                        .as_mut()
                        .map(|navigator| navigator.open_dialog(FileNavigatorDialog::About))
                });
            }
            Some(FileNavigatorAction::SetPerformance(mode)) => {
                let mode = match mode {
                    crate::runtime::object_navigation::PerformanceMode::Restricted => {
                        crate::runtime::resource_policy::ResourceMode::Restricted
                    }
                    crate::runtime::object_navigation::PerformanceMode::Balanced => {
                        crate::runtime::resource_policy::ResourceMode::Balanced
                    }
                    crate::runtime::object_navigation::PerformanceMode::Expanded => {
                        crate::runtime::resource_policy::ResourceMode::Expanded
                    }
                };
                self.apply_application_resource_mode(
                    crate::runtime::task_manager::IMAGE_FILE_NAVIGATOR,
                    mode,
                );
            }
            Some(FileNavigatorAction::PerformanceSettings) => {
                self.checkpoint_active_file_navigator();
                self.open_settings(0);
            }
            Some(FileNavigatorAction::SetView(view_mode)) => {
                let _ = crate::runtime::with_runtime(|runtime| {
                    runtime.file_navigator.as_mut().map(|navigator| {
                        navigator.view_mode = view_mode;
                        navigator.close_overlays();
                    })
                });
            }
            Some(FileNavigatorAction::TogglePreview) => {
                let _ = crate::runtime::with_runtime(|runtime| {
                    runtime.file_navigator.as_mut().map(|navigator| {
                        navigator.inspector_open = !navigator.inspector_open;
                        navigator.menu_open = None;
                    })
                });
            }
            Some(FileNavigatorAction::Navigate(location)) => {
                self.home_previous_location = self.home_location;
                self.home_location = location;
                let _ = crate::runtime::with_runtime(|runtime| {
                    runtime.file_navigator.as_mut().map(|navigator| {
                        let result = navigator.navigate(home_location_path(location));
                        navigator.menu_open = None;
                        result
                    })
                });
                self.home_selected_item = None;
            }
            Some(FileNavigatorAction::CustomLocation) => {
                let _ = crate::runtime::with_runtime(|runtime| {
                    runtime.file_navigator.as_mut().map(|navigator| {
                        navigator.begin_location_edit();
                        navigator.open_dialog(FileNavigatorDialog::Location);
                        navigator.location_editing = true;
                    })
                });
            }
            Some(FileNavigatorAction::Help) => {
                let _ = crate::runtime::with_runtime(|runtime| {
                    runtime
                        .file_navigator
                        .as_mut()
                        .map(|navigator| navigator.open_dialog(FileNavigatorDialog::Help))
                });
            }
            None => {
                let _ = crate::runtime::with_runtime(|runtime| {
                    runtime
                        .file_navigator
                        .as_mut()
                        .map(|navigator| navigator.close_overlays())
                });
            }
        }
    }

    // ------------------------=
    // FUNC: apply_application_resource_mode
    // DESC: Applies one trusted menu preset to persistent policy and every live context for the application.
    // ------------------=
    fn apply_application_resource_mode(
        &mut self,
        image_identity: u32,
        mode: crate::runtime::resource_policy::ResourceMode,
    ) {
        let _ = crate::runtime::with_runtime(|runtime| {
            let app_id = crate::runtime::resource_policy::AppId(image_identity);
            if runtime
                .resources
                .set_mode_from_trusted_ui(app_id, mode)
                .is_err()
            {
                return;
            }
            let policy = runtime.resources.effective_policy(
                app_id,
                crate::runtime::resource_policy::ApplicationManifestRequest::balanced(),
                crate::runtime::resource_policy::SystemCapacity {
                    logical_compute_units: 4,
                    memory_bytes: 8 * 1024 * 1024,
                    gpu_available: false,
                    npu_available: false,
                },
                crate::runtime::resource_policy::SystemConditions {
                    on_battery: false,
                    low_power: false,
                    thermal_pressure: false,
                    foreground: true,
                },
            );
            let Ok(policy) = policy else { return };
            let mut handles = [None; crate::runtime::execution::MAX_CONTEXTS];
            let mut count = 0usize;
            for index in 0..runtime.execution.count() {
                if let Some(context) = runtime.execution.nth(index) {
                    if context.image_identity == image_identity {
                        handles[count] = Some(context.handle);
                        count += 1;
                    }
                }
            }
            for handle in handles[..count].iter().flatten().copied() {
                let _ = runtime
                    .resources
                    .apply_to_context(&mut runtime.execution, handle, policy);
            }
        });
    }

    // ------------------------=
    // FUNC: activate_file_navigator_dialog
    // DESC: Applies one modal File Navigator action with explicit confirmation for destructive Trash removal.
    // ------------------=
    fn activate_file_navigator_dialog(&mut self, action: usize) {
        use crate::runtime::object_navigation::FileNavigatorDialog;

        let state = crate::runtime::with_runtime(|runtime| runtime.file_navigator).flatten();
        let Some(state) = state else { return };
        match (state.dialog_open, action) {
            (Some(FileNavigatorDialog::EmptyTrash), 0) => {
                let _ = crate::storage::trash_empty();
                let _ = crate::runtime::with_runtime(|runtime| {
                    runtime
                        .file_navigator
                        .as_mut()
                        .map(|navigator| navigator.close_overlays())
                });
            }
            (Some(FileNavigatorDialog::Location), 0) => {
                self.commit_file_navigator_edit(state);
            }
            (Some(FileNavigatorDialog::Location), 2) => {}
            (_, 1) => {
                let _ = crate::runtime::with_runtime(|runtime| {
                    runtime
                        .file_navigator
                        .as_mut()
                        .map(|navigator| navigator.close_overlays())
                });
            }
            _ => {}
        }
    }

    // ------------------------=
    // FUNC: create_file_navigator_folder
    // DESC: Creates a collision-free New Folder namespace in the visible location.
    // ------------------=
    fn create_file_navigator_folder(&mut self, parent: &[u8]) {
        for suffix in 0..100usize {
            let mut name = [0u8; 24];
            let length = if suffix == 0 {
                name[..10].copy_from_slice(b"New Folder");
                10
            } else {
                name[..11].copy_from_slice(b"New Folder ");
                11 + write_decimal(&mut name[11..], suffix)
            };
            let Ok(path) =
                crate::runtime::object_navigation::namespace_child_path(parent, &name[..length])
            else {
                return;
            };
            if crate::storage::namespace_create(path.as_bytes()).is_ok() {
                return;
            }
        }
    }

    // ------------------------=
    // FUNC: copy_file_navigator_entry
    // DESC: Copies an object beside its source with a collision-free Finder-style suffix.
    // ------------------=
    fn copy_file_navigator_entry(&mut self, source: &[u8], parent: &[u8]) {
        let leaf = crate::runtime::object_navigation::namespace_basename(source);
        for suffix in 0..100usize {
            let mut name = [0u8; 64];
            let base = leaf.len().min(48);
            name[..base].copy_from_slice(&leaf[..base]);
            let mut length = base;
            let marker = if suffix == 0 {
                b" copy".as_slice()
            } else {
                b" copy ".as_slice()
            };
            name[length..length + marker.len()].copy_from_slice(marker);
            length += marker.len();
            if suffix != 0 {
                length += write_decimal(&mut name[length..], suffix);
            }
            let Ok(destination) =
                crate::runtime::object_navigation::namespace_child_path(parent, &name[..length])
            else {
                return;
            };
            if crate::storage::object_copy_path(source, destination.as_bytes()).is_ok() {
                return;
            }
        }
    }

    // ------------------------=
    // FUNC: present_continuous_motion
    // DESC: Coalesces held-pointer visual updates onto the display clock while presenting release state immediately.
    // ------------------=
    fn present_continuous_motion(&mut self, released: bool) {
        if self.continuous_motion_frames.request(released) {
            self.redraw();
        }
    }

    // ------------------------=
    // FUNC: pointer_interaction
    // DESC: Handles pointer interaction input or state transitions.
    // ------------------=
    fn pointer_interaction(&mut self, buttons: u8) {
        let buttons = crate::ui::input_preferences::current().buttons(buttons);
        // Activate on the press edge. VirtualBox can consume the release packet
        // used to capture a relative USB pointer, so release-edge activation
        // makes a visibly moving mouse appear unable to click.
        let left_button = buttons & crate::drivers::input::pointer::BUTTON_LEFT != 0;
        let right_button = buttons & crate::drivers::input::pointer::BUTTON_RIGHT != 0;
        let clicked =
            left_button && self.pointer_buttons & crate::drivers::input::pointer::BUTTON_LEFT == 0;
        let right_clicked = right_button
            && self.pointer_buttons & crate::drivers::input::pointer::BUTTON_RIGHT == 0;
        let back_clicked = buttons & crate::drivers::input::pointer::BUTTON_BACK != 0
            && self.pointer_buttons & crate::drivers::input::pointer::BUTTON_BACK == 0;
        let forward_clicked = buttons & crate::drivers::input::pointer::BUTTON_FORWARD != 0
            && self.pointer_buttons & crate::drivers::input::pointer::BUTTON_FORWARD == 0;
        let released = !left_button && self.pointer_pressed;
        self.pointer_pressed = left_button;
        self.pointer_buttons = buttons;
        let layout = SystemLayout::new(
            self.system.framebuffer_width,
            self.system.framebuffer_height,
        );
        if self.mode == ConsoleMode::Desktop && (back_clicked || forward_clicked) {
            let _ = crate::runtime::with_runtime(|runtime| {
                runtime.file_navigator.as_mut().map(|navigator| {
                    if back_clicked {
                        navigator.back()
                    } else {
                        navigator.forward()
                    }
                })
            });
            self.redraw();
            return;
        }
        if self.mode == ConsoleMode::Startup && clicked {
            if (180..=820).contains(&self.pointer_x) && (710..=758).contains(&self.pointer_y) {
                crate::output_text(b"[mouse] Installer selected\n");
                self.show_installer();
            } else if (180..=820).contains(&self.pointer_x) && (760..=808).contains(&self.pointer_y)
            {
                crate::output_text(b"[mouse] Repair selected\n");
                self.enter_repair();
            } else if (180..=820).contains(&self.pointer_x) && (810..=858).contains(&self.pointer_y)
            {
                crate::output_text(b"[mouse] Console selected\n");
                self.enter_console();
            }
        } else if self.mode == ConsoleMode::Installer {
            if self.installer_step == InstallerStep::DateTime
                && (100..=475).contains(&self.pointer_x)
            {
                let field = if (465..=535).contains(&self.pointer_y) {
                    Some(2)
                } else if (555..=625).contains(&self.pointer_y) {
                    Some(3)
                } else if (645..=715).contains(&self.pointer_y) {
                    Some(4)
                } else {
                    None
                };
                if clicked {
                    if let Some(field) = field {
                        self.installer_focus = field;
                        if field == 2 && (170..=405).contains(&self.pointer_x) {
                            self.installer_date_time_part = if self.pointer_x < 248 {
                                0
                            } else if self.pointer_x < 326 {
                                1
                            } else {
                                2
                            };
                        } else if field == 3 && (170..=405).contains(&self.pointer_x) {
                            self.installer_date_time_part =
                                if self.pointer_x < 288 { 3 } else { 4 };
                        }
                        if self.pointer_x <= 155 {
                            if field == 4 {
                                self.adjust_installer_time_zone(true);
                            } else {
                                self.adjust_installer_date_time(true);
                            }
                        } else if self.pointer_x >= 420 {
                            if field == 4 {
                                self.adjust_installer_time_zone(false);
                            } else {
                                self.adjust_installer_date_time(false);
                            }
                        }
                    }
                }
            }
            if self.installer_step == InstallerStep::DateTime
                && clicked
                && (500..=900).contains(&self.pointer_x)
                && (405..=770).contains(&self.pointer_y)
            {
                self.installer_focus = 4;
                self.select_installer_time_zone_from_map(self.pointer_x);
                crate::output_text(b"[installer] time-zone selected from map\n");
            }
            let popup = self.installer_step == InstallerStep::Confirm;
            let confirmation_target = if popup {
                crate::ui::installer_layout::installer_confirmation_target(
                    self.pointer_x,
                    self.pointer_y,
                )
            } else {
                None
            };
            let welcome = self.installer_step == InstallerStep::Welcome;
            let hierarchy = self.installer_step == InstallerStep::Hierarchy;
            let over_back = self.installer_has_back()
                && if popup {
                    confirmation_target
                        == Some(crate::ui::installer_layout::InstallerConfirmationTarget::Cancel)
                } else if welcome {
                    (145..=470).contains(&self.pointer_x) && (810..=865).contains(&self.pointer_y)
                } else if hierarchy {
                    (135..=485).contains(&self.pointer_x) && (820..=875).contains(&self.pointer_y)
                } else {
                    (190..=490).contains(&self.pointer_x) && (830..=885).contains(&self.pointer_y)
                };
            let over_primary = self.installer_has_primary()
                && if popup {
                    confirmation_target
                        == Some(crate::ui::installer_layout::InstallerConfirmationTarget::Install)
                } else if welcome {
                    (480..=830).contains(&self.pointer_x) && (810..=865).contains(&self.pointer_y)
                } else if hierarchy {
                    (510..=860).contains(&self.pointer_x) && (820..=875).contains(&self.pointer_y)
                } else {
                    (510..=810).contains(&self.pointer_x) && (830..=885).contains(&self.pointer_y)
                };
            if over_back {
                self.installer_focus = 0;
            }
            if over_primary {
                self.installer_focus = 1;
            }
            // Installer buttons render their depressed frame on press and
            // activate on release. This keeps the interaction visible while
            // preserving the startup selector's firmware-specific press-edge
            // fallback.
            if released && (over_back || over_primary) {
                crate::output_text(if over_back {
                    b"[installer] mouse=back\n"
                } else {
                    b"[installer] mouse=primary\n"
                });
                self.activate_installer_focus();
            }
        } else if matches!(
            self.mode,
            ConsoleMode::Onboarding | ConsoleMode::Authentication | ConsoleMode::Locked
        ) {
            if self.mode == ConsoleMode::Authentication || self.mode == ConsoleMode::Locked {
                if let Some(target) = layout.authentication_target(self.pointer_x, self.pointer_y) {
                    self.system_focus = target;
                    if clicked && target == 1 {
                        let field = layout.authentication_password_geometry();
                        self.command_cursor = self.clicked_caret_index(
                            field,
                            field.height as usize,
                            self.command_length,
                        );
                    }
                    if clicked && target != 1 {
                        self.input_authentication(ConsoleKey::Enter);
                    }
                }
            } else if let Some(target) =
                layout.onboarding_target(self.system_step, self.pointer_x, self.pointer_y)
            {
                match target {
                    OnboardingTarget::Back => self.system_focus = 0,
                    OnboardingTarget::Primary | OnboardingTarget::Input => self.system_focus = 1,
                    OnboardingTarget::NetworkChoice(index) => self.system_focus = index + 2,
                }
                if clicked && matches!(target, OnboardingTarget::Input) {
                    if let Some(field) = layout.onboarding_input_geometry(self.system_step) {
                        self.command_cursor = self.clicked_caret_index(
                            field,
                            16 * layout.scale(),
                            self.command_length,
                        );
                    }
                }
                if clicked && !matches!(target, OnboardingTarget::Input) {
                    self.input_onboarding(ConsoleKey::Enter);
                }
            }
        } else if self.mode == ConsoleMode::Desktop {
            let chat_state = crate::runtime::ai::with_ai_runtime(|runtime| {
                (runtime.chat.enabled(), runtime.chat.minimized())
            });
            if clicked && chat_state.0 {
                if let Some(target) =
                    layout.ai_chat_target(self.pointer_x, self.pointer_y, chat_state.1)
                {
                    match target {
                        AiChatTarget::Model => {
                            self.ai_chat_focus = 1;
                            self.select_next_chat_model();
                        }
                        AiChatTarget::Composer => {
                            self.ai_chat_focus = 2;
                            let field = layout.ai_chat_geometry(chat_state.1).composer;
                            let length = crate::runtime::ai::with_ai_runtime(|runtime| {
                                runtime.chat.input().len()
                            });
                            let cursor =
                                self.clicked_caret_index(field, 12 * layout.scale(), length);
                            crate::runtime::ai::with_ai_runtime(|runtime| {
                                runtime.chat.set_input_cursor(cursor)
                            });
                        }
                        AiChatTarget::Send => {
                            self.ai_chat_focus = 3;
                            self.submit_ai_chat_input();
                        }
                        AiChatTarget::Minimize => {
                            self.ai_chat_focus = 4;
                            crate::runtime::ai::with_ai_runtime(|runtime| {
                                runtime.chat.set_minimized(!runtime.chat.minimized())
                            });
                        }
                        AiChatTarget::Close => self.set_ai_chat_enabled(false),
                        AiChatTarget::Timeline => self.ai_chat_focus = 0,
                    }
                    self.redraw();
                    return;
                }
            }
            if clicked {
                self.ai_chat_focus = 0;
            }
            if self.editor_dialog != EditorDialog::None {
                if clicked {
                    let count = self.editor_document_count();
                    if let Some(target) = layout.desktop_editor_dialog_target(
                        self.pointer_x,
                        self.pointer_y,
                        self.app_window_x,
                        self.app_window_y,
                        self.app_window_width,
                        self.app_window_height,
                        self.app_window_maximized,
                        self.editor_dialog == EditorDialog::Open,
                        count,
                    ) {
                        match target {
                            EditorDialogTarget::Row(index) => self.system_focus = index,
                            EditorDialogTarget::Cancel => self.close_editor_dialog(),
                            EditorDialogTarget::Accept => {
                                if self.editor_dialog == EditorDialog::Open {
                                    self.open_selected_editor_document();
                                } else {
                                    self.save_editor_document_as();
                                }
                            }
                            EditorDialogTarget::NameField => {
                                let content = layout
                                    .desktop_app_window_geometry(
                                        self.app_window_x,
                                        self.app_window_y,
                                        self.app_window_width,
                                        self.app_window_height,
                                        self.app_window_maximized,
                                    )
                                    .content;
                                let scale = layout.scale();
                                let sheet_width = (420 * scale)
                                    .min((content.width as usize).saturating_sub(40 * scale));
                                let sheet_height = 220 * scale;
                                let left = content.x.max(0) as usize
                                    + (content.width as usize).saturating_sub(sheet_width) / 2;
                                let top = content.y.max(0) as usize
                                    + (content.height as usize).saturating_sub(sheet_height) / 2;
                                let field = crate::ui::geometry::Rect {
                                    x: (left + 24 * scale) as i32,
                                    y: (top + 72 * scale) as i32,
                                    width: sheet_width.saturating_sub(48 * scale) as u32,
                                    height: (46 * scale) as u32,
                                };
                                self.command_cursor = self.clicked_caret_index(
                                    field,
                                    16 * scale,
                                    self.command_length,
                                );
                            }
                        }
                    }
                }
                self.redraw();
                return;
            } else if self.editor_scroll_dragging {
                let geometry = self.editor_scroll_geometry();
                if left_button {
                    self.editor_scroll_row = layout.desktop_editor_scroll_offset_for_thumb(
                        self.pointer_y,
                        geometry,
                        self.editor_scroll_grab_offset,
                    );
                }
                if released {
                    self.editor_scroll_dragging = false;
                }
                self.present_continuous_motion(released);
                return;
            } else if let Some(corner) = self.app_window_resizing {
                if left_button {
                    let (minimum_width, minimum_height) =
                        if self.desktop_app == DesktopAppKind::TaskManager {
                            (620, 500)
                        } else {
                            (420, 360)
                        };
                    let resized = crate::ui::system_layout::resize_native_window(
                        self.app_window_x,
                        self.app_window_y,
                        self.app_window_width,
                        self.app_window_height,
                        corner,
                        self.pointer_x,
                        self.pointer_y,
                        minimum_width,
                        minimum_height,
                    );
                    self.app_window_x = resized.0;
                    self.app_window_y = resized.1;
                    self.app_window_width = resized.2;
                    self.app_window_height = resized.3;
                }
                if released {
                    self.app_window_resizing = None;
                    let _ = self.checkpoint_desktop_layout();
                }
                self.present_continuous_motion(released);
                return;
            } else if self.app_window_dragging {
                if left_button {
                    self.app_window_x = (self.pointer_x - self.app_window_drag_offset_x)
                        .clamp(0, 1000i32.saturating_sub(self.app_window_width));
                    self.app_window_y = (self.pointer_y - self.app_window_drag_offset_y)
                        .clamp(50, 900i32.saturating_sub(self.app_window_height));
                }
                if released {
                    self.app_window_dragging = false;
                    let _ = self.checkpoint_desktop_layout();
                }
                self.present_continuous_motion(released);
                return;
            } else if self.desktop_app != DesktopAppKind::None {
                if clicked {
                    if self.activate_native_app_performance_pointer() {
                        self.redraw();
                        return;
                    }
                    if self.desktop_app == DesktopAppKind::TaskManager
                        && self.activate_task_manager_pointer()
                    {
                        self.redraw();
                        return;
                    }
                    if self.desktop_app == DesktopAppKind::TextEditor
                        && self.editor_dialog == EditorDialog::None
                    {
                        let scroll_geometry = self.editor_scroll_geometry();
                        if let Some(scroll_target) = layout.desktop_editor_scroll_target(
                            self.pointer_x,
                            self.pointer_y,
                            scroll_geometry,
                        ) {
                            match scroll_target {
                                EditorScrollTarget::Page(down) => {
                                    self.scroll_editor(if down { 6 } else { -6 });
                                }
                                EditorScrollTarget::Thumb => {
                                    let pointer_y = self.system.framebuffer_height as i32
                                        * self.pointer_y
                                        / 1000;
                                    self.editor_scroll_grab_offset =
                                        pointer_y.saturating_sub(scroll_geometry.thumb.y);
                                    self.editor_scroll_dragging = true;
                                }
                            }
                            self.redraw();
                            return;
                        }
                    }
                    let app_target = layout.desktop_app_window_target(
                        self.pointer_x,
                        self.pointer_y,
                        self.app_window_x,
                        self.app_window_y,
                        self.app_window_width,
                        self.app_window_height,
                        self.app_window_maximized,
                        self.desktop_app == DesktopAppKind::TextEditor,
                    );
                    match app_target {
                        DesktopAppWindowTarget::Resize(corner) if !self.app_window_maximized => {
                            self.app_window_resizing = Some(corner);
                        }
                        DesktopAppWindowTarget::Title if !self.app_window_maximized => {
                            self.app_window_dragging = true;
                            self.app_window_drag_offset_x = self.pointer_x - self.app_window_x;
                            self.app_window_drag_offset_y = self.pointer_y - self.app_window_y;
                        }
                        DesktopAppWindowTarget::Minimize => {
                            self.minimize_desktop_app();
                        }
                        DesktopAppWindowTarget::Close => {
                            self.close_desktop_app();
                        }
                        DesktopAppWindowTarget::Maximize => {
                            if self.app_window_maximized {
                                self.app_window_x = self.app_window_restore_x;
                                self.app_window_y = self.app_window_restore_y;
                                self.app_window_width = self.app_window_restore_width;
                                self.app_window_height = self.app_window_restore_height;
                            } else {
                                self.app_window_restore_x = self.app_window_x;
                                self.app_window_restore_y = self.app_window_y;
                                self.app_window_restore_width = self.app_window_width;
                                self.app_window_restore_height = self.app_window_height;
                            }
                            self.app_window_maximized = !self.app_window_maximized;
                            let _ = self.checkpoint_desktop_layout();
                        }
                        DesktopAppWindowTarget::NewDocument => {
                            self.editor_document.clear();
                            self.editor_document_path_length = 0;
                            self.editor_document_name_length = 0;
                            self.editor_scroll_row = 0;
                        }
                        DesktopAppWindowTarget::OpenDocument => self.open_editor_document(),
                        DesktopAppWindowTarget::SaveDocument => {
                            self.save_editor_document();
                        }
                        DesktopAppWindowTarget::SaveAsDocument => {
                            self.open_editor_save_as_dialog();
                        }
                        DesktopAppWindowTarget::DeleteDocument => self.delete_editor_document(),
                        DesktopAppWindowTarget::None => {
                            if let Some(app) = self.inactive_app_at_pointer(layout) {
                                self.focus_desktop_app(app);
                            } else if let Some(index) = self.inactive_file_navigator_at_pointer() {
                                let _ = self.load_file_navigator_window(index);
                            } else {
                                match self.desktop_target(layout) {
                                    Some(DesktopTarget::InfinityMenu) => self.open_shell_menu(0),
                                    Some(DesktopTarget::TopMenu(menu)) => {
                                        self.open_shell_menu(menu)
                                    }
                                    Some(DesktopTarget::Status(item)) => {
                                        self.activate_status_item(item)
                                    }
                                    Some(DesktopTarget::Dock(0)) => self.open_app_launcher(),
                                    _ => {}
                                }
                            }
                        }
                        DesktopAppWindowTarget::Content => {
                            if self.desktop_app == DesktopAppKind::TextEditor {
                                let geometry = layout.desktop_app_window_geometry(
                                    self.app_window_x,
                                    self.app_window_y,
                                    self.app_window_width,
                                    self.app_window_height,
                                    self.app_window_maximized,
                                );
                                let scale = layout.scale().max(1);
                                let pointer_x =
                                    self.system.framebuffer_width as i32 * self.pointer_x / 1000;
                                let pointer_y =
                                    self.system.framebuffer_height as i32 * self.pointer_y / 1000;
                                let row = self.editor_scroll_row
                                    + pointer_y
                                        .saturating_sub(geometry.content.y + (18 * scale) as i32)
                                        as usize
                                        / (24 * scale);
                                let columns = (geometry.content.width as usize)
                                    .saturating_sub(52 * scale)
                                    / (9 * scale);
                                let start = crate::ui::text_editor::visual_line_start(
                                    self.editor_document.bytes(),
                                    columns.max(1),
                                    row,
                                );
                                let end = crate::ui::text_editor::visual_line_start(
                                    self.editor_document.bytes(),
                                    columns.max(1),
                                    row + 1,
                                );
                                let column = crate::ui::text_input::caret_from_x(
                                    pointer_x - geometry.content.x - (20 * scale) as i32,
                                    9 * scale,
                                    end.saturating_sub(start),
                                );
                                self.editor_document.set_cursor((start + column).min(end));
                            } else if self.desktop_app == DesktopAppKind::CommandWindow {
                                let geometry = layout.desktop_app_window_geometry(
                                    self.app_window_x,
                                    self.app_window_y,
                                    self.app_window_width,
                                    self.app_window_height,
                                    self.app_window_maximized,
                                );
                                let scale = layout.scale();
                                let field = crate::ui::geometry::Rect {
                                    x: geometry.content.x + (74 * scale) as i32,
                                    y: geometry.content.bottom() - (50 * scale) as i32,
                                    width: geometry
                                        .content
                                        .width
                                        .saturating_sub((92 * scale) as u32),
                                    height: (42 * scale) as u32,
                                };
                                self.command_cursor =
                                    self.clicked_caret_index(field, 0, self.command_length);
                            }
                        }
                        DesktopAppWindowTarget::Title | DesktopAppWindowTarget::Resize(_) => {}
                    }
                    if app_target != DesktopAppWindowTarget::None {
                        self.redraw();
                        return;
                    }
                }
            } else if clicked {
                if let Some(app) = self.inactive_app_at_pointer(layout) {
                    self.focus_desktop_app(app);
                    self.redraw();
                    return;
                }
                if let Some(index) = self.inactive_file_navigator_at_pointer() {
                    let _ = self.load_file_navigator_window(index);
                    self.redraw();
                    return;
                }
            }
            let navigator_context = crate::runtime::with_runtime(|runtime| runtime.file_navigator)
                .flatten()
                .filter(|state| state.context_menu_open);
            if clicked {
                if let Some(context) = navigator_context {
                    let row = layout.file_navigator_context_action(
                        context.context_x,
                        context.context_y,
                        self.pointer_x,
                        self.pointer_y,
                    );
                    if let Some(action) = row {
                        self.activate_file_navigator_context(action, context);
                    } else {
                        let _ = crate::runtime::with_runtime(|runtime| {
                            runtime
                                .file_navigator
                                .as_mut()
                                .map(|navigator| navigator.context_menu_open = false)
                        });
                    }
                    self.redraw();
                    return;
                }
            }
            if right_clicked && self.home_window_visible {
                let target = self.desktop_target(layout);
                if matches!(
                    target,
                    Some(DesktopTarget::HomeItem(_)) | Some(DesktopTarget::HomeContent)
                ) {
                    let item = match target {
                        Some(DesktopTarget::HomeItem(index)) => Some(index),
                        _ => None,
                    };
                    let _ = crate::runtime::with_runtime(|runtime| {
                        runtime.file_navigator.as_mut().map(|navigator| {
                            navigator.open_context_menu(
                                self.pointer_x.min(800),
                                self.pointer_y.min(780),
                                item,
                            )
                        })
                    });
                    self.redraw();
                    return;
                }
            }
            if let Some(corner) = self.home_window_resizing {
                if left_button {
                    let resized = crate::ui::system_layout::resize_home_window(
                        self.home_window_x,
                        self.home_window_y,
                        self.home_window_width,
                        self.home_window_height,
                        corner,
                        self.pointer_x,
                        self.pointer_y,
                    );
                    self.home_window_x = resized.0;
                    self.home_window_y = resized.1;
                    self.home_window_width = resized.2;
                    self.home_window_height = resized.3;
                }
                if released {
                    self.home_window_resizing = None;
                    self.checkpoint_active_file_navigator();
                    let _ = self.checkpoint_desktop_layout();
                }
                self.present_continuous_motion(released);
                return;
            } else if let Some(item) = self.home_dragging_item {
                if left_button
                    && ((self.pointer_x - self.home_drag_origin_x).abs() > 7
                        || (self.pointer_y - self.home_drag_origin_y).abs() > 7)
                {
                    self.home_drag_moved = true;
                }
                if released {
                    let target = self.desktop_target(layout);
                    if self.home_drag_from_desktop {
                        if self.home_drag_moved {
                            self.desktop_item_positions[item] =
                                [self.pointer_x.clamp(35, 950), self.pointer_y.clamp(90, 880)];
                        }
                    } else if !self.home_drag_moved {
                        self.open_file_navigator_selection();
                    } else if self.home_drag_moved {
                        match target {
                            Some(DesktopTarget::HomeSidebar(location)) if item == 6 => {
                                self.home_note_previous_location = self.home_note_location;
                                self.home_note_location = location;
                                self.home_previous_location = self.home_location;
                                self.home_location = location;
                                crate::output_text(b"[objects] notes.txt moved by drag and drop\n");
                            }
                            None if self.pointer_y > 55 && self.pointer_y < 900 => {
                                self.place_desktop_reference(item);
                            }
                            _ => {}
                        }
                    }
                    self.home_dragging_item = None;
                    self.home_drag_from_desktop = false;
                    self.home_drag_moved = false;
                    let _ = self.checkpoint_desktop_layout();
                }
                self.present_continuous_motion(released);
                return;
            } else if self.home_window_dragging {
                if left_button {
                    self.home_window_x = (self.pointer_x - self.home_window_drag_offset_x)
                        .clamp(0, 1000i32.saturating_sub(self.home_window_width));
                    self.home_window_y = (self.pointer_y - self.home_window_drag_offset_y)
                        .clamp(50, 920i32.saturating_sub(self.home_window_height));
                }
                if released {
                    self.home_window_dragging = false;
                    self.checkpoint_active_file_navigator();
                    let _ = self.checkpoint_desktop_layout();
                }
                self.present_continuous_motion(released);
                return;
            } else if clicked {
                let target = self.desktop_target(layout);
                if !matches!(
                    target,
                    Some(DesktopTarget::HomeMenu(_))
                        | Some(DesktopTarget::HomeMenuItem(_))
                        | Some(DesktopTarget::HomeDialogAction(_))
                ) {
                    let _ = crate::runtime::with_runtime(|runtime| {
                        runtime.file_navigator.as_mut().map(|navigator| {
                            if navigator.dialog_open.is_none() {
                                navigator.menu_open = None;
                            }
                        })
                    });
                }
                match target {
                    Some(DesktopTarget::HomeResize(corner)) => {
                        self.home_window_resizing = Some(corner);
                    }
                    Some(DesktopTarget::HomeTitle) => {
                        if !self.home_window_maximized {
                            self.home_window_dragging = true;
                            self.home_window_drag_offset_x = self.pointer_x - self.home_window_x;
                            self.home_window_drag_offset_y = self.pointer_y - self.home_window_y;
                        }
                    }
                    Some(DesktopTarget::HomeControl(0)) => {
                        self.checkpoint_active_file_navigator();
                        let next = crate::runtime::with_runtime(|runtime| {
                            runtime.file_navigators.minimize_active()
                        })
                        .flatten();
                        if let Some(next) = next {
                            let index = crate::runtime::with_runtime(|runtime| {
                                runtime.file_navigators.active_index()
                            })
                            .flatten();
                            if let Some(index) = index {
                                let _ = self.load_file_navigator_window(index);
                            }
                            self.home_window_visible = next.visible;
                        } else {
                            self.home_window_visible = false;
                        }
                        let _ = self.checkpoint_desktop_layout();
                    }
                    Some(DesktopTarget::HomeControl(2)) => {
                        self.checkpoint_active_file_navigator();
                        let next = crate::runtime::with_runtime(|runtime| {
                            runtime.file_navigators.close_active()
                        })
                        .flatten();
                        if let Some(next) = next {
                            let index = crate::runtime::with_runtime(|runtime| {
                                runtime.file_navigators.active_index()
                            })
                            .flatten();
                            if let Some(index) = index {
                                let _ = self.load_file_navigator_window(index);
                            }
                            self.home_window_visible = next.visible;
                        } else {
                            self.home_window_visible = false;
                            self.home_selected_item = None;
                        }
                        let _ = self.checkpoint_desktop_layout();
                    }
                    Some(DesktopTarget::HomeControl(1)) => {
                        if self.home_window_maximized {
                            self.home_window_x = self.home_window_restore_x;
                            self.home_window_y = self.home_window_restore_y;
                            self.home_window_width = self.home_window_restore_width;
                            self.home_window_height = self.home_window_restore_height;
                        } else {
                            self.home_window_restore_x = self.home_window_x;
                            self.home_window_restore_y = self.home_window_y;
                            self.home_window_restore_width = self.home_window_width;
                            self.home_window_restore_height = self.home_window_height;
                        }
                        self.home_window_maximized = !self.home_window_maximized;
                        let _ = self.checkpoint_desktop_layout();
                    }
                    Some(DesktopTarget::HomeMenu(menu)) => {
                        if let Some(menu) =
                            crate::runtime::object_navigation::FileNavigatorMenu::from_index(menu)
                        {
                            let _ = crate::runtime::with_runtime(|runtime| {
                                runtime
                                    .file_navigator
                                    .as_mut()
                                    .map(|navigator| navigator.open_menu(menu))
                            });
                        }
                    }
                    Some(DesktopTarget::HomeMenuItem(item)) => {
                        let menu = crate::runtime::with_runtime(|runtime| {
                            runtime
                                .file_navigator
                                .and_then(|navigator| navigator.menu_open)
                        })
                        .flatten();
                        if let Some(menu) = menu {
                            self.activate_file_navigator_menu(menu, item);
                        }
                    }
                    Some(DesktopTarget::HomeDialogAction(action)) => {
                        self.activate_file_navigator_dialog(action);
                    }
                    Some(DesktopTarget::HomeToolbar(action)) => {
                        let _ = crate::runtime::with_runtime(|runtime| {
                            let Some(navigator) = runtime.file_navigator.as_mut() else {
                                return;
                            };
                            match action {
                                0 => {
                                    let _ = navigator.back();
                                }
                                1 => {
                                    let _ = navigator.forward();
                                }
                                2 => {
                                    if let Ok(parent) =
                                        crate::runtime::object_navigation::parent_path(
                                            navigator.active_namespace_ref.as_bytes(),
                                        )
                                    {
                                        let _ = navigator.navigate(parent.as_bytes());
                                    }
                                }
                                3 => {
                                    navigator.view_mode =
                                        crate::runtime::object_navigation::ViewMode::List
                                }
                                4 => {
                                    navigator.view_mode =
                                        crate::runtime::object_navigation::ViewMode::Grid
                                }
                                _ => {}
                            }
                        });
                        self.home_selected_item = None;
                    }
                    Some(DesktopTarget::HomeLocation) => {
                        let (left, top, width, _) = layout.home_window_geometry_sized(
                            self.home_window_x,
                            self.home_window_y,
                            self.home_window_width,
                            self.home_window_height,
                            self.home_window_maximized,
                        );
                        let scale = layout.scale();
                        let field = crate::ui::geometry::Rect {
                            x: (left + 100 * scale) as i32,
                            y: (top + 34 * scale + 5 * scale) as i32,
                            width: width.saturating_sub(250 * scale) as u32,
                            height: (28 * scale) as u32,
                        };
                        let length = crate::runtime::with_runtime(|runtime| {
                            runtime
                                .file_navigator
                                .map(|navigator| navigator.active_namespace_ref.as_bytes().len())
                        })
                        .flatten()
                        .unwrap_or(0);
                        let cursor = self.clicked_caret_index(field, 14 * scale, length);
                        let _ = crate::runtime::with_runtime(|runtime| {
                            runtime.file_navigator.as_mut().map(|navigator| {
                                navigator.begin_location_edit();
                                navigator.editor_cursor = cursor;
                            })
                        });
                    }
                    Some(DesktopTarget::HomeSidebar(location)) => {
                        self.home_previous_location = self.home_location;
                        self.home_location = location;
                        let path = home_location_path(location);
                        let _ = crate::runtime::with_runtime(|runtime| {
                            runtime
                                .file_navigator
                                .as_mut()
                                .map(|navigator| navigator.navigate(path))
                        });
                        self.home_selected_item = None;
                    }
                    Some(DesktopTarget::HomeItem(item)) => {
                        self.home_selected_item = Some(item);
                        let _ = crate::runtime::with_runtime(|runtime| {
                            runtime.file_navigator.as_mut().map(|navigator| {
                                navigator.selected_index = item as u16;
                                navigator.context_menu_open = false;
                            })
                        });
                        self.home_dragging_item = Some(item);
                        self.home_drag_from_desktop = false;
                        self.home_drag_origin_x = self.pointer_x;
                        self.home_drag_origin_y = self.pointer_y;
                        self.home_drag_moved = false;
                    }
                    Some(DesktopTarget::InfinityMenu) => {
                        self.open_shell_menu(0);
                    }
                    Some(DesktopTarget::TopMenu(menu)) => self.open_shell_menu(menu),
                    Some(DesktopTarget::Status(item)) => self.activate_status_item(item),
                    Some(DesktopTarget::Dock(index)) => {
                        match DESKTOP_DOCK_ENTRIES.get(index).map(|entry| entry.action) {
                            Some(DockAction::Launcher) => self.open_app_launcher(),
                            Some(DockAction::Files) => {
                                let restored = crate::runtime::with_runtime(|runtime| {
                                    runtime.file_navigators.restore_minimized()
                                })
                                .flatten();
                                if let Some(index) = restored {
                                    let _ = self.load_file_navigator_window(index);
                                } else {
                                    let _ = self.open_file_navigator_window(b"/home/default");
                                }
                            }
                            Some(DockAction::Settings) => self.open_settings(0),
                            Some(DockAction::About) => self.open_settings(8),
                            Some(DockAction::AiVoice) => self.open_settings(3),
                            Some(DockAction::Appearance) => self.open_settings(1),
                            Some(DockAction::Network) => self.open_settings(6),
                            Some(DockAction::Trash) => {
                                self.home_window_visible = true;
                                self.home_location = 8;
                                crate::output_text(b"[objects] recycle collection opened\n")
                            }
                            None => {}
                        }
                    }
                    Some(DesktopTarget::HomeContent) => {
                        self.home_selected_item = None;
                        let _ = crate::runtime::with_runtime(|runtime| {
                            runtime.file_navigator.as_mut().map(|navigator| {
                                navigator.selected_index =
                                    crate::runtime::object_navigation::FILE_NAVIGATOR_NO_SELECTION;
                                navigator.context_menu_open = false;
                            })
                        });
                    }
                    None => {
                        if let Some(item) = self.desktop_item_at_pointer() {
                            self.home_dragging_item = Some(item);
                            self.home_drag_from_desktop = true;
                            self.home_drag_origin_x = self.pointer_x;
                            self.home_drag_origin_y = self.pointer_y;
                            self.home_drag_moved = false;
                        }
                    }
                    _ => {}
                }
            }
        } else if self.mode == ConsoleMode::AppLauncher {
            let visible = launcher_visible_count(&self.command[..self.command_length]);
            let geometry = layout.app_launcher_geometry();
            let scroll = layout.app_launcher_scroll_geometry(visible);
            let target = layout.app_launcher_target(self.pointer_x, self.pointer_y, visible);
            let presentation = crate::ui::app_launcher::launcher_presentation();
            if let Some(source) = presentation.drag_source {
                if left_button {
                    let pointer_y = self.system.framebuffer_height as i32 * self.pointer_y / 1000;
                    let edge = (28 * layout.scale()) as i32;
                    if pointer_y < geometry.grid_viewport.y.saturating_add(edge) {
                        let _ = crate::ui::app_launcher::launcher_scroll_by(
                            -(12 * layout.scale() as i32),
                            scroll.maximum_scroll,
                        );
                    } else if pointer_y > geometry.grid_viewport.bottom().saturating_sub(edge) {
                        let _ = crate::ui::app_launcher::launcher_scroll_by(
                            12 * layout.scale() as i32,
                            scroll.maximum_scroll,
                        );
                    }
                    let drag_target = match target {
                        AppLauncherTarget::App(index) => Some(index),
                        _ => None,
                    };
                    let _ = crate::ui::app_launcher::launcher_update_drag(
                        drag_target,
                        self.pointer_x,
                        self.pointer_y,
                    );
                }
                if released {
                    match crate::ui::app_launcher::launcher_finish_drag(
                        &self.command[..self.command_length],
                    ) {
                        crate::ui::app_launcher::LauncherRelease::Activate(index) => {
                            self.system_focus = index + 1;
                            self.activate_launcher_focus();
                        }
                        crate::ui::app_launcher::LauncherRelease::Reordered => {
                            self.system_focus = source.min(visible.saturating_sub(1)) + 1;
                        }
                        crate::ui::app_launcher::LauncherRelease::None => {}
                    }
                }
                self.present_continuous_motion(released);
                return;
            }
            if self.launcher_scroll_dragging {
                if left_button {
                    let pointer_y = self.system.framebuffer_height as i32 * self.pointer_y / 1000;
                    let track_top = geometry.scrollbar_track.y;
                    let travel = geometry
                        .scrollbar_track
                        .height
                        .saturating_sub(scroll.thumb.height)
                        as usize;
                    let thumb_top = pointer_y
                        .saturating_sub(self.launcher_scroll_grab_offset)
                        .saturating_sub(track_top)
                        .clamp(0, travel.min(i32::MAX as usize) as i32)
                        as usize;
                    let offset = if travel == 0 {
                        0
                    } else {
                        thumb_top.saturating_mul(scroll.maximum_scroll) / travel
                    };
                    let _ =
                        crate::ui::app_launcher::launcher_scroll_to(offset, scroll.maximum_scroll);
                }
                if released {
                    self.launcher_scroll_dragging = false;
                }
                self.present_continuous_motion(released);
                return;
            }
            match target {
                AppLauncherTarget::Search => {
                    self.system_focus = 0;
                    if clicked {
                        let field = layout.app_launcher_geometry().search;
                        self.command_cursor = self.clicked_caret_index(
                            field,
                            50 * layout.scale(),
                            self.command_length,
                        );
                    }
                }
                AppLauncherTarget::App(index) => {
                    self.system_focus = index + 1;
                    if clicked {
                        crate::ui::app_launcher::launcher_begin_drag(
                            index,
                            self.pointer_x,
                            self.pointer_y,
                        );
                    }
                }
                AppLauncherTarget::Category(index) => {
                    self.system_focus = visible + index + 1;
                    if clicked {
                        self.activate_launcher_focus();
                    }
                }
                AppLauncherTarget::Close
                | AppLauncherTarget::DockToggle
                | AppLauncherTarget::Dismiss
                    if clicked =>
                {
                    self.close_app_launcher();
                }
                AppLauncherTarget::ScrollbarThumb if clicked => {
                    let pointer_y = self.system.framebuffer_height as i32 * self.pointer_y / 1000;
                    self.launcher_scroll_grab_offset = pointer_y.saturating_sub(scroll.thumb.y);
                    self.launcher_scroll_dragging = true;
                }
                AppLauncherTarget::ScrollbarTrack if clicked => {
                    let pointer_y = self.system.framebuffer_height as i32 * self.pointer_y / 1000;
                    let travel = geometry
                        .scrollbar_track
                        .height
                        .saturating_sub(scroll.thumb.height)
                        as usize;
                    let thumb_top = pointer_y
                        .saturating_sub(geometry.scrollbar_track.y)
                        .saturating_sub(scroll.thumb.height as i32 / 2)
                        .clamp(0, travel.min(i32::MAX as usize) as i32)
                        as usize;
                    let offset = if travel == 0 {
                        0
                    } else {
                        thumb_top.saturating_mul(scroll.maximum_scroll) / travel
                    };
                    let _ =
                        crate::ui::app_launcher::launcher_scroll_to(offset, scroll.maximum_scroll);
                }
                AppLauncherTarget::Panel
                | AppLauncherTarget::ScrollbarThumb
                | AppLauncherTarget::ScrollbarTrack
                | AppLauncherTarget::Close
                | AppLauncherTarget::DockToggle
                | AppLauncherTarget::Dismiss => {}
            }
        } else if self.mode == ConsoleMode::SystemMenu {
            if clicked {
                match self.desktop_target(layout) {
                    Some(DesktopTarget::InfinityMenu) => {
                        self.open_shell_menu(0);
                        self.redraw();
                        return;
                    }
                    Some(DesktopTarget::TopMenu(menu)) => {
                        self.open_shell_menu(menu);
                        self.redraw();
                        return;
                    }
                    Some(DesktopTarget::Status(item)) => {
                        self.activate_status_item(item);
                        self.redraw();
                        return;
                    }
                    _ => {}
                }
            }
            match layout.system_menu_target(self.shell_menu, self.pointer_x, self.pointer_y) {
                SystemMenuTarget::Item(index) => {
                    self.system_focus = index;
                    if clicked {
                        self.input_shell(ConsoleKey::Enter);
                    }
                }
                SystemMenuTarget::Dismiss if clicked => self.enter_desktop(),
                SystemMenuTarget::Dismiss => {}
            }
        } else if self.mode == ConsoleMode::Settings {
            if self.settings_timeout_dragging {
                if left_button {
                    let value = layout.settings_slider_drag_value(
                        self.pointer_x,
                        self.settings_window,
                        3,
                        crate::runtime::identity::MAX_NO_ACTIVITY_TIMEOUT_MINUTES - 1,
                    );
                    self.preview_user_no_activity_timeout(value.saturating_add(1));
                }
                if released {
                    self.settings_timeout_dragging = false;
                    self.commit_user_no_activity_timeout();
                }
                self.present_continuous_motion(released);
                return;
            } else if let Some(row) = self.settings_effect_dragging {
                if left_button {
                    let maximum = if row == 4 { 15 } else { 8 };
                    let value = layout.settings_effect_slider_drag_value(
                        self.pointer_x,
                        self.settings_window,
                        row,
                        maximum,
                    );
                    self.preview_background_effect(
                        row,
                        if row == 4 { 40 + value * 4 } else { value },
                    );
                }
                if released {
                    self.settings_effect_dragging = None;
                    self.commit_background_effects();
                }
                self.present_continuous_motion(released);
                return;
            } else if self.system_focus == 4 && left_button && clicked {
                if let Some(value) = layout.settings_slider_target(
                    self.pointer_x,
                    self.pointer_y,
                    self.settings_window,
                    3,
                    crate::runtime::identity::MAX_NO_ACTIVITY_TIMEOUT_MINUTES - 1,
                ) {
                    self.settings_timeout_dragging = true;
                    self.preview_user_no_activity_timeout(value.saturating_add(1));
                    self.redraw();
                    return;
                }
            } else if self.settings_scroll_dragging {
                if left_button {
                    let offset = layout.settings_scroll_offset_for_thumb_in_section(
                        self.pointer_y,
                        self.settings_window,
                        self.settings_scroll_grab_offset,
                        self.system_focus,
                    );
                    self.settings_window.scroll_offset = offset;
                    self.settings_scroll_target = offset;
                    self.settings_window.scroll_offset = offset;
                }
                if released {
                    self.settings_scroll_dragging = false;
                }
                self.present_continuous_motion(released);
                return;
            } else if let Some(corner) = self.settings_window_resizing {
                if left_button {
                    let resized = crate::ui::system_layout::resize_native_window(
                        self.settings_window.x,
                        self.settings_window.y,
                        self.settings_window.width,
                        self.settings_window.height,
                        corner,
                        self.pointer_x,
                        self.pointer_y,
                        600,
                        420,
                    );
                    self.settings_window.x = resized.0;
                    self.settings_window.y = resized.1;
                    self.settings_window.width = resized.2;
                    self.settings_window.height = resized.3;
                }
                if released {
                    self.settings_window_resizing = None;
                    let _ = self.checkpoint_desktop_layout();
                }
                self.present_continuous_motion(released);
                return;
            } else if self.settings_window_dragging {
                if left_button {
                    self.settings_window.x = (self.pointer_x - self.settings_window_drag_offset_x)
                        .clamp(0, 1000i32.saturating_sub(self.settings_window.width));
                    self.settings_window.y = (self.pointer_y - self.settings_window_drag_offset_y)
                        .clamp(50, 920i32.saturating_sub(self.settings_window.height));
                }
                if released {
                    self.settings_window_dragging = false;
                    let _ = self.checkpoint_desktop_layout();
                }
                self.present_continuous_motion(released);
                return;
            } else if self.system_focus == 1 && left_button {
                if clicked {
                    if let Some(row) = self
                        .settings_window
                        .expanded_row
                        .filter(|row| matches!(row, 4 | 5))
                    {
                        let maximum = if row == 4 { 15 } else { 8 };
                        if let Some(value) = layout.settings_effect_slider_target(
                            self.pointer_x,
                            self.pointer_y,
                            self.settings_window,
                            row,
                            maximum,
                        ) {
                            self.settings_effect_dragging = Some(row);
                            self.preview_background_effect(
                                row,
                                if row == 4 { 40 + value * 4 } else { value },
                            );
                            self.redraw();
                            return;
                        }
                    }
                }
                if let Some(target) = layout.settings_primary_target(
                    self.pointer_x,
                    self.pointer_y,
                    self.settings_window,
                ) {
                    self.adjust_primary(target);
                    self.present_continuous_motion(false);
                    return;
                }
                if let Some(target) = layout.settings_accent_target(
                    self.pointer_x,
                    self.pointer_y,
                    self.settings_window,
                ) {
                    self.adjust_accent(target);
                    self.present_continuous_motion(false);
                    return;
                }
            }
            if released && self.settings_primary_dirty {
                self.commit_primary();
                self.redraw();
                return;
            }
            if released && self.settings_accent_dirty {
                self.commit_accent();
                self.redraw();
                return;
            }
            if clicked && self.system_focus == 1 {
                if let Some(theme) = layout.settings_icon_theme_target(
                    self.pointer_x,
                    self.pointer_y,
                    self.settings_window,
                ) {
                    self.select_icon_theme(theme);
                    self.redraw();
                    return;
                }
            }
            if clicked && self.system_focus == 6 {
                if let Some(target) = layout.network_settings_target(
                    self.pointer_x,
                    self.pointer_y,
                    self.settings_window,
                ) {
                    match target {
                        NetworkSettingsTarget::Page(index) => {
                            self.settings_window.expanded_row = Some(index.min(6));
                            self.settings_window.scroll_offset = 0;
                            self.settings_scroll_target = 0;
                            self.settings_window.control_focus = 0;
                            self.settings_editing = false;
                            self.reset_input();
                        }
                        NetworkSettingsTarget::Control(index) => {
                            self.activate_network_control(index)
                        }
                    }
                    self.redraw();
                    return;
                }
            }
            if clicked && self.system_focus == 7 {
                if let Some(target) = layout.node_settings_target(
                    self.pointer_x,
                    self.pointer_y,
                    self.settings_window,
                ) {
                    match target {
                        NetworkSettingsTarget::Page(index) => {
                            self.settings_window.expanded_row = Some(index.min(4));
                            self.settings_window.scroll_offset = 0;
                            self.settings_scroll_target = 0;
                            self.settings_window.control_focus = 0;
                        }
                        NetworkSettingsTarget::Control(index) => self.activate_node_control(index),
                    }
                    self.redraw();
                    return;
                }
            }
            if let Some(target) = layout.settings_target_for_section(
                self.pointer_x,
                self.pointer_y,
                self.settings_window,
                self.system_focus,
            ) {
                match target {
                    SettingsTarget::Section(index) if clicked => {
                        self.system_focus = index;
                        self.settings_window.row_count = if matches!(index, 1 | 6 | 7 | 10) {
                            8
                        } else if index == 3 {
                            7
                        } else {
                            5
                        };
                        self.settings_editing = false;
                        self.settings_window.expanded_row = matches!(index, 6 | 7).then_some(0);
                        self.settings_window.scroll_offset = 0;
                        self.settings_scroll_target = 0;
                        self.settings_window.control_focus = 0;
                        self.reset_input();
                    }
                    SettingsTarget::ContentRow(row)
                        if clicked && !matches!(self.system_focus, 6 | 7) =>
                    {
                        self.toggle_settings_row(row)
                    }
                    SettingsTarget::ExpandedAction if clicked => {
                        if let Some(row) = self.settings_window.expanded_row {
                            self.activate_settings_content_row(row);
                        }
                    }
                    SettingsTarget::ScrollPage(down) if clicked => {
                        self.scroll_settings(if down { 4 } else { -4 })
                    }
                    SettingsTarget::ScrollThumb if clicked => {
                        let geometry = layout.settings_window_geometry_for_section(
                            self.settings_window,
                            self.system_focus,
                        );
                        let pointer_y =
                            self.system.framebuffer_height as i32 * self.pointer_y / 1000;
                        self.settings_scroll_grab_offset =
                            pointer_y.saturating_sub(geometry.scrollbar_thumb.y);
                        self.settings_scroll_dragging = true;
                    }
                    SettingsTarget::Title if clicked && !self.settings_window.maximized => {
                        self.settings_window_dragging = true;
                        self.settings_window_drag_offset_x =
                            self.pointer_x - self.settings_window.x;
                        self.settings_window_drag_offset_y =
                            self.pointer_y - self.settings_window.y;
                    }
                    SettingsTarget::Resize(corner)
                        if clicked && !self.settings_window.maximized =>
                    {
                        self.settings_window_resizing = Some(corner)
                    }
                    SettingsTarget::WindowControl(0 | 2) if clicked => {
                        self.enter_desktop();
                        let _ = self.checkpoint_desktop_layout();
                    }
                    SettingsTarget::WindowControl(1) if clicked => {
                        self.settings_window.maximized = !self.settings_window.maximized;
                        let _ = self.checkpoint_desktop_layout();
                    }
                    SettingsTarget::Section(_)
                    | SettingsTarget::ContentRow(_)
                    | SettingsTarget::ExpandedAction
                    | SettingsTarget::ScrollPage(_)
                    | SettingsTarget::ScrollThumb
                    | SettingsTarget::Title
                    | SettingsTarget::Resize(_)
                    | SettingsTarget::WindowControl(_) => {}
                }
            }
        }
        if buttons == 0
            && !released
            && !clicked
            && !right_clicked
            && matches!(self.mode, ConsoleMode::Desktop | ConsoleMode::Settings)
        {
            self.publish_text_input_presentation();
            crate::bootstrap::system_ui_cursor(self.pointer_x, self.pointer_y);
        } else {
            self.redraw();
        }
    }

    // ------------------------=
    // FUNC: pointer_absolute
    // DESC: Handles pointer absolute input or state transitions.
    // ------------------=
    fn pointer_absolute(&mut self, x: i32, y: i32, buttons: u8) {
        if matches!(self.mode, ConsoleMode::Console | ConsoleMode::Repair) {
            return;
        }
        let next_x = x.clamp(0, 1000);
        let next_y = y.clamp(0, 1000);
        let left_button = buttons & crate::drivers::input::pointer::BUTTON_LEFT != 0;
        if !crate::drivers::input::pointer::absolute_pointer_state_changed(
            self.pointer_x,
            self.pointer_y,
            self.pointer_pressed,
            next_x,
            next_y,
            left_button,
        ) {
            return;
        }
        self.session_idle.note_activity();
        crate::bootstrap::note_pointer_activity();
        self.pointer_x = next_x;
        self.pointer_y = next_y;
        self.pointer_interaction(buttons);
    }

    // ------------------------=
    // FUNC: input_console
    // DESC: Handles input console input or state transitions.
    // ------------------=
    fn input_console(&mut self, key: ConsoleKey) {
        if self.edit_input(key) {
            return;
        }
        match key {
            ConsoleKey::Escape => {
                if self.current_session.is_zero() {
                    self.show_startup();
                } else {
                    self.enter_desktop();
                }
            }
            ConsoleKey::Enter => self.execute_input(),
            _ => {}
        }
    }

    // ------------------------=
    // FUNC: execute_input
    // DESC: Implements the execute input operation.
    // ------------------=
    fn execute_input(&mut self) {
        if self.command_length == 0 {
            return;
        }
        let command = self.command;
        let length = self.command_length;
        self.output
            .write_segments(&[self.prompt(), &command[..length]]);
        let resolved_alias = crate::runtime::with_runtime(|runtime| {
            runtime
                .shell_profiles
                .as_ref()
                .and_then(|profiles| profiles.resolve(&command[..length]).ok())
        })
        .flatten();
        if let Some(resolution) = resolved_alias {
            if !resolution.native {
                self.output.write_segments(&[
                    b"Resolved profile ",
                    &profile_id_text(resolution.resolved_profile),
                    b" -> ",
                    resolution.canonical.as_bytes(),
                ]);
            }
            if self.execute_native_navigation_command(resolution.canonical.as_bytes()) {
                self.reset_input();
                return;
            }
        }
        if self.execute_language_command(&command[..length]) {
            self.reset_input();
            return;
        }
        if self.execute_runtime_command(&command[..length]) {
            self.reset_input();
            return;
        }
        if self.execute_ai_command(&command[..length]) {
            self.reset_input();
            return;
        }
        if self.execute_storage_command(&command[..length]) {
            self.reset_input();
            return;
        }
        let input = core::str::from_utf8(&command[..length]).unwrap_or("");
        let context = self.context();
        match self.intent.resolve(input, &context) {
            Ok(resolved) => {
                if resolved.source == ResolutionSource::BuiltInIntent {
                    self.output
                        .write_segments(&[b"Interpreted as: ", resolved.canonical_command]);
                }
                let policy = KnownOperationPolicy;
                if policy.authorize(resolved.operation, &context) {
                    self.dispatch(resolved.operation);
                } else {
                    self.output
                        .write_line(b"That operation is not permitted in this context.");
                }
            }
            Err(_) => {
                let correlation = command_correlation(&command[..length]);
                let plan = crate::runtime::with_runtime(|runtime| {
                    runtime.resolve_console_intent(&command[..length], 0, correlation)
                })
                .and_then(Result::ok);
                if let Some(plan) = plan {
                    if plan.ambiguous {
                        self.output
                            .write_line(b"Local AI needs clarification; no operation executed.");
                    } else if let Some(planned) = plan.operations[0] {
                        self.output
                            .write_line(b"Interpreted locally by local-intent-v1.");
                        self.output
                            .write_number(b"Confidence: ", plan.confidence_milli as u64);
                        if let Some(operation) = system_operation_from_iop(planned.operation) {
                            self.dispatch(operation);
                        } else {
                            self.output
                                .write_line(b"The proposed typed operation is unavailable.");
                        }
                    }
                } else {
                    self.output
                        .write_line(b"I couldn't determine what operation you intended.");
                    self.output
                        .write_line(b"Try help, or describe what you want another way.");
                }
            }
        }
        self.reset_input();
    }

    // ------------------------=
    // FUNC: execute_native_navigation_command
    // DESC: Dispatches Milestone 8 vocabulary through typed Namespace, Object, Trash, Shell, and Application operations.
    // ------------------=
    fn execute_native_navigation_command(&mut self, command: &[u8]) -> bool {
        let first = command_word(command, 0).unwrap_or(&[]);
        if first == b"path" && command_word(command, 1).is_none() {
            self.output.write_line(self.navigation_context.path());
            return true;
        }
        if matches!(first, b"idir" | b"cd") {
            let target = command_tail(command, 1).unwrap_or(b"home");
            let result = self.navigation_context.navigate(target, |path| {
                crate::storage::namespace_resolve(path).is_ok()
            });
            match result {
                Ok(path) => self.output.write_line(path),
                Err(_) => self
                    .output
                    .write_line(b"Namespace navigation denied or unavailable."),
            }
            return true;
        }
        if first == b"list" {
            let mut prefix = self.navigation_context.path();
            let mut tree = false;
            if command_word(command, 1) == Some(b"tree") {
                tree = true;
                if let Some(path) = command_word(command, 2) {
                    prefix = path;
                }
            } else if let Some(path) = command_word(command, 1) {
                prefix = path;
            }
            let mut shown = 0usize;
            if !tree {
                self.output.write_line(b".");
                self.output.write_line(b"..");
                shown = crate::runtime::object_navigation::FILE_NAVIGATOR_NAVIGATION_ENTRY_COUNT;
            }
            for index in 0..32usize {
                let entry = match crate::storage::namespace_list_nth(prefix, index) {
                    Ok(Some(entry)) => entry,
                    _ => break,
                };
                let path = &entry.path[..entry.path_len as usize];
                if tree || immediate_namespace_child(prefix, path) {
                    self.output.write_line(path);
                    shown += 1;
                }
            }
            self.output.write_number(b"Objects shown: ", shown as u64);
            return true;
        }
        if matches!(
            first,
            b"examine" | b"resolve" | b"versions" | b"references" | b"relationships"
        ) {
            let Some(target) = command_word(command, 1) else {
                self.output
                    .write_line(b"A NamespaceRef or ObjectId is required.");
                return true;
            };
            match crate::storage::namespace_resolve(target) {
                Ok(id) => {
                    self.output.write_id(b"ObjectId: ", id);
                    if first == b"resolve" {
                        self.output.write_line(target);
                    } else if first == b"versions" {
                        if let Ok((_, count, current)) = crate::storage::object_history(target) {
                            self.output.write_number(b"Versions: ", count as u64);
                            self.output
                                .write_number(b"Current version: ", current as u64);
                        }
                    } else if first == b"references" {
                        for index in 0..32usize {
                            let mut path = [0u8; 96];
                            let Some(length) =
                                crate::storage::object_reference_nth(id, index, &mut path)
                            else {
                                break;
                            };
                            self.output.write_line(&path[..length]);
                        }
                    } else if first == b"relationships" {
                        self.output.write_line(
                            b"Typed semantic relationships available through Object.Relationships.",
                        );
                    } else if let Ok((metadata, references)) =
                        crate::storage::object_inspect_path(target)
                    {
                        self.output
                            .write_number(b"Object type: ", metadata.kind as u64);
                        self.output
                            .write_number(b"Reference count: ", references as u64);
                        self.output
                            .write_number(b"Version: ", metadata.current_version as u64);
                        self.output
                            .write_number(b"Logical bytes: ", metadata.logical_size as u64);
                    }
                }
                Err(error) => self.storage_error(error),
            }
            return true;
        }
        if first == b"find" {
            let query = command_tail(command, 1).unwrap_or(&[]);
            let mut found = 0usize;
            for index in 0..128usize {
                let mut path = [0u8; 96];
                let Some((length, _)) = crate::storage::namespace_entry(index, &mut path) else {
                    break;
                };
                if ascii_contains_case_insensitive(&path[..length], query) {
                    self.output.write_line(&path[..length]);
                    found += 1;
                    if found == 32 {
                        break;
                    }
                }
            }
            self.output.write_number(b"Search results: ", found as u64);
            return true;
        }
        if first == b"open" {
            let Some(target) = command_word(command, 1) else {
                return true;
            };
            if crate::storage::object_inspect_path(target).is_ok() {
                self.output
                    .write_line(b"ApplicationAssociation.Resolve -> Application.Launch");
                self.open_text_editor();
            } else {
                self.output
                    .write_line(b"No authorized application association.");
            }
            return true;
        }
        if first == b"navigator" {
            let target = command_word(command, 1).unwrap_or(self.navigation_context.path());
            let target = if target == b"." {
                self.navigation_context.path()
            } else {
                target
            };
            let navigated = crate::runtime::with_runtime(|runtime| {
                runtime
                    .file_navigator
                    .as_mut()
                    .map(|navigator| navigator.navigate(target).is_ok())
                    .unwrap_or(false)
            })
            .unwrap_or(false);
            if navigated {
                self.home_window_visible = true;
                self.enter_desktop();
            } else {
                self.output
                    .write_line(b"Navigator target is not a valid NamespaceRef.");
            }
            return true;
        }
        if first == b"namespace" {
            return self.execute_namespace_resource_command(command);
        }
        if first == b"object" {
            return self.execute_object_resource_command(command);
        }
        if first == b"reference" {
            return self.execute_reference_resource_command(command);
        }
        if first == b"trash" {
            return self.execute_trash_resource_command(command);
        }
        if first == b"shell" {
            return self.execute_shell_profile_command(command);
        }
        if first == b"commands" {
            let group = command_word(command, 1).unwrap_or(b"all");
            if matches!(group, b"all" | b"navigation") {
                self.output.write_line(b"path idir cd list examine resolve references versions relationships find open navigator");
            }
            if matches!(group, b"all" | b"objects") {
                self.output.write_line(b"namespace create/delete/move/list; object create/copy/delete/destroy; reference create/delete/list; trash add/list/restore/delete/empty");
            }
            if matches!(group, b"all" | b"shell") {
                self.output.write_line(b"shell profile list/inspect/create/clone/enable/disable/set-default/delete; shell alias list/add/delete/resolve");
            }
            return true;
        }
        false
    }

    // ------------------------=
    // FUNC: execute_namespace_resource_command
    // DESC: Executes typed Namespace lifecycle commands with safe empty-container deletion.
    // ------------------=
    fn execute_namespace_resource_command(&mut self, command: &[u8]) -> bool {
        let action = command_word(command, 1).unwrap_or(&[]);
        let source = command_word(command, 2).unwrap_or(&[]);
        let result = match action {
            b"create" => crate::storage::namespace_create(source).map(|id| Some(id)),
            b"delete" => crate::storage::namespace_delete(source).map(|id| Some(id)),
            b"move" => {
                let destination = command_word(command, 3).unwrap_or(&[]);
                crate::storage::namespace_move(source, destination)
                    .map(|_| crate::storage::namespace_resolve(destination).ok())
            }
            b"list" => {
                let path = if source.is_empty() {
                    self.navigation_context.path()
                } else {
                    source
                };
                for index in 0..32usize {
                    let Ok(Some(entry)) = crate::storage::namespace_list_nth(path, index) else {
                        break;
                    };
                    self.output
                        .write_line(&entry.path[..entry.path_len as usize]);
                }
                return true;
            }
            _ => return false,
        };
        match result {
            Ok(Some(id)) => self.output.write_id(b"ObjectId: ", id),
            Ok(None) => self.output.write_line(b"Namespace operation committed."),
            Err(error) => self.storage_error(error),
        }
        true
    }

    // ------------------------=
    // FUNC: execute_object_resource_command
    // DESC: Executes typed Object create, copy, safe delete, and explicit destroy commands.
    // ------------------=
    fn execute_object_resource_command(&mut self, command: &[u8]) -> bool {
        let action = command_word(command, 1).unwrap_or(&[]);
        let source = command_word(command, 2).unwrap_or(&[]);
        match action {
            b"create" => {
                let name = source.rsplit(|byte| *byte == b'/').next().unwrap_or(source);
                match crate::storage::object_create_note_at(name, b"", source) {
                    Ok(id) => self.output.write_id(b"ObjectId: ", id),
                    Err(error) => self.storage_error(error),
                }
            }
            b"copy" => match crate::storage::object_copy_path(
                source,
                command_word(command, 3).unwrap_or(&[]),
            ) {
                Ok(id) => self.output.write_id(b"New ObjectId: ", id),
                Err(error) => self.storage_error(error),
            },
            b"delete" => match crate::storage::trash_move(source) {
                Ok(id) => self.output.write_id(b"Moved to Trash: ", id),
                Err(error) => self.storage_error(error),
            },
            b"destroy" => {
                let confirmed = command_word(command, 3) == Some(b"confirm=true");
                match crate::storage::object_destroy_explicit(source, confirmed) {
                    Ok(()) => self
                        .output
                        .write_line(b"Object identity permanently destroyed."),
                    Err(error) => self.storage_error(error),
                }
            }
            b"inspect" | b"history" | b"relationships" => {
                let alias = if action == b"history" {
                    b"versions".as_slice()
                } else if action == b"relationships" {
                    b"relationships".as_slice()
                } else {
                    b"examine".as_slice()
                };
                let mut translated = [0u8; COMMAND_CAPACITY];
                translated[..alias.len()].copy_from_slice(alias);
                translated[alias.len()] = b' ';
                translated[alias.len() + 1..alias.len() + 1 + source.len()].copy_from_slice(source);
                return self.execute_native_navigation_command(
                    &translated[..alias.len() + 1 + source.len()],
                );
            }
            _ => return false,
        }
        true
    }

    // ------------------------=
    // FUNC: execute_reference_resource_command
    // DESC: Executes additional-reference creation, selected-reference deletion, and reference enumeration.
    // ------------------=
    fn execute_reference_resource_command(&mut self, command: &[u8]) -> bool {
        let action = command_word(command, 1).unwrap_or(&[]);
        let source = command_word(command, 2).unwrap_or(&[]);
        match action {
            b"create" => match crate::storage::namespace_link(
                source,
                command_word(command, 3).unwrap_or(&[]),
            ) {
                Ok(id) => self.output.write_id(b"Referenced ObjectId: ", id),
                Err(error) => self.storage_error(error),
            },
            b"delete" => match crate::storage::namespace_detach(source) {
                Ok(()) => self
                    .output
                    .write_line(b"Selected Namespace reference removed; ObjectId retained."),
                Err(error) => self.storage_error(error),
            },
            b"list" => {
                let mut translated = [0u8; COMMAND_CAPACITY];
                translated[..11].copy_from_slice(b"references ");
                translated[11..11 + source.len()].copy_from_slice(source);
                return self.execute_native_navigation_command(&translated[..11 + source.len()]);
            }
            _ => return false,
        }
        true
    }

    // ------------------------=
    // FUNC: execute_trash_resource_command
    // DESC: Executes recoverable Trash lifecycle commands against the authoritative Namespace layer.
    // ------------------=
    fn execute_trash_resource_command(&mut self, command: &[u8]) -> bool {
        let action = command_word(command, 1).unwrap_or(&[]);
        let target = command_word(command, 2).unwrap_or(&[]);
        match action {
            b"add" => match crate::storage::trash_move(target) {
                Ok(id) => self.output.write_id(b"Trash ObjectId: ", id),
                Err(error) => self.storage_error(error),
            },
            b"list" => {
                for index in 0..32usize {
                    let Ok(Some(entry)) = crate::storage::namespace_list_nth(b"/trash", index)
                    else {
                        break;
                    };
                    self.output
                        .write_line(&entry.path[..entry.path_len as usize]);
                }
            }
            b"restore" => match crate::storage::trash_restore(target) {
                Ok(id) => self.output.write_id(b"Restored ObjectId: ", id),
                Err(error) => self.storage_error(error),
            },
            b"delete" => match crate::storage::trash_delete(target) {
                Ok(id) => self.output.write_id(b"Permanently deleted ObjectId: ", id),
                Err(error) => self.storage_error(error),
            },
            b"empty" => match crate::storage::trash_empty() {
                Ok(count) => self
                    .output
                    .write_number(b"Trash entries deleted: ", count as u64),
                Err(error) => self.storage_error(error),
            },
            _ => return false,
        }
        true
    }

    // ------------------------=
    // FUNC: execute_shell_profile_command
    // DESC: Executes persistent capability-aware Shell Profile and alias lifecycle operations.
    // ------------------=
    fn execute_shell_profile_command(&mut self, command: &[u8]) -> bool {
        let resource = command_word(command, 1).unwrap_or(&[]);
        if resource == b"status" {
            crate::runtime::with_runtime(|runtime| {
                if let Some(profiles) = runtime.shell_profiles.as_ref() {
                    for index in 0..profiles.profile_count() {
                        if let Some(profile) = profiles
                            .profile_nth(index)
                            .filter(|profile| profile.enabled)
                        {
                            self.output.write_line(profile.name.as_bytes());
                        }
                    }
                }
            });
            return true;
        }
        let action = command_word(command, 2).unwrap_or(&[]);
        let name = command_word(command, 3).unwrap_or(&[]);
        let actor = self.current_user.short();
        let now = self.desktop_clock.second as u64 + 1;
        if resource == b"profile" && action == b"list" {
            crate::runtime::with_runtime(|runtime| {
                if let Some(profiles) = runtime.shell_profiles.as_ref() {
                    for index in 0..profiles.profile_count() {
                        if let Some(profile) = profiles.profile_nth(index) {
                            self.output.write_segments(&[
                                profile.name.as_bytes(),
                                if profile.enabled {
                                    b" [enabled]"
                                } else {
                                    b" [disabled]"
                                },
                            ]);
                        }
                    }
                }
            });
            return true;
        }
        let result = crate::runtime::with_runtime(|runtime| {
            let profiles = runtime
                .shell_profiles
                .as_mut()
                .ok_or(crate::runtime::object_navigation::ProfileError::CorruptState)?;
            match (resource, action) {
                (b"profile", b"inspect") => profiles
                    .profile(name)
                    .map(|_| ())
                    .ok_or(crate::runtime::object_navigation::ProfileError::NotFound),
                (b"profile", b"create") => profiles.create(actor, name, now).map(|_| ()),
                (b"profile", b"clone") => profiles
                    .clone_profile(actor, name, command_word(command, 4).unwrap_or(&[]), now)
                    .map(|_| ()),
                (b"profile", b"enable") => profiles.enable(actor, name, now),
                (b"profile", b"disable") => profiles.disable(actor, name, now),
                (b"profile", b"set-default") => profiles.set_default(actor, name),
                (b"profile", b"delete") => profiles.delete(actor, name).map(|_| ()),
                (b"alias", b"add") => profiles.alias_add(
                    actor,
                    name,
                    command_word(command, 4).unwrap_or(&[]),
                    unquote_command_tail(command_tail(command, 5).unwrap_or(&[])),
                    now,
                ),
                (b"alias", b"delete") => {
                    profiles.alias_delete(actor, name, command_word(command, 4).unwrap_or(&[]), now)
                }
                (b"alias", b"resolve") => profiles.resolve(name).map(|_| ()),
                (b"alias", b"list") => profiles
                    .profile(name)
                    .map(|_| ())
                    .ok_or(crate::runtime::object_navigation::ProfileError::NotFound),
                _ => return Err(crate::runtime::object_navigation::ProfileError::NotFound),
            }
        })
        .unwrap_or(Err(
            crate::runtime::object_navigation::ProfileError::CorruptState,
        ));
        match result {
            Ok(()) => {
                let _ = crate::runtime::persist_shell_profile_state();
                self.output.write_line(b"Shell Profile operation committed.");
            }
            Err(_) => self.output.write_line(b"Shell Profile operation rejected by validation, ownership, or immutability policy."),
        }
        true
    }

    // ------------------------=
    // FUNC: execute_language_command
    // DESC: Parses schema-registered Console syntax and executes or renders its typed operation graph.
    // ------------------=
    fn execute_language_command(&mut self, command: &[u8]) -> bool {
        use crate::runtime::console_language::{
            self, ConsoleLanguageError, ParseOutcome, SideEffectClass,
        };
        let first = command
            .split(|byte| byte.is_ascii_whitespace())
            .next()
            .unwrap_or(&[]);
        let known_domain = console_language::domain(first).is_some()
            || first == b"help"
            || first == b"plan"
            || command
                .iter()
                .position(|byte| *byte == b'=')
                .map(|at| {
                    console_language::domain(
                        command[at + 1..]
                            .split(|byte| byte.is_ascii_whitespace())
                            .find(|part| !part.is_empty())
                            .unwrap_or(&[]),
                    )
                    .is_some()
                })
                .unwrap_or(false);
        let parsed = match console_language::parse(command) {
            Ok(value) => value,
            Err(ConsoleLanguageError::UnknownOperation) if known_domain => {
                if self.execute_runtime_command(command)
                    || self.execute_ai_command(command)
                    || self.execute_storage_command(command)
                {
                    return true;
                }
                self.output
                    .write_line(language_error_text(ConsoleLanguageError::UnknownOperation));
                self.output
                    .write_line(b"Type the domain name or help <domain> to discover operations.");
                return true;
            }
            Err(error) if known_domain => {
                self.output.write_line(language_error_text(error));
                if matches!(
                    error,
                    ConsoleLanguageError::UnknownOperation | ConsoleLanguageError::MissingArgument
                ) {
                    self.output.write_line(
                        b"Type the domain name or help <domain> to discover operations.",
                    );
                }
                return true;
            }
            Err(_) => return false,
        };
        match parsed {
            ParseOutcome::DomainDiscovery(domain) => {
                self.output
                    .write_segments(&[domain.name, b" - ", domain.description]);
                self.output.write_line(b"Available operations:");
                for operation in console_language::OPERATIONS
                    .iter()
                    .filter(|item| item.domain == domain.name)
                {
                    self.output
                        .write_segments(&[b"  ", operation.domain, b" ", operation.action]);
                }
            }
            ParseOutcome::OperationDiscovery(operation) => {
                self.output.write_segments(&[
                    operation.domain,
                    b" ",
                    operation.action,
                    b" - ",
                    operation.description,
                ]);
                self.output
                    .write_line(b"More information is required. Example:");
                self.output.write_segments(&[b"  ", operation.example]);
            }
            ParseOutcome::Help(domain, operation) => {
                crate::output_text(b"[operation] help\n");
                if let Some(operation) = operation {
                    self.output
                        .write_segments(&[operation.domain, b" ", operation.action]);
                    self.output.write_line(operation.description);
                    self.output
                        .write_segments(&[b"Result type: ", value_type_text(operation.output)]);
                    self.output
                        .write_segments(&[b"Example: ", operation.example]);
                } else if let Some(domain) = domain {
                    self.output
                        .write_segments(&[domain.name, b" - ", domain.description]);
                    for operation in console_language::OPERATIONS
                        .iter()
                        .filter(|item| item.domain == domain.name)
                    {
                        self.output.write_segments(&[
                            b"  ",
                            operation.action,
                            b" - ",
                            operation.description,
                        ]);
                    }
                } else {
                    self.output
                        .write_line(b"Infinity Console: DOMAIN ACTION [TARGET] [key=value]");
                    self.output.write_line(
                        b"Use |> for typed composition; use plan before meaningful changes.",
                    );
                    self.output.write_line(
                        b"Domains: system device storage object namespace project collection",
                    );
                    self.output
                        .write_line(b"service runtime task capability event ai model agent voice");
                }
            }
            ParseOutcome::Graph(graph) => {
                crate::output_text(b"[console] typed operation graph validated\n");
                let executable_node_mutation = graph.node_count == 1
                    && graph.nodes[0]
                        .map(|node| is_node_console_mutation(node.schema.operation))
                        .unwrap_or(false);
                if graph.plan_only
                    || (graph.maximum_effect != SideEffectClass::Query && !executable_node_mutation)
                {
                    self.output
                        .write_number(b"Operation plan stages: ", graph.node_count as u64);
                    self.output
                        .write_segments(&[b"Result type: ", value_type_text(graph.result_type)]);
                    self.output.write_segments(&[
                        b"Side effect: ",
                        side_effect_text(graph.maximum_effect),
                    ]);
                    self.output
                        .write_line(if graph.maximum_effect == SideEffectClass::Query {
                            b"Plan validated; no operation executed."
                        } else {
                            b"Policy review and capability confirmation required."
                        });
                } else if graph.node_count > 1 {
                    self.output
                        .write_number(b"Typed composition stages: ", graph.node_count as u64);
                    self.output
                        .write_segments(&[b"Result type: ", value_type_text(graph.result_type)]);
                    self.output
                        .write_line(b"Composition validated without rendered-text parsing.");
                } else if let Some(node) = graph.nodes[0] {
                    if let Some(name) = graph.assignment {
                        let _ = self.language_session.assign(name, graph.result_type);
                    }
                    if node.schema.domain == b"object" && node.schema.action == b"inspect" {
                        return self.execute_storage_command(command);
                    }
                    return self.execute_registered_node(&node);
                }
            }
        }
        true
    }

    // ------------------------=
    // FUNC: execute_registered_node
    // DESC: Routes one validated schema node to an existing typed operation implementation.
    // ------------------=
    fn execute_registered_node(
        &mut self,
        node: &crate::runtime::console_language::OperationNode<'_>,
    ) -> bool {
        use crate::runtime::iop::OperationId;
        match node.schema.operation {
            OperationId::SystemStatus => self.system_status(),
            OperationId::SystemInfo => self.system_info(),
            OperationId::SystemGenerationInspect => self.system_generation(),
            OperationId::SystemBootStatus => self.system_boot(),
            OperationId::DeviceList => self.device_list(),
            OperationId::StorageUsage | OperationId::StorageQuery => {
                if node.schema.action == b"usage" {
                    return self.execute_storage_command(b"storage usage");
                }
                self.output.write_line(b"Infinity Pool: online");
                self.output
                    .write_line(b"Organization: objects + relationships + namespace views");
            }
            OperationId::ObjectQuery => self.render_object_query(node),
            OperationId::NetworkStatus
            | OperationId::NetworkInterfaceList
            | OperationId::NetworkInterfaceInspect
            | OperationId::NetworkAddressList
            | OperationId::NetworkRouteList
            | OperationId::NetworkConnectionList
            | OperationId::NetworkConnectionInspect
            | OperationId::NetworkPolicyList
            | OperationId::NetworkPolicyInspect
            | OperationId::NetworkProfileList
            | OperationId::NetworkProfileInspect
            | OperationId::NetworkDiagnostics
            | OperationId::ServiceDiscoverLocal => return self.execute_network_node(node),
            OperationId::ProjectList => self.render_typed_object_list(
                crate::storage::object::ObjectType::Project,
                b"ProjectSet",
            ),
            OperationId::ProjectInspect => {
                self.inspect_organization(node, crate::storage::object::ObjectType::Project)
            }
            OperationId::ProjectCreate => {
                self.create_organization(node, crate::storage::object::ObjectType::Project)
            }
            OperationId::CollectionList => self.render_typed_object_list(
                crate::storage::object::ObjectType::Collection,
                b"CollectionSet",
            ),
            OperationId::CollectionInspect => {
                self.inspect_organization(node, crate::storage::object::ObjectType::Collection)
            }
            OperationId::CollectionCreate => {
                self.create_organization(node, crate::storage::object::ObjectType::Collection)
            }
            OperationId::IdentityCreate
            | OperationId::IdentityList
            | OperationId::IdentityRead
            | OperationId::IdentityUpdate
            | OperationId::IdentityDelete
            | OperationId::MachineRead
            | OperationId::MachineUpdate
            | OperationId::CredentialCreate
            | OperationId::CredentialList
            | OperationId::CredentialDelete
            | OperationId::SessionList
            | OperationId::SessionRead
            | OperationId::SessionLock
            | OperationId::SessionEnd
            | OperationId::ProfileRead
            | OperationId::ProfileUpdate
            | OperationId::PersonalSpaceRead
            | OperationId::AiProfileRead
            | OperationId::AiProfileUpdate
            | OperationId::VoiceProfileRead
            | OperationId::VoiceProfileUpdate
            | OperationId::SettingsRead
            | OperationId::SettingsUpdate => return self.execute_identity_node(node),
            OperationId::NodeList
            | OperationId::NodeInspect
            | OperationId::NodeDiscoverStatus
            | OperationId::NodeTrustRead
            | OperationId::NodeSessionList
            | OperationId::NodeSessionInspect
            | OperationId::NodeCapabilityList
            | OperationId::NodePolicyRead
            | OperationId::NodeHealth
            | OperationId::NodeDiagnostics
            | OperationId::NodeAuditList
            | OperationId::NodeAuditInspect
            | OperationId::NodeDomainList
            | OperationId::NodeDomainInspect
            | OperationId::MeshStatus
            | OperationId::MeshMemberList
            | OperationId::MeshPolicyRead => return self.execute_node_query(node),
            OperationId::NodePairBegin
            | OperationId::NodePairConfirm
            | OperationId::NodePairCancel
            | OperationId::NodeTrustUpdate
            | OperationId::NodeRevokeTrust
            | OperationId::NodeBlock
            | OperationId::NodeUnblock
            | OperationId::NodeSessionClose
            | OperationId::NodeCapabilityRevoke
            | OperationId::NodePolicyUpdate
            | OperationId::MeshPolicyUpdate
            | OperationId::MeshMemberAdd
            | OperationId::MeshMemberRemove
            | OperationId::NodeJoin
            | OperationId::NodeLeave => return self.execute_node_mutation(node),
            _ => {
                // Existing service-specific handlers remain the typed operation adapters
                // until all services accept native IOP payloads directly.
                return self.execute_runtime_command_for_operation(node.schema.operation);
            }
        }
        true
    }

    // ------------------------=
    // FUNC: execute_node_query
    // DESC: Projects typed node, session, authority, audit, and mesh state without parsing Console text.
    // ------------------=
    fn execute_node_query(
        &mut self,
        node: &crate::runtime::console_language::OperationNode<'_>,
    ) -> bool {
        use crate::runtime::iop::OperationId;
        crate::runtime::with_runtime(|runtime| match node.schema.operation {
            OperationId::NodeSessionList | OperationId::NodeSessionInspect => {
                self.output.write_number(
                    b"NodeSessionSet count: ",
                    runtime.nodes.sessions().iter().flatten().count() as u64,
                );
            }
            OperationId::NodeCapabilityList => {
                self.output.write_number(
                    b"RemoteCapabilitySet count: ",
                    runtime.nodes.remote_grants().iter().flatten().count() as u64,
                );
            }
            OperationId::NodeAuditList | OperationId::NodeAuditInspect => {
                self.output.write_number(
                    b"NodeAuditSet count: ",
                    runtime.nodes.audit_records().iter().flatten().count() as u64,
                );
            }
            OperationId::NodeDomainList
            | OperationId::NodeDomainInspect
            | OperationId::MeshStatus
            | OperationId::MeshMemberList
            | OperationId::MeshPolicyRead => {
                self.output.write_number(
                    b"MeshDomainSet member count: ",
                    runtime
                        .nodes
                        .mesh_members()
                        .iter()
                        .flatten()
                        .filter(|member| member.enabled)
                        .count() as u64,
                );
            }
            OperationId::NodeTrustRead | OperationId::NodePolicyRead => {
                let trusted = runtime
                    .nodes
                    .discovered_nodes()
                    .iter()
                    .flatten()
                    .filter(|peer| {
                        matches!(
                            peer.trust,
                            crate::runtime::node::types::TrustState::Trusted
                                | crate::runtime::node::types::TrustState::Restricted
                        )
                    })
                    .count();
                self.output
                    .write_number(b"NodePolicy trusted peers: ", trusted as u64);
            }
            _ => {
                self.output.write_number(
                    b"NodeSet count: ",
                    runtime.nodes.discovered_nodes().iter().flatten().count() as u64,
                );
            }
        });
        true
    }

    // ------------------------=
    // FUNC: execute_node_mutation
    // DESC: Executes one deterministic Console node mutation through the shared typed node-operation service adapter and reports committed state only.
    // ------------------=
    fn execute_node_mutation(
        &mut self,
        node: &crate::runtime::console_language::OperationNode<'_>,
    ) -> bool {
        use crate::runtime::iop::{NodeOperationV1, OperationId, NODE_OPERATION_HUMAN_APPROVED};

        let mut request = NodeOperationV1 {
            node_id: [0; 32],
            handle: 0,
            scope: 0,
            lease_deadline: 0,
            operation: node.schema.operation.machine_id(),
            rights: 0,
            value: 0,
            flags: 0,
            schema_version: 1,
        };
        let target = node.target.map(|reference| reference.value);
        match node.schema.operation {
            OperationId::NodePairConfirm => {
                let Some(pairing_id) = target.and_then(parse_u64_decimal) else {
                    self.output.write_line(b"Invalid pairing reference.");
                    return true;
                };
                let Some(code_bytes) = node_argument(node, b"code") else {
                    self.output
                        .write_line(b"A six-digit verification code is required.");
                    return true;
                };
                if code_bytes.len() != 6 || code_bytes.iter().any(|byte| !byte.is_ascii_digit()) {
                    self.output
                        .write_line(b"A six-digit verification code is required.");
                    return true;
                }
                let Some(code) = parse_u32_decimal(code_bytes) else {
                    return true;
                };
                request.handle = pairing_id;
                request.value = code;
                request.flags = NODE_OPERATION_HUMAN_APPROVED;
            }
            OperationId::NodePairCancel
            | OperationId::NodeSessionClose
            | OperationId::NodeCapabilityRevoke => {
                let Some(handle) = target.and_then(parse_u64_decimal) else {
                    self.output.write_line(b"Invalid operation handle.");
                    return true;
                };
                request.handle = handle;
            }
            _ => {
                let Some(id) = target.and_then(parse_node_id) else {
                    self.output.write_line(b"A full NodeId is required.");
                    return true;
                };
                request.node_id = id.0;
            }
        }
        if node.schema.operation == OperationId::NodeTrustUpdate {
            if node_argument(node, b"name") != Some(b"state".as_slice()) {
                self.output.write_line(b"Supported trust field: state");
                return true;
            }
            request.value = match node_argument(node, b"value") {
                Some(b"untrusted") => 1,
                Some(b"trusted") => 3,
                Some(b"restricted") => 4,
                Some(b"revoked") => 5,
                Some(b"blocked") => 6,
                _ => {
                    self.output.write_line(b"Invalid trust state.");
                    return true;
                }
            };
        }
        if matches!(
            node.schema.operation,
            OperationId::NodePolicyUpdate | OperationId::MeshPolicyUpdate
        ) {
            let Some(category) = node_argument(node, b"name").and_then(node_policy_category) else {
                self.output.write_line(b"Invalid policy category.");
                return true;
            };
            request.flags = category as u32;
            request.value = match node_argument(node, b"value") {
                Some(b"deny") => 0,
                Some(b"allow") => 1,
                Some(b"session") | Some(b"session-only") => 2,
                Some(b"leased") => 3,
                _ => {
                    self.output
                        .write_line(b"Policy value must be deny, allow, session, or leased.");
                    return true;
                }
            };
        }
        let now = crate::runtime::with_runtime(|runtime| {
            runtime
                .nodes
                .audit_records()
                .iter()
                .flatten()
                .map(|record| record.timestamp)
                .max()
                .unwrap_or(0)
                .saturating_add(1)
        })
        .unwrap_or(1);
        let result = crate::runtime::with_runtime(|runtime| {
            crate::runtime::iop::execute_node_operation(
                &mut runtime.nodes,
                node.schema.operation,
                request,
                now,
                now,
            )
        });
        let Ok(response) = result.unwrap_or(Err(crate::runtime::iop::IopError::InvalidPayload))
        else {
            self.output.write_line(b"Node operation denied or invalid.");
            return true;
        };
        if !crate::runtime::persist_node_state() {
            self.output
                .write_line(b"Node state commit failed; no success was reported.");
            return true;
        }
        let event_type = node_event_for_operation(node.schema.operation);
        if event_type != 0 {
            let event_node = crate::runtime::node::types::NodeId(response.node_id);
            let _ = crate::runtime::publish_node_state_event(event_type, event_node, now, now);
        }
        if node.schema.operation == OperationId::NodePairBegin {
            self.output
                .write_number(b"Pairing transaction: pairing:", response.handle);
            self.output
                .write_number(b"Verification code: ", response.value as u64);
            self.output
                .write_line(b"Confirm only after independently verifying the remote node.");
        } else {
            self.output.write_line(b"Node operation committed.");
        }
        true
    }

    // ------------------------=
    // FUNC: execute_network_node
    // DESC: Renders authorized typed Network Service results without parsing shell output.
    // ------------------=
    fn execute_network_node(
        &mut self,
        node: &crate::runtime::console_language::OperationNode<'_>,
    ) -> bool {
        use crate::runtime::iop::OperationId;
        use crate::runtime::network::types::ConnectivityClass;
        match node.schema.operation {
            OperationId::NetworkStatus => {
                let status =
                    crate::runtime::with_runtime(|runtime| runtime.network.status()).unwrap();
                let class = match status.connectivity {
                    ConnectivityClass::Offline => b"Offline".as_slice(),
                    ConnectivityClass::LinkOnly => b"LinkOnly".as_slice(),
                    ConnectivityClass::LocalNetwork => b"LocalNetwork".as_slice(),
                    ConnectivityClass::LimitedConnectivity => b"LimitedConnectivity".as_slice(),
                    ConnectivityClass::Routed => b"Routed".as_slice(),
                    ConnectivityClass::InternetReachableOptional => {
                        b"InternetReachableOptional".as_slice()
                    }
                    ConnectivityClass::Degraded => b"Degraded".as_slice(),
                };
                self.output.write_segments(&[b"connectivity: ", class]);
                self.output.write_number(
                    b"active profile: network-profile:",
                    status.active_profile as u64,
                );
                self.output
                    .write_number(b"interfaces: ", status.interfaces as u64);
                self.output
                    .write_number(b"connections: ", status.active_connections as u64);
            }
            OperationId::NetworkInterfaceList | OperationId::NetworkInterfaceInspect => {
                let count = crate::runtime::with_runtime(|runtime| {
                    runtime.network.interfaces.interface_count()
                })
                .unwrap_or(0);
                for index in 0..count {
                    if let Some(interface) = crate::runtime::with_runtime(|runtime| {
                        runtime.network.interfaces.interface_nth(index).copied()
                    })
                    .flatten()
                    {
                        self.output.write_number(b"interface:", interface.id as u64);
                        self.output
                            .write_number(b"  device identity: ", interface.device.device_id);
                        self.output
                            .write_number(b"  rx packets: ", interface.rx_packets);
                        self.output
                            .write_number(b"  tx packets: ", interface.tx_packets);
                    }
                }
            }
            OperationId::NetworkAddressList => {
                let count = crate::runtime::with_runtime(|runtime| {
                    runtime.network.interfaces.address_count()
                })
                .unwrap_or(0);
                for index in 0..count {
                    if let Some(address) = crate::runtime::with_runtime(|runtime| {
                        runtime.network.interfaces.address_nth(index).copied()
                    })
                    .flatten()
                    {
                        self.output.write_number(b"address:", address.id as u64);
                        self.output
                            .write_number(b"  interface: ", address.interface_id as u64);
                        self.output
                            .write_number(b"  prefix: ", address.prefix_length as u64);
                    }
                }
            }
            OperationId::NetworkRouteList => {
                let count = crate::runtime::with_runtime(|runtime| {
                    runtime.network.interfaces.route_count()
                })
                .unwrap_or(0);
                for index in 0..count {
                    if let Some(route) = crate::runtime::with_runtime(|runtime| {
                        runtime.network.interfaces.route_nth(index).copied()
                    })
                    .flatten()
                    {
                        self.output.write_number(b"route:", route.id as u64);
                        self.output
                            .write_number(b"  interface: ", route.interface_id as u64);
                        self.output
                            .write_number(b"  prefix: ", route.prefix_length as u64);
                        self.output.write_number(b"  metric: ", route.metric as u64);
                    }
                }
            }
            OperationId::NetworkConnectionList | OperationId::NetworkConnectionInspect => {
                let count =
                    crate::runtime::with_runtime(|runtime| runtime.network.connections.count())
                        .unwrap_or(0);
                self.output
                    .write_number(b"authorized connections: ", count as u64);
            }
            OperationId::NetworkPolicyList | OperationId::NetworkPolicyInspect => {
                let count = crate::runtime::with_runtime(|runtime| runtime.network.policy.count())
                    .unwrap_or(0);
                self.output
                    .write_number(b"effective policy rules: ", count as u64);
            }
            OperationId::NetworkProfileList | OperationId::NetworkProfileInspect => {
                let count =
                    crate::runtime::with_runtime(|runtime| runtime.network.profiles.count())
                        .unwrap_or(0);
                let active =
                    crate::runtime::with_runtime(|runtime| runtime.network.profiles.active_id())
                        .unwrap_or(0);
                for index in 0..count {
                    if let Some(profile) = crate::runtime::with_runtime(|runtime| {
                        runtime.network.profiles.nth(index).copied()
                    })
                    .flatten()
                    {
                        self.output.write_number(
                            if profile.id == active {
                                b"active network-profile:"
                            } else {
                                b"network-profile:"
                            },
                            profile.id as u64,
                        );
                    }
                }
            }
            OperationId::NetworkDiagnostics => {
                let diagnostics =
                    crate::runtime::with_runtime(|runtime| runtime.network.diagnostics()).unwrap();
                self.output
                    .write_number(b"rx packets: ", diagnostics.rx_packets);
                self.output
                    .write_number(b"tx packets: ", diagnostics.tx_packets);
                self.output.write_number(b"drops: ", diagnostics.drops);
                self.output
                    .write_number(b"queue pressure: ", diagnostics.queue_pressure);
                self.output
                    .write_number(b"policy denials: ", diagnostics.policy_denials);
            }
            OperationId::ServiceDiscoverLocal => {
                let count =
                    crate::runtime::with_runtime(|runtime| runtime.network.discovery.count())
                        .unwrap_or(0);
                self.output
                    .write_number(b"discovered untrusted services: ", count as u64);
            }
            _ => return false,
        }
        true
    }

    // ------------------------=
    // FUNC: execute_identity_node
    // DESC: Executes identity, session, and preference schemas against the shared native services.
    // ------------------=
    fn execute_identity_node(
        &mut self,
        node: &crate::runtime::console_language::OperationNode<'_>,
    ) -> bool {
        use crate::runtime::identity::{AiProviderPolicy, IdentityError, VoiceActivation};
        use crate::runtime::iop::OperationId;
        let argument = |name: &[u8]| {
            node.arguments
                .iter()
                .flatten()
                .find(|argument| argument.name == name)
                .map(|argument| argument.value)
        };
        let target_number = node
            .target
            .and_then(|target| parse_reference_number(target.value));
        let actor = self.current_user;
        let administrator = crate::runtime::with_runtime(|runtime| {
            runtime
                .identity
                .session_by_short(self.current_session.short())
                .map(|session| {
                    session.capabilities & crate::runtime::identity::SESSION_IDENTITY_MANAGE != 0
                })
                .unwrap_or(false)
        })
        .unwrap_or(false);
        let result: Result<(), IdentityError> = (|| match node.schema.operation {
            OperationId::IdentityList => {
                let count = crate::runtime::with_runtime(|runtime| runtime.identity.user_count())
                    .unwrap_or(0);
                for index in 0..count {
                    if let Some(user) =
                        crate::runtime::with_runtime(|runtime| runtime.identity.user_nth(index))
                            .flatten()
                    {
                        self.output.write_number(b"user:", user.id.short());
                        self.output.write_segments(&[
                            b"  ",
                            user.display_name.as_bytes(),
                            b"  @",
                            user.handle.as_bytes(),
                        ]);
                    }
                }
                Ok(())
            }
            OperationId::IdentityRead => {
                let user = target_number
                    .and_then(|id| {
                        crate::runtime::with_runtime(|runtime| runtime.identity.user_by_short(id))
                            .flatten()
                    })
                    .or_else(|| {
                        crate::runtime::with_runtime(|runtime| runtime.identity.user(actor))
                            .flatten()
                    })
                    .ok_or(IdentityError::NotFound)?;
                self.output.write_number(b"id: user:", user.id.short());
                self.output
                    .write_segments(&[b"name: ", user.display_name.as_bytes()]);
                self.output
                    .write_segments(&[b"handle: ", user.handle.as_bytes()]);
                Ok(())
            }
            OperationId::IdentityCreate => {
                if !administrator {
                    Err(IdentityError::AccessDenied)
                } else {
                    let handle = argument(b"handle").ok_or(IdentityError::InvalidInput)?;
                    let name = argument(b"display-name").ok_or(IdentityError::InvalidInput)?;
                    let user = crate::runtime::with_runtime(|runtime| {
                        runtime.identity.create_user(handle, name, 20)
                    })
                    .unwrap_or(Err(IdentityError::InvalidState))?;
                    self.output
                        .write_number(b"user created: user:", user.id.short());
                    if crate::runtime::persist_identity_state() {
                        Ok(())
                    } else {
                        Err(IdentityError::InvalidState)
                    }
                }
            }
            OperationId::IdentityUpdate => {
                let id = target_number
                    .and_then(|id| {
                        crate::runtime::with_runtime(|runtime| runtime.identity.user_by_short(id))
                            .flatten()
                    })
                    .map(|user| user.id)
                    .ok_or(IdentityError::NotFound)?;
                let name = argument(b"display-name").ok_or(IdentityError::InvalidInput)?;
                crate::runtime::with_runtime(|runtime| {
                    runtime
                        .identity
                        .update_user_name(actor, id, name, administrator)
                })
                .unwrap_or(Err(IdentityError::InvalidState))?;
                if crate::runtime::persist_identity_state() {
                    Ok(())
                } else {
                    Err(IdentityError::InvalidState)
                }
            }
            OperationId::IdentityDelete => {
                let id = target_number
                    .and_then(|id| {
                        crate::runtime::with_runtime(|runtime| runtime.identity.user_by_short(id))
                            .flatten()
                    })
                    .map(|user| user.id)
                    .ok_or(IdentityError::NotFound)?;
                crate::runtime::with_runtime(|runtime| {
                    runtime.identity.deactivate_user(actor, id, administrator)
                })
                .unwrap_or(Err(IdentityError::InvalidState))?;
                if crate::runtime::persist_identity_state() {
                    Ok(())
                } else {
                    Err(IdentityError::InvalidState)
                }
            }
            OperationId::MachineRead => {
                let machine = crate::runtime::with_runtime(|runtime| runtime.identity.machine())
                    .flatten()
                    .ok_or(IdentityError::NotFound)?;
                self.output
                    .write_number(b"id: machine:", machine.id.short());
                self.output
                    .write_segments(&[b"name: ", machine.display_name.as_bytes()]);
                self.output
                    .write_number(b"system generation: ", machine.system_generation);
                Ok(())
            }
            OperationId::MachineUpdate => {
                let name = argument(b"name").ok_or(IdentityError::InvalidInput)?;
                crate::runtime::with_runtime(|runtime| runtime.identity.update_machine_name(name))
                    .unwrap_or(Err(IdentityError::InvalidState))?;
                if crate::runtime::persist_identity_state() {
                    Ok(())
                } else {
                    Err(IdentityError::InvalidState)
                }
            }
            OperationId::CredentialCreate => {
                self.output
                    .write_line(b"Credential creation requires private masked entry in Settings.");
                Ok(())
            }
            OperationId::CredentialList => {
                let mut index = 0;
                while let Some(credential) =
                    crate::runtime::with_runtime(|runtime| runtime.identity.credential_nth(index))
                        .flatten()
                {
                    self.output
                        .write_number(b"credential:", credential.id.short());
                    self.output
                        .write_number(b"  owner user:", credential.owner.short());
                    index += 1;
                }
                Ok(())
            }
            OperationId::CredentialDelete => {
                let id = target_number.ok_or(IdentityError::InvalidInput)?;
                let credential = crate::runtime::with_runtime(|runtime| {
                    (0..crate::runtime::identity::MAX_CREDENTIALS).find_map(|index| {
                        runtime
                            .identity
                            .credential_nth(index)
                            .filter(|value| value.id.short() == id)
                    })
                })
                .flatten()
                .ok_or(IdentityError::NotFound)?;
                crate::runtime::with_runtime(|runtime| {
                    runtime
                        .identity
                        .revoke_credential(actor, credential.id, administrator)
                })
                .unwrap_or(Err(IdentityError::InvalidState))?;
                if crate::runtime::persist_identity_state() {
                    Ok(())
                } else {
                    Err(IdentityError::InvalidState)
                }
            }
            OperationId::SessionList => {
                let mut index = 0;
                while let Some(session) =
                    crate::runtime::with_runtime(|runtime| runtime.identity.session_nth(index))
                        .flatten()
                {
                    self.output.write_number(b"session:", session.id.short());
                    self.output.write_number(b"  user:", session.user.short());
                    index += 1;
                }
                Ok(())
            }
            OperationId::SessionRead => {
                let session = target_number
                    .and_then(|id| {
                        crate::runtime::with_runtime(|runtime| {
                            runtime.identity.session_by_short(id)
                        })
                        .flatten()
                    })
                    .ok_or(IdentityError::NotFound)?;
                if session.user != actor && !administrator {
                    return Err(IdentityError::AccessDenied);
                }
                self.output.write_number(b"session:", session.id.short());
                self.output
                    .write_number(b"capabilities: ", session.capabilities);
                Ok(())
            }
            OperationId::SessionLock => crate::runtime::with_runtime(|runtime| {
                runtime.identity.lock_session(self.current_session, actor)
            })
            .unwrap_or(Err(IdentityError::InvalidState)),
            OperationId::SessionEnd => {
                let session = target_number
                    .and_then(|id| {
                        crate::runtime::with_runtime(|runtime| {
                            runtime.identity.session_by_short(id)
                        })
                        .flatten()
                    })
                    .ok_or(IdentityError::NotFound)?;
                if session.id == self.current_session {
                    let _ = self.persist_desktop_layout();
                }
                crate::runtime::with_runtime(|runtime| {
                    runtime.identity.end_session(session.id, actor)
                })
                .unwrap_or(Err(IdentityError::InvalidState))
            }
            OperationId::PersonalSpaceRead => {
                let user = target_number
                    .and_then(|id| {
                        crate::runtime::with_runtime(|runtime| runtime.identity.user_by_short(id))
                            .flatten()
                    })
                    .map(|user| user.id)
                    .unwrap_or(actor);
                if user != actor && !administrator {
                    Err(IdentityError::AccessDenied)
                } else {
                    let ownership = crate::runtime::with_runtime(|runtime| {
                        runtime.identity.personal_space(user)
                    })
                    .flatten()
                    .ok_or(IdentityError::NotFound)?;
                    self.output
                        .write_number(b"owner user:", ownership.owner.short());
                    self.output
                        .write_number(b"personal-space:", ownership.space.short());
                    Ok(())
                }
            }
            OperationId::AiProfileRead => {
                let profile =
                    crate::runtime::with_runtime(|runtime| runtime.identity.ai_profile(actor))
                        .flatten()
                        .ok_or(IdentityError::NotFound)?;
                self.output.write_segments(&[
                    b"provider policy: ",
                    ai_policy_text(profile.provider_policy),
                ]);
                Ok(())
            }
            OperationId::AiProfileUpdate => {
                let policy = match argument(b"provider-policy") {
                    Some(b"prefer-local") => AiProviderPolicy::PreferLocal,
                    Some(b"ask-before-remote") => AiProviderPolicy::AskBeforeRemote,
                    Some(b"remote-allowed") => AiProviderPolicy::RemoteAllowed,
                    Some(b"local-only") => AiProviderPolicy::LocalOnly,
                    _ => return Err(IdentityError::InvalidInput),
                };
                crate::runtime::with_runtime(|runtime| {
                    runtime.identity.update_ai_profile(actor, actor, policy)
                })
                .unwrap_or(Err(IdentityError::InvalidState))?;
                if crate::runtime::persist_identity_state() {
                    Ok(())
                } else {
                    Err(IdentityError::InvalidState)
                }
            }
            OperationId::VoiceProfileRead => {
                let profile =
                    crate::runtime::with_runtime(|runtime| runtime.identity.voice_profile(actor))
                        .flatten()
                        .ok_or(IdentityError::NotFound)?;
                self.output.write_segments(&[
                    b"voice: ",
                    if profile.enabled {
                        b"enabled"
                    } else {
                        b"disabled"
                    },
                ]);
                Ok(())
            }
            OperationId::VoiceProfileUpdate => {
                let enabled = argument(b"enabled") == Some(b"true");
                let activation = if enabled {
                    VoiceActivation::PushToTalk
                } else {
                    VoiceActivation::Disabled
                };
                crate::runtime::with_runtime(|runtime| {
                    runtime
                        .identity
                        .update_voice_profile(actor, actor, enabled, activation)
                })
                .unwrap_or(Err(IdentityError::InvalidState))?;
                if crate::runtime::persist_identity_state() {
                    Ok(())
                } else {
                    Err(IdentityError::InvalidState)
                }
            }
            OperationId::SettingsRead | OperationId::ProfileRead => {
                let profile =
                    crate::runtime::with_runtime(|runtime| runtime.identity.user_profile(actor))
                        .flatten()
                        .ok_or(IdentityError::NotFound)?;
                self.output
                    .write_segments(&[b"language: ", profile.language.as_bytes()]);
                self.output
                    .write_segments(&[b"region: ", profile.region.as_bytes()]);
                self.output
                    .write_segments(&[b"appearance.theme: ", profile.theme.as_bytes()]);
                Ok(())
            }
            OperationId::SettingsUpdate | OperationId::ProfileUpdate => {
                if argument(b"name") != Some(b"appearance.theme") {
                    Err(IdentityError::InvalidInput)
                } else {
                    let value = argument(b"value").ok_or(IdentityError::InvalidInput)?;
                    crate::runtime::with_runtime(|runtime| {
                        runtime.identity.update_user_theme(actor, actor, value)
                    })
                    .unwrap_or(Err(IdentityError::InvalidState))?;
                    if crate::runtime::persist_identity_state() {
                        Ok(())
                    } else {
                        Err(IdentityError::InvalidState)
                    }
                }
            }
            _ => Err(IdentityError::InvalidInput),
        })();
        match result {
            Ok(()) => self.output.write_line(b"Success"),
            Err(error) => return self.identity_error(error),
        }
        true
    }

    // ------------------------=
    // FUNC: identity_error
    // DESC: Renders a typed identity failure without exposing credential details.
    // ------------------=
    fn identity_error(&mut self, error: crate::runtime::identity::IdentityError) -> bool {
        self.output
            .write_segments(&[b"Identity operation failed: ", identity_error_text(error)]);
        true
    }

    // ------------------------=
    // FUNC: execute_runtime_command_for_operation
    // DESC: Bridges registered query operations to existing service adapters without reparsing output.
    // ------------------=
    fn execute_runtime_command_for_operation(
        &mut self,
        operation: crate::runtime::iop::OperationId,
    ) -> bool {
        use crate::runtime::iop::OperationId;
        let command: &[u8] = match operation {
            OperationId::ServiceList => b"service list",
            OperationId::RuntimeContexts => b"runtime contexts",
            OperationId::CapabilityList => b"capability list",
            OperationId::EventSubscriptions => b"event subscriptions",
            OperationId::IntentResolve => b"ai status",
            OperationId::ModelList => b"model list",
            OperationId::AgentList => b"agent list",
            OperationId::VoiceStatus => b"voice status",
            _ => {
                self.output
                    .write_line(b"Typed operation is registered but has no Console renderer yet.");
                return true;
            }
        };
        self.execute_runtime_command(command) || self.execute_ai_command(command)
    }

    // ------------------------=
    // FUNC: render_object_query
    // DESC: Executes a bounded native Object.Query and binds session references to its ObjectSet.
    // ------------------=
    fn render_object_query(&mut self, node: &crate::runtime::console_language::OperationNode<'_>) {
        let mut kind = None;
        let mut space = None;
        for argument in node.arguments.iter().flatten() {
            if argument.name == b"type" {
                kind = console_object_type(argument.value);
            }
            if argument.name == b"space" {
                space = console_space(argument.value);
            }
        }
        let mut ids = [[0u8; 16]; 16];
        let mut count = 0usize;
        for index in 0..16 {
            match crate::storage::object_query_nth(kind, space, index) {
                Ok(Some(result)) => {
                    ids[count] = result.object.id.0;
                    count += 1;
                    self.output.write_segments(&[
                        b"[",
                        &number_pair(count),
                        b"] ",
                        &result.display_name[..result.display_name_length as usize],
                    ]);
                    self.output.write_id(b"    obj:", result.object.id);
                }
                _ => break,
            }
        }
        self.language_session.bind_references(&ids[..count]);
        self.output.write_number(b"ObjectSet count: ", count as u64);
    }

    // ------------------------=
    // FUNC: render_typed_object_list
    // DESC: Renders first-class Project or Collection objects as a typed set.
    // ------------------=
    fn render_typed_object_list(&mut self, kind: crate::storage::object::ObjectType, label: &[u8]) {
        let mut count = 0;
        for index in 0..16 {
            match crate::storage::object_query_nth(Some(kind), None, index) {
                Ok(Some(result)) => {
                    self.output.write_segments(&[
                        b"- ",
                        &result.display_name[..result.display_name_length as usize],
                    ]);
                    self.output.write_id(b"  obj:", result.object.id);
                    count += 1;
                }
                _ => break,
            }
        }
        self.output.write_segments(&[label, b" result"]);
        self.output.write_number(b"Count: ", count);
    }

    // ------------------------=
    // FUNC: create_organization
    // DESC: Creates a first-class Project or Collection through native object storage.
    // ------------------=
    fn create_organization(
        &mut self,
        node: &crate::runtime::console_language::OperationNode<'_>,
        kind: crate::storage::object::ObjectType,
    ) {
        let Some(name) = node
            .arguments
            .iter()
            .flatten()
            .find(|argument| argument.name == b"name")
            .map(|argument| argument.value)
        else {
            self.output
                .write_line(b"MissingArgument: name is required.");
            return;
        };
        match crate::storage::organization_create(kind, name) {
            Ok(id) => {
                self.output
                    .write_segments(&[object_type_text(kind), b" created as a native object."]);
                self.output.write_id(b"Object ID: obj:", id);
            }
            Err(error) => self.storage_error(error),
        }
    }

    // ------------------------=
    // FUNC: inspect_organization
    // DESC: Resolves and renders Project or Collection metadata without treating its name as authority.
    // ------------------=
    fn inspect_organization(
        &mut self,
        node: &crate::runtime::console_language::OperationNode<'_>,
        kind: crate::storage::object::ObjectType,
    ) {
        let Some(target) = node.target else {
            self.output
                .write_line(b"MissingArgument: target is required.");
            return;
        };
        match crate::storage::organization_inspect(kind, target.value) {
            Ok(metadata) => {
                self.output
                    .write_segments(&[b"Type: ", object_type_text(metadata.kind)]);
                self.output.write_id(b"Object ID: obj:", metadata.object.id);
                self.output
                    .write_line(b"Name is a resolved reference, not authority.");
            }
            Err(error) => self.storage_error(error),
        }
    }

    // ------------------------=
    // FUNC: dispatch
    // DESC: Implements the dispatch operation.
    // ------------------=
    fn dispatch(&mut self, operation: SystemOperation) {
        match operation {
            SystemOperation::Help => self.help(),
            SystemOperation::ClearConsole => {
                self.output.clear();
                self.output.write_line(if self.mode == ConsoleMode::Repair {
                    b"InfinityOS Repair Environment"
                } else {
                    b"Infinity Console"
                });
                crate::output_text(b"[operation] console.clear\n");
            }
            SystemOperation::DeviceList => self.device_list(),
            SystemOperation::SystemStatus => self.system_status(),
            SystemOperation::SystemInfo => self.system_info(),
            SystemOperation::SystemGenerationInspect => self.system_generation(),
            SystemOperation::SystemBootStatus => self.system_boot(),
            SystemOperation::MemoryStatus => self.memory_status(),
            SystemOperation::ExitConsole => {
                if self.mode == ConsoleMode::Desktop
                    && self.desktop_app == DesktopAppKind::CommandWindow
                {
                    self.enter_desktop();
                    return;
                }
                if self.system.live_profile {
                    self.show_startup();
                } else {
                    self.output
                        .write_line(b"Installed system console cannot exit to Recovery Node.");
                }
            }
            SystemOperation::Greeting => self.output.write_line(b"Hello there!"),
        }
    }

    // ------------------------=
    // FUNC: help
    // DESC: Implements the help operation.
    // ------------------=
    fn help(&mut self) {
        crate::output_text(b"[operation] help\n");
        self.output.write_line(b"Commands: help | clear | exit");
        self.output.write_line(b"device list");
        self.output
            .write_line(b"system status | system info | system generation | system boot");
        self.output.write_line(b"memory status | storage usage");
        self.output
            .write_line(b"object create|inspect|history|write|restore|remove|collect");
        self.output.write_line(b"namespace list|inspect|move|link");
        self.output
            .write_line(b"service list|inspect|restart | runtime contexts|inspect");
        self.output
            .write_line(b"capability list|inspect | event subscriptions|trace | system resources");
        self.output
            .write_line(b"ai status|trace|infer | model list|inspect | provider list | voice status | agent list");
    }

    // ------------------------=
    // FUNC: device_list
    // DESC: Implements the device list operation.
    // ------------------=
    fn device_list(&mut self) {
        crate::output_text(b"[operation] device.list\n");
        self.output
            .write_line(b"DEVICE       TYPE       DRIVER                 STATE");
        for device in self.system.devices {
            self.output.write_segments(&[
                device.name.as_bytes(),
                b"  ",
                device.kind.name(),
                b"  ",
                device.driver.as_bytes(),
                b"  ",
                device.state.name(),
            ]);
        }
    }

    // ------------------------=
    // FUNC: system_status
    // DESC: Implements the system status operation.
    // ------------------=
    fn system_status(&mut self) {
        crate::output_text(b"[operation] system.status\n");
        self.output.write_line(b"InfinityOS System Status");
        self.output
            .write_segments(&[b"Architecture: ", self.system.architecture]);
        self.output.write_line(b"Kernel: online");
        self.output.write_line(b"Drivers: online");
        self.output.write_line(b"Intent Runtime: online");
        self.output.write_segments(&[
            b"Ready devices: ",
            &[b'0' + self.system.ready_device_count() as u8],
            b"/3",
        ]);
    }

    // ------------------------=
    // FUNC: system_generation
    // DESC: Implements the system generation operation.
    // ------------------=
    fn system_generation(&mut self) {
        crate::output_text(b"[operation] System.GenerationInspect\n");
        if !self.system.live_profile {
            self.output.write_line(b"Active generation: 1");
            self.output.write_line(b"State: ACTIVE");
            self.output.write_line(b"Build: 1");
            self.output
                .write_segments(&[b"Architecture: ", self.system.architecture]);
            self.output
                .write_line(b"Integrity: valid (verified by Infinity EFI)");
        } else {
            self.output.write_line(
                b"No installed generation is active in recovery/development boot mode.",
            );
        }
    }

    // ------------------------=
    // FUNC: system_boot
    // DESC: Implements the system boot operation.
    // ------------------=
    fn system_boot(&mut self) {
        crate::output_text(b"[operation] System.BootStatus\n");
        if !self.system.live_profile {
            self.output.write_line(b"Boot mode: Installed");
            self.output.write_line(b"Bootloader: Infinity EFI");
            self.output.write_line(b"Boot device: storage0");
            self.output.write_line(b"System Space: online");
            self.output.write_line(b"Generation: 1");
        } else {
            self.output.write_line(if cfg!(target_arch = "x86") {
                b"Boot mode: Development"
            } else {
                b"Boot mode: Recovery"
            });
            self.output.write_line(if cfg!(target_arch = "x86") {
                b"Bootloader: Infinity BIOS bootstrap"
            } else {
                b"Bootloader: Infinity EFI (installation media)"
            });
        }
    }

    // ------------------------=
    // FUNC: system_info
    // DESC: Implements the system info operation.
    // ------------------=
    fn system_info(&mut self) {
        crate::output_text(b"[operation] system.info\n");
        self.output.write_line(b"InfinityOS");
        self.output
            .write_segments(&[b"Architecture: ", self.system.architecture]);
        self.output
            .write_segments(&[b"Boot mode: ", self.system.boot_mode]);
        self.output.write_line(b"Kernel state: online");
        self.output.write_line(b"Build: development");
    }

    // ------------------------=
    // FUNC: memory_status
    // DESC: Implements the memory status operation.
    // ------------------=
    fn memory_status(&mut self) {
        crate::output_text(b"[operation] memory.status\n");
        self.output.write_line(b"Known boot memory state");
        self.output
            .write_number(b"Firmware map bytes: ", self.system.memory_map_size);
        self.output
            .write_number(b"Descriptor bytes: ", self.system.memory_descriptor_size);
        self.output
            .write_number(b"Descriptors: ", self.system.memory_descriptor_count());
        self.output.write_line(b"Kernel heap: not configured");
    }

    // ------------------------=
    // FUNC: execute_runtime_command
    // DESC: Implements the execute runtime command operation.
    // ------------------=
    fn execute_runtime_command(&mut self, command: &[u8]) -> bool {
        if command == b"task" || command == b"help task" {
            self.output
                .write_line(b"task - monitor and control live execution contexts");
            self.output
                .write_line(b"list | inspect HANDLE | launch APP | end HANDLE | relaunch HANDLE");
            self.output.write_line(
                b"pause HANDLE | resume HANDLE | throttle HANDLE cpu=N memory=N queue=N io=N",
            );
            return true;
        }
        if command == b"task list" || command == b"tasks" {
            crate::output_text(b"[operation] Task.List\n");
            crate::runtime::with_runtime(|runtime| {
                self.output
                    .write_line(b"HANDLE  NAME                 STATE      CPU  MEMORY / LIMIT");
                for index in 0..runtime.task_manager.task_count(&runtime.execution) {
                    if let Some(task) = runtime.task_manager.task_nth(&runtime.execution, index) {
                        self.output
                            .write_number(b"Task handle: ", task.handle.0 as u64);
                        self.output.write_segments(&[
                            b"  ",
                            task_name(task.service_identity, task.image_identity),
                            b"  ",
                            context_state_text(task.state),
                        ]);
                        self.output
                            .write_number(b"  CPU ticks: ", task.usage.cpu_ticks);
                        self.output
                            .write_number(b"  Memory limit: ", task.budget.memory_limit);
                    }
                }
            });
            return true;
        }
        if let Some(value) = command.strip_prefix(b"task inspect ") {
            crate::output_text(b"[operation] Task.Inspect\n");
            let Some(handle) = parse_u32(value)
                .map(|value| crate::runtime::execution::ContextHandle(value as u16))
            else {
                self.output.write_line(b"Usage: task inspect <handle>");
                return true;
            };
            crate::runtime::with_runtime(|runtime| {
                match runtime.task_manager.inspect(&runtime.execution, handle) {
                    Ok(task) => {
                        self.output.write_segments(&[
                            b"Name: ",
                            task_name(task.service_identity, task.image_identity),
                        ]);
                        self.output
                            .write_segments(&[b"State: ", context_state_text(task.state)]);
                        self.output
                            .write_number(b"CPU ticks: ", task.usage.cpu_ticks);
                        self.output
                            .write_number(b"Memory bytes: ", task.usage.memory_bytes);
                        self.output
                            .write_number(b"Memory limit: ", task.budget.memory_limit);
                        self.output
                            .write_number(b"CPU weight: ", task.budget.cpu_weight as u64);
                        self.output
                            .write_number(b"Queue limit: ", task.budget.message_queue_limit as u64);
                        self.output
                            .write_number(b"I/O priority: ", task.budget.io_priority as u64);
                    }
                    Err(_) => self.output.write_line(b"Unknown task handle."),
                }
            });
            return true;
        }
        if let Some(name) = command.strip_prefix(b"task launch ") {
            crate::output_text(b"[operation] Task.Launch\n");
            let Some(image) = task_image_id(name) else {
                self.output.write_line(b"Unknown installed application.");
                return true;
            };
            if image == crate::runtime::task_manager::IMAGE_FILE_NAVIGATOR {
                if let Some(handle) = self.open_file_navigator_window(b"/home/default") {
                    self.output.write_number(b"Launched task: ", handle as u64);
                } else {
                    self.output.write_line(b"Application launch failed.");
                }
                return true;
            }
            let launched = crate::runtime::with_runtime(|runtime| {
                runtime.task_manager.launch(&mut runtime.execution, image)
            });
            match launched {
                Some(Ok(handle)) => {
                    self.output
                        .write_number(b"Launched task: ", handle.0 as u64);
                    match image {
                        crate::runtime::task_manager::IMAGE_TEXT_EDITOR => self.open_text_editor(),
                        crate::runtime::task_manager::IMAGE_TASK_MANAGER => {
                            self.open_task_manager()
                        }
                        _ => {}
                    }
                }
                _ => self.output.write_line(b"Application launch failed."),
            }
            return true;
        }
        for (prefix, action) in [
            (b"task end ".as_slice(), 0u8),
            (b"task relaunch ".as_slice(), 1),
            (b"task pause ".as_slice(), 2),
            (b"task resume ".as_slice(), 3),
        ] {
            if let Some(value) = command.strip_prefix(prefix) {
                let Some(handle) = parse_u32(value)
                    .map(|value| crate::runtime::execution::ContextHandle(value as u16))
                else {
                    self.output
                        .write_line(b"A numeric task handle is required.");
                    return true;
                };
                let task = crate::runtime::with_runtime(|runtime| {
                    runtime
                        .task_manager
                        .inspect(&runtime.execution, handle)
                        .ok()
                })
                .flatten();
                let result = crate::runtime::with_runtime(|runtime| match action {
                    0 => runtime.task_manager.end(&mut runtime.execution, handle),
                    1 => runtime
                        .task_manager
                        .relaunch(&mut runtime.execution, handle),
                    2 => runtime.task_manager.pause(&mut runtime.execution, handle),
                    _ => runtime.task_manager.resume(&mut runtime.execution, handle),
                });
                self.output.write_line(if matches!(result, Some(Ok(()))) {
                    b"Task state updated."
                } else {
                    b"Task operation rejected; inspect state or system-task protection."
                });
                if action == 0 && matches!(result, Some(Ok(()))) {
                    if let Some(task) = task {
                        self.close_task_surface(task);
                    }
                } else if action == 1 && matches!(result, Some(Ok(()))) {
                    if let Some(task) = task {
                        self.reopen_task_surface(task);
                    }
                }
                return true;
            }
        }
        if let Some(arguments) = command.strip_prefix(b"task throttle ") {
            let Some(handle_text) = command_word(arguments, 0) else {
                self.output
                    .write_line(b"Usage: task throttle <handle> cpu=N memory=N queue=N io=N");
                return true;
            };
            let Some(handle_value) = parse_u32(handle_text) else {
                self.output
                    .write_line(b"A numeric task handle is required.");
                return true;
            };
            let handle = crate::runtime::execution::ContextHandle(handle_value as u16);
            let result =
                crate::runtime::with_runtime(|runtime| {
                    let task = runtime.task_manager.inspect(&runtime.execution, handle)?;
                    let mut budget = task.budget;
                    let mut priority = task.priority;
                    for index in 1..7 {
                        let Some(setting) = command_word(arguments, index) else {
                            break;
                        };
                        let Some((name, value)) = split_once(setting, b'=') else {
                            return Err(
                                crate::runtime::task_manager::TaskManagerError::InvalidBudget,
                            );
                        };
                        match name {
                            b"cpu" => budget.cpu_weight = parse_u32(value)
                                .ok_or(
                                    crate::runtime::task_manager::TaskManagerError::InvalidBudget,
                                )?
                                .min(u16::MAX as u32)
                                as u16,
                            b"memory" => {
                                budget.memory_limit = parse_u32(value).ok_or(
                                    crate::runtime::task_manager::TaskManagerError::InvalidBudget,
                                )? as u64
                            }
                            b"queue" => budget.message_queue_limit = parse_u32(value)
                                .ok_or(
                                    crate::runtime::task_manager::TaskManagerError::InvalidBudget,
                                )?
                                .min(u16::MAX as u32)
                                as u16,
                            b"io" => budget.io_priority = parse_u32(value)
                                .ok_or(
                                    crate::runtime::task_manager::TaskManagerError::InvalidBudget,
                                )?
                                .min(u8::MAX as u32)
                                as u8,
                            b"priority" => priority = match value {
                                b"background" => {
                                    crate::runtime::execution::PriorityClass::Background
                                }
                                b"interactive" => crate::runtime::execution::PriorityClass::Normal,
                                b"system" => crate::runtime::execution::PriorityClass::System,
                                b"critical" => crate::runtime::execution::PriorityClass::Critical,
                                _ => return Err(
                                    crate::runtime::task_manager::TaskManagerError::InvalidBudget,
                                ),
                            },
                            _ => {
                                return Err(
                                    crate::runtime::task_manager::TaskManagerError::InvalidBudget,
                                )
                            }
                        }
                    }
                    runtime
                        .task_manager
                        .throttle(&mut runtime.execution, handle, budget, priority)
                });
            self.output.write_line(if matches!(result, Some(Ok(()))) {
                b"Task resource limits and scheduling priority updated."
            } else {
                b"Throttle rejected."
            });
            return true;
        }
        if command == b"service list" || command == b"show me running services" {
            crate::output_text(b"[operation] Service.List\n");
            crate::runtime::with_runtime(|runtime| {
                self.output.write_line(b"SERVICE       VERSION  STATE");
                for index in 0..runtime.services.count() {
                    if let Some(service) = runtime.services.nth(index) {
                        self.output.write_segments(&[
                            service_name(service.manifest.service_id),
                            b"  v1  ",
                            service_state_text(service.state),
                        ]);
                    }
                }
            });
            return true;
        }
        if let Some(name) = command.strip_prefix(b"service inspect ") {
            crate::output_text(b"[operation] Service.Inspect\n");
            let Some(id) = service_id(name) else {
                self.output.write_line(b"Unknown service.");
                return true;
            };
            crate::runtime::with_runtime(|runtime| {
                if let Some(service) = runtime.services.inspect(id) {
                    self.output
                        .write_segments(&[b"Service: ", service_name(id)]);
                    self.output.write_number(
                        b"Manifest version: ",
                        service.manifest.format_version as u64,
                    );
                    self.output
                        .write_number(b"Service version: ", service.manifest.version as u64);
                    self.output
                        .write_segments(&[b"State: ", service_state_text(service.state)]);
                    self.output
                        .write_number(b"Dependencies: ", service.manifest.dependency_count as u64);
                    self.output
                        .write_number(b"Operations: ", service.manifest.operation_count as u64);
                    self.output
                        .write_number(b"Memory budget: ", service.manifest.resources.memory_limit);
                }
            });
            return true;
        }
        if let Some(name) = command.strip_prefix(b"service restart ") {
            crate::output_text(b"[operation] Service.Restart\n");
            let Some(id) = service_id(name) else {
                self.output.write_line(b"Unknown service.");
                return true;
            };
            crate::runtime::with_runtime(|runtime| {
                match runtime.services.restart(id, &mut runtime.execution, 0) {
                    Ok(()) => {
                        runtime.start_all(0);
                        self.output
                            .write_line(b"Restart requested; readiness re-established.")
                    }
                    Err(_) => self.output.write_line(b"Service restart failed."),
                }
            });
            return true;
        }
        if command == b"runtime contexts" {
            crate::output_text(b"[operation] Runtime.Contexts\n");
            crate::runtime::with_runtime(|runtime| {
                self.output
                    .write_number(b"Execution contexts: ", runtime.execution.count() as u64);
                for index in 0..runtime.execution.count() {
                    if let Some(context) = runtime.execution.nth(index) {
                        self.output.write_segments(&[
                            b"ctx ",
                            service_name(context.service_identity),
                            b" isolated-region",
                        ]);
                    }
                }
            });
            return true;
        }
        if let Some(value) = command.strip_prefix(b"runtime inspect ") {
            crate::output_text(b"[operation] Runtime.Inspect\n");
            if let Some(handle) = parse_u32(value) {
                crate::runtime::with_runtime(|runtime| {
                    if let Some(context) = runtime
                        .execution
                        .get(crate::runtime::execution::ContextHandle(handle as u16))
                    {
                        self.output
                            .write_number(b"Context handle: ", context.handle.0 as u64);
                        self.output.write_segments(&[
                            b"Service identity: ",
                            service_name(context.service_identity),
                        ]);
                        self.output
                            .write_number(b"Memory limit: ", context.budget.memory_limit);
                        self.output
                            .write_number(b"CPU ticks: ", context.usage.cpu_ticks);
                        self.output
                            .write_number(b"Queue depth: ", context.usage.queued_messages as u64);
                    } else {
                        self.output.write_line(b"Unknown execution context.")
                    }
                });
            }
            return true;
        }
        if command == b"capability list" {
            crate::output_text(b"[operation] Capability.List\n");
            crate::runtime::with_runtime(|runtime| {
                self.output.write_number(
                    b"Explicit capabilities: ",
                    runtime.capabilities.count() as u64,
                );
                self.output
                    .write_line(b"No ambient administrator authority.");
            });
            return true;
        }
        if let Some(value) = command.strip_prefix(b"capability inspect ") {
            crate::output_text(b"[operation] Capability.Inspect\n");
            if let Some(id) = parse_u32(value) {
                crate::runtime::with_runtime(|runtime| {
                    if let Some(capability) = runtime.capabilities.get(id as u64) {
                        self.output.write_number(b"Capability ID: ", capability.id);
                        self.output.write_number(b"Target: ", capability.target);
                        self.output
                            .write_number(b"Rights mask: ", capability.rights as u64);
                        self.output.write_line(if capability.revoked {
                            b"State: revoked"
                        } else {
                            b"State: active"
                        });
                    } else {
                        self.output.write_line(b"Unknown capability.")
                    }
                });
            }
            return true;
        }
        if command == b"event subscriptions" {
            crate::output_text(b"[operation] Event.Subscriptions\n");
            crate::runtime::with_runtime(|runtime| {
                self.output.write_number(
                    b"Active leased subscriptions: ",
                    runtime.events.subscription_count() as u64,
                )
            });
            return true;
        }
        if command == b"event trace" {
            crate::output_text(b"[operation] Event.Trace\n");
            crate::runtime::with_runtime(|runtime| {
                self.output.write_number(
                    b"Durable security records: ",
                    runtime.events.record_count() as u64,
                );
                self.output
                    .write_line(b"Trace IDs preserve correlation and causation.");
            });
            return true;
        }
        if command == b"system resources" {
            crate::output_text(b"[operation] Runtime.Resources\n");
            crate::runtime::with_runtime(|runtime| {
                let mut memory = 0u64;
                let mut ticks = 0u64;
                let mut queues = 0u64;
                for index in 0..runtime.execution.count() {
                    if let Some(context) = runtime.execution.nth(index) {
                        memory += context.usage.memory_bytes;
                        ticks += context.usage.cpu_ticks;
                        queues += context.usage.queued_messages as u64;
                    }
                }
                self.output
                    .write_number(b"Contexts: ", runtime.execution.count() as u64);
                self.output
                    .write_number(b"Accounted memory bytes: ", memory);
                self.output.write_number(b"CPU dispatch ticks: ", ticks);
                self.output.write_number(b"Queued messages: ", queues);
                self.output.write_line(b"Memory pressure: normal");
            });
            return true;
        }
        false
    }

    // ------------------------=
    // FUNC: execute_ai_command
    // DESC: Projects typed AI, model, provider, voice, and agent state into the human console.
    // ------------------=
    fn execute_ai_command(&mut self, command: &[u8]) -> bool {
        use crate::runtime::ai::{model::LOCAL_INTENT_MODEL_ID, types::DataLocality};
        if command == b"ai status" {
            crate::output_text(b"[operation] AI.Status\n");
            crate::runtime::ai::with_ai_runtime(|ai| {
                self.output.write_line(if ai.initialized() {
                    b"Infinity AI Service: ready"
                } else {
                    b"Infinity AI Service: degraded"
                });
                self.output.write_line(b"Default policy: local-only");
                self.output.write_line(b"CPU backend: TESTED");
                self.output.write_line(b"GPU/NPU backends: UNSUPPORTED");
                self.output
                    .write_number(b"Inference queue depth: ", ai.queue_depth() as u64);
                self.output
                    .write_number(b"Completed inference: ", ai.inference_count());
                self.output
                    .write_number(b"Inference failures: ", ai.inference_failures());
            });
            return true;
        }
        if command == b"model list" {
            crate::output_text(b"[operation] Model.List\n");
            crate::runtime::ai::with_ai_runtime(|ai| {
                self.output
                    .write_number(b"Registered native models: ", ai.models.count() as u64);
                for index in 0..ai.models.count() {
                    if let Some(model) = ai.models.nth(index) {
                        self.output.write_segments(&[
                            b"local-intent-v1  provider=local  state=",
                            if model.install_state
                                == crate::runtime::ai::types::InstallState::Loaded
                            {
                                b"loaded"
                            } else {
                                b"unloaded"
                            },
                        ]);
                    }
                }
            });
            return true;
        }
        if command == b"model inspect local-intent-v1" {
            crate::output_text(b"[operation] Model.Inspect\n");
            crate::runtime::ai::with_ai_runtime(|ai| {
                if let Some(model) = ai.models.inspect(LOCAL_INTENT_MODEL_ID) {
                    self.output
                        .write_number(b"Model identity: ", model.id as u64);
                    self.output.write_number(b"Version: ", model.version as u64);
                    self.output
                        .write_line(b"Adapter: Infinity native quantized linear");
                    self.output
                        .write_line(b"Capabilities: intent-resolution, classification");
                    self.output
                        .write_line(b"Install class: SYSTEM OPTIONAL (bundled development model)");
                    self.output
                        .write_number(b"Model checksum: ", model.checksum as u64);
                    self.output.write_line(if model.object_ref == [0; 16] {
                        b"Object binding: immutable live-media payload"
                    } else {
                        b"Object binding: native System object"
                    });
                    self.output.write_line(b"Processing: LOCAL");
                }
            });
            return true;
        }
        if command == b"provider list" {
            crate::output_text(b"[operation] AI.ProviderList\n");
            self.output
                .write_line(b"local-cpu  ready  LOCAL  offline-capable");
            self.output
                .write_line(b"remote       SCAFFOLDED  disabled by policy");
            return true;
        }
        if command == b"voice status" {
            crate::output_text(b"[operation] Voice.Status\n");
            crate::runtime::ai::with_ai_runtime(|ai| {
                self.output.write_segments(&[
                    b"Voice session: ",
                    if ai.voice.state() == crate::runtime::ai::voice::VoiceState::Idle {
                        b"idle"
                    } else {
                        b"active"
                    },
                ]);
            });
            self.output
                .write_line(b"Push-to-talk capability boundary: TESTED");
            self.output
                .write_line(b"Audio capture and offline speech model: UNSUPPORTED");
            return true;
        }
        if command == b"agent list" {
            crate::output_text(b"[operation] Agent.List\n");
            crate::runtime::ai::with_ai_runtime(|ai| {
                self.output
                    .write_number(b"Defined constrained agents: ", ai.agents.count() as u64);
                self.output
                    .write_number(b"Queued agent tasks: ", ai.agents.queue_depth() as u64);
            });
            self.output
                .write_line(b"Agents receive no inherited user authority.");
            return true;
        }
        if command == b"ai trace" {
            crate::output_text(b"[operation] AI.Trace\n");
            crate::runtime::ai::with_ai_runtime(|ai| {
                if let Some(result) = ai.last_result() {
                    self.output
                        .write_number(b"Correlation: ", result.correlation_id);
                    self.output.write_number(b"Model: ", result.model as u64);
                    self.output
                        .write_line(if result.locality == DataLocality::Local {
                            b"Processing: LOCAL; data left device: no"
                        } else {
                            b"Processing: REMOTE; data left device: yes"
                        });
                    self.output
                        .write_number(b"Confidence: ", result.confidence_milli as u64);
                    self.output
                        .write_number(b"Context classes: ", result.context_classes as u64);
                    if let Some(plan) = ai.last_plan() {
                        if let Some(operation) = plan.operations[0] {
                            self.output.write_number(
                                b"Result operation ID: ",
                                operation.operation.machine_id() as u64,
                            );
                        }
                    }
                } else {
                    self.output.write_line(b"No completed inference to trace.");
                }
            });
            return true;
        }
        if let Some(input) = command.strip_prefix(b"ai infer ") {
            crate::output_text(b"[operation] Intent.Resolve\n");
            let correlation = command_correlation(input);
            let plan = crate::runtime::with_runtime(|runtime| {
                runtime.resolve_console_intent(input, 0, correlation)
            })
            .and_then(Result::ok);
            if let Some(plan) = plan {
                self.output
                    .write_line(b"Typed IntentPlan proposed; not executed.");
                self.output
                    .write_number(b"Confidence: ", plan.confidence_milli as u64);
                self.output
                    .write_number(b"Correlation: ", plan.correlation_id);
                self.output
                    .write_line(b"Processing: LOCAL; data left device: no");
                self.output.write_line(if plan.ambiguous {
                    b"Plan state: needs clarification"
                } else {
                    b"Plan state: validated"
                });
            } else {
                self.output
                    .write_line(b"Local model could not form a confident typed plan.");
            }
            return true;
        }
        false
    }

    // ------------------------=
    // FUNC: execute_storage_command
    // DESC: Implements the execute storage command operation.
    // ------------------=
    fn execute_storage_command(&mut self, command: &[u8]) -> bool {
        use crate::storage;
        if command == b"storage usage" {
            crate::output_text(b"[operation] storage.usage\n");
            match storage::storage_usage() {
                Ok((used, total, generation)) => {
                    self.output
                        .write_line(b"Infinity Pool Object Allocation (4 KiB blocks)");
                    self.output.write_number(b"Total blocks: ", total as u64);
                    self.output.write_number(b"Allocated blocks: ", used as u64);
                    self.output
                        .write_number(b"Available blocks: ", total.saturating_sub(used) as u64);
                    if let Ok(by_space) = storage::storage_usage_by_space() {
                        self.output.write_number(b"System: ", by_space[0] as u64);
                        self.output.write_number(b"Personal: ", by_space[1] as u64);
                        self.output
                            .write_number(b"Applications: ", by_space[2] as u64);
                        self.output.write_number(b"Recovery: ", by_space[3] as u64);
                    }
                    self.output
                        .write_number(b"Committed generation: ", generation);
                }
                Err(e) => self.storage_error(e),
            }
            return true;
        }
        if command == b"namespace list"
            || command.starts_with(b"namespace list ")
            || command == b"show my files"
        {
            crate::output_text(b"[operation] namespace.list\n");
            let prefix = if command == b"show my files" {
                b"/home/default".as_slice()
            } else {
                command.strip_prefix(b"namespace list ").unwrap_or(b"/")
            };
            let mut path = [0u8; 95];
            let mut shown = 0;
            for index in 0..32 {
                if let Some((n, _)) = storage::namespace_entry(index, &mut path) {
                    if path[..n].starts_with(prefix) {
                        self.output.write_line(&path[..n]);
                        shown += 1;
                    }
                }
            }
            if shown == 0 {
                self.output.write_line(b"No namespace entries found.");
            }
            return true;
        }
        if let Some(path) = command.strip_prefix(b"namespace inspect ") {
            crate::output_text(b"[operation] namespace.inspect\n");
            let mut data = [0u8; 4096];
            match storage::object_read_path(path, None, &mut data) {
                Ok((id, size)) => {
                    self.output.write_id(b"Object ID: obj:", id);
                    self.output.write_number(b"Content bytes: ", size as u64);
                }
                Err(e) => self.storage_error(e),
            }
            return true;
        }
        if let Some(args) = command.strip_prefix(b"namespace move ") {
            if let Some((from, to)) = split_once(args, b' ') {
                crate::output_text(b"[operation] namespace.move\n");
                match storage::namespace_move(from, to) {
                    Ok(()) => self
                        .output
                        .write_line(b"Namespace relationship moved; object identity unchanged."),
                    Err(e) => self.storage_error(e),
                }
            } else {
                self.output
                    .write_line(b"Usage: namespace move <old-path> <new-path>");
            }
            return true;
        }
        if let Some(args) = command.strip_prefix(b"namespace link ") {
            if let Some((from, to)) = split_once(args, b' ') {
                crate::output_text(b"[operation] namespace.attach\n");
                match storage::namespace_link(from, to) {
                    Ok(id) => self
                        .output
                        .write_id(b"Attached direct reference to obj:", id),
                    Err(e) => self.storage_error(e),
                }
            } else {
                self.output
                    .write_line(b"Usage: namespace link <existing-path> <new-path>");
            }
            return true;
        }
        let create_name = command
            .strip_prefix(b"object create ")
            .or_else(|| command.strip_prefix(b"create a note called "));
        if let Some(name) = create_name {
            let mut path = [0u8; 95];
            let base = b"/home/default/documents/";
            if base.len() + name.len() > path.len() {
                self.output.write_line(b"Object name is too long.");
                return true;
            }
            path[..base.len()].copy_from_slice(base);
            path[base.len()..base.len() + name.len()].copy_from_slice(name);
            crate::output_text(b"[operation] object.create\n");
            match storage::object_create_note_at(name, b"", &path[..base.len() + name.len()]) {
                Ok(id) => {
                    self.output.write_id(b"Created obj:", id);
                    self.output
                        .write_segments(&[b"Attached: ", &path[..base.len() + name.len()]]);
                }
                Err(e) => self.storage_error(e),
            }
            return true;
        }
        if command.starts_with(b"move ") && command.ends_with(b" into archive") {
            let name = &command[5..command.len() - 13];
            let mut from = [0u8; 95];
            let mut to = [0u8; 95];
            let fb = b"/home/default/documents/";
            let tb = b"/home/default/archive/";
            if fb.len() + name.len() <= 95 && tb.len() + name.len() <= 95 {
                from[..fb.len()].copy_from_slice(fb);
                from[fb.len()..fb.len() + name.len()].copy_from_slice(name);
                to[..tb.len()].copy_from_slice(tb);
                to[tb.len()..tb.len() + name.len()].copy_from_slice(name);
                match storage::namespace_move(
                    &from[..fb.len() + name.len()],
                    &to[..tb.len() + name.len()],
                ) {
                    Ok(()) => self
                        .output
                        .write_line(b"Moved to archive; object identity unchanged."),
                    Err(e) => self.storage_error(e),
                }
            } else {
                self.output.write_line(b"Object name is too long.");
            }
            return true;
        }
        if let Some(args) = command.strip_prefix(b"object write ") {
            if let Some((path, content)) = split_once(args, b' ') {
                crate::output_text(b"[operation] object.update\n");
                match storage::object_write_path(path, content) {
                    Ok(v) => self.output.write_number(b"Committed version: ", v as u64),
                    Err(e) => self.storage_error(e),
                }
            } else {
                self.output
                    .write_line(b"Usage: object write <path> <content>");
            }
            return true;
        }
        if let Some(path) = command.strip_prefix(b"object inspect ") {
            crate::output_text(b"[operation] object.inspect\n");
            match storage::object_inspect_path(path) {
                Ok((m, refs)) => {
                    self.output.write_id(b"Object ID: obj:", m.object.id);
                    self.output
                        .write_segments(&[b"Type: ", object_type_text(m.kind)]);
                    self.output
                        .write_segments(&[b"Space: ", space_text(m.space)]);
                    self.output
                        .write_number(b"Current version: ", m.current_version as u64);
                    self.output.write_number(b"Size: ", m.logical_size as u64);
                    self.output.write_number(b"Created generation: ", m.created);
                    self.output
                        .write_number(b"Modified generation: ", m.modified);
                    self.output.write_number(b"Namespace refs: ", refs as u64);
                    self.output.write_line(b"Integrity: valid");
                }
                Err(e) => self.storage_error(e),
            }
            return true;
        }
        if let Some(path) = command.strip_prefix(b"object read ") {
            crate::output_text(b"[operation] object.read\n");
            let mut data = [0u8; 4096];
            match storage::object_read_path(path, None, &mut data) {
                Ok((id, n)) => {
                    self.output.write_id(b"Object ID: obj:", id);
                    self.output
                        .write_segments(&[b"Content: ", &data[..n.min(80)]]);
                }
                Err(e) => self.storage_error(e),
            }
            return true;
        }
        if let Some(path) = command.strip_prefix(b"object history ") {
            crate::output_text(b"[operation] object.history\n");
            match storage::object_history(path) {
                Ok((id, count, current)) => {
                    self.output.write_id(b"Object ID: obj:", id);
                    self.output
                        .write_number(b"Versions retained: ", count as u64);
                    self.output
                        .write_number(b"Current version: ", current as u64);
                    for i in 0..count {
                        if let Ok((version, is_current)) = storage::object_history_version(path, i)
                        {
                            self.output.write_number(
                                if is_current {
                                    b"Version (current): "
                                } else {
                                    b"Version: "
                                },
                                version as u64,
                            );
                        }
                    }
                }
                Err(e) => self.storage_error(e),
            }
            return true;
        }
        if let Some(name) = command.strip_prefix(b"show the previous versions of ") {
            let base = b"/home/default/documents/";
            let mut path = [0u8; 95];
            if base.len() + name.len() <= 95 {
                path[..base.len()].copy_from_slice(base);
                path[base.len()..base.len() + name.len()].copy_from_slice(name);
                match storage::object_history(&path[..base.len() + name.len()]) {
                    Ok((id, count, current)) => {
                        self.output.write_id(b"Object ID: obj:", id);
                        self.output
                            .write_number(b"Versions retained: ", count as u64);
                        self.output
                            .write_number(b"Current version: ", current as u64);
                    }
                    Err(e) => self.storage_error(e),
                }
            } else {
                self.output.write_line(b"Object name is too long.");
            }
            return true;
        }
        if let Some(args) = command.strip_prefix(b"object restore ") {
            if let Some((path, version)) = split_once(args, b' ') {
                if let Some(v) = parse_u32(version) {
                    crate::output_text(b"[operation] object.restore\n");
                    match storage::object_restore(path, v) {
                        Ok(new) => self
                            .output
                            .write_number(b"Restored as new version: ", new as u64),
                        Err(e) => self.storage_error(e),
                    }
                } else {
                    self.output.write_line(b"Invalid version number.");
                }
            } else {
                self.output
                    .write_line(b"Usage: object restore <path> <version>");
            }
            return true;
        }
        if let Some(path) = command.strip_prefix(b"object remove ") {
            crate::output_text(b"[operation] object.remove\n");
            match storage::object_remove_path(path) {
                Ok(id) => self.output.write_id(
                    b"Namespace detached / object tombstoned if unreferenced: obj:",
                    id,
                ),
                Err(e) => self.storage_error(e),
            }
            return true;
        }
        if command == b"object collect" {
            crate::output_text(b"[operation] object.collect\n");
            match storage::object_collect() {
                Ok(blocks) => self
                    .output
                    .write_number(b"Reclaimed 4 KiB blocks: ", blocks as u64),
                Err(e) => self.storage_error(e),
            }
            return true;
        }
        false
    }

    // ------------------------=
    // FUNC: storage_error
    // DESC: Implements the storage error operation.
    // ------------------=
    fn storage_error(&mut self, error: crate::storage::object::ObjectError) {
        self.output
            .write_segments(&[b"Storage error: ", object_error_text(error)]);
    }
}

impl ConsoleOutput {
    // ------------------------=
    // FUNC: write_id
    // DESC: Writes or updates write id data.
    // ------------------=
    fn write_id(&mut self, label: &[u8], id: crate::storage::object::ObjectId) {
        let hex = b"0123456789ABCDEF";
        let mut text = [0u8; 32];
        for i in 0..16 {
            text[i * 2] = hex[(id.0[i] >> 4) as usize];
            text[i * 2 + 1] = hex[(id.0[i] & 15) as usize];
        }
        self.write_segments(&[label, &text]);
    }
}

// ------------------------=
// FUNC: split_once
// DESC: Implements the split once operation.
// ------------------=
fn split_once(bytes: &[u8], separator: u8) -> Option<(&[u8], &[u8])> {
    let at = bytes.iter().position(|x| *x == separator)?;
    if at == 0 || at + 1 >= bytes.len() {
        None
    } else {
        Some((&bytes[..at], &bytes[at + 1..]))
    }
}

// ------------------------=
// FUNC: command_word
// DESC: Returns one whitespace-delimited command word without allocating or evaluating text.
// ------------------=
fn command_word(command: &[u8], index: usize) -> Option<&[u8]> {
    command
        .split(|byte| byte.is_ascii_whitespace())
        .filter(|word| !word.is_empty())
        .nth(index)
}

// ------------------------=
// FUNC: node_argument
// DESC: Returns one validated typed argument from a parsed node operation without reparsing command text.
// ------------------=
fn node_argument<'a>(
    node: &'a crate::runtime::console_language::OperationNode<'a>,
    name: &[u8],
) -> Option<&'a [u8]> {
    node.arguments
        .iter()
        .flatten()
        .find(|argument| argument.name == name)
        .map(|argument| argument.value)
}

// ------------------------=
// FUNC: parse_u64_decimal
// DESC: Parses a bounded unsigned decimal Console reference without allocation.
// ------------------=
fn parse_u64_decimal(value: &[u8]) -> Option<u64> {
    if value.is_empty() || value.iter().any(|byte| !byte.is_ascii_digit()) {
        return None;
    }
    value.iter().try_fold(0u64, |number, byte| {
        number.checked_mul(10)?.checked_add((byte - b'0') as u64)
    })
}

// ------------------------=
// FUNC: parse_u32_decimal
// DESC: Parses a bounded unsigned 32-bit Console value without allocation.
// ------------------=
fn parse_u32_decimal(value: &[u8]) -> Option<u32> {
    u32::try_from(parse_u64_decimal(value)?).ok()
}

// ------------------------=
// FUNC: parse_node_id
// DESC: Decodes a complete 128-bit hexadecimal NodeId so mutable list position never becomes authority identity.
// ------------------=
fn parse_node_id(value: &[u8]) -> Option<crate::runtime::node::types::NodeId> {
    if value.len() != 64 {
        return None;
    }
    let mut bytes = [0u8; 32];
    for index in 0..32 {
        let high = hex_nibble(value[index * 2])?;
        let low = hex_nibble(value[index * 2 + 1])?;
        bytes[index] = high << 4 | low;
    }
    Some(crate::runtime::node::types::NodeId(bytes))
}

// ------------------------=
// FUNC: hex_nibble
// DESC: Converts one ASCII hexadecimal digit into its four-bit value.
// ------------------=
fn hex_nibble(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

// ------------------------=
// FUNC: node_policy_category
// DESC: Resolves the stable twelve-category node policy index from its deterministic human name.
// ------------------=
fn node_policy_category(value: &[u8]) -> Option<usize> {
    [
        b"object".as_slice(),
        b"namespace",
        b"compute",
        b"ai",
        b"service",
        b"event",
        b"storage",
        b"clipboard",
        b"device",
        b"diagnostics",
        b"mesh",
        b"administrative",
    ]
    .iter()
    .position(|candidate| *candidate == value)
}

// ------------------------=
// FUNC: is_node_console_mutation
// DESC: Identifies the explicitly implemented Milestone 9 mutations that may execute after deterministic parsing.
// ------------------=
fn is_node_console_mutation(operation: crate::runtime::iop::OperationId) -> bool {
    use crate::runtime::iop::OperationId;
    matches!(
        operation,
        OperationId::NodePairBegin
            | OperationId::NodePairConfirm
            | OperationId::NodePairCancel
            | OperationId::NodeTrustUpdate
            | OperationId::NodeRevokeTrust
            | OperationId::NodeBlock
            | OperationId::NodeUnblock
            | OperationId::NodeSessionClose
            | OperationId::NodeCapabilityRevoke
            | OperationId::NodePolicyUpdate
            | OperationId::MeshPolicyUpdate
            | OperationId::MeshMemberAdd
            | OperationId::MeshMemberRemove
            | OperationId::NodeJoin
            | OperationId::NodeLeave
    )
}

// ------------------------=
// FUNC: node_event_for_operation
// DESC: Maps a committed node mutation to its bounded post-commit IEF notification type.
// ------------------=
fn node_event_for_operation(operation: crate::runtime::iop::OperationId) -> u32 {
    use crate::runtime::iop::OperationId;
    match operation {
        OperationId::NodePairBegin => crate::runtime::EVENT_NODE_PAIRING_REQUESTED,
        OperationId::NodePairConfirm => crate::runtime::EVENT_NODE_PAIRED,
        OperationId::NodePairCancel => crate::runtime::EVENT_NODE_PAIRING_REJECTED,
        OperationId::NodeTrustUpdate => crate::runtime::EVENT_NODE_TRUST_CHANGED,
        OperationId::NodeRevokeTrust => crate::runtime::EVENT_NODE_TRUST_REVOKED,
        OperationId::NodeBlock => crate::runtime::EVENT_NODE_BLOCKED,
        OperationId::NodeUnblock => crate::runtime::EVENT_NODE_UNBLOCKED,
        OperationId::NodeSessionClose => crate::runtime::EVENT_NODE_SESSION_CLOSED,
        OperationId::NodeJoin | OperationId::MeshMemberAdd => crate::runtime::EVENT_NODE_JOINED,
        OperationId::NodeLeave | OperationId::MeshMemberRemove => crate::runtime::EVENT_NODE_LEFT,
        OperationId::NodePolicyUpdate | OperationId::MeshPolicyUpdate => {
            crate::runtime::EVENT_NODE_TRUST_CHANGED
        }
        _ => 0,
    }
}

// ------------------------=
// FUNC: command_tail
// DESC: Returns the trimmed command remainder after a fixed number of words.
// ------------------=
fn command_tail(command: &[u8], words: usize) -> Option<&[u8]> {
    let mut at = 0usize;
    let mut consumed = 0usize;
    while at < command.len() && consumed < words {
        while at < command.len() && command[at].is_ascii_whitespace() {
            at += 1;
        }
        if at == command.len() {
            return None;
        }
        while at < command.len() && !command[at].is_ascii_whitespace() {
            at += 1;
        }
        consumed += 1;
    }
    while at < command.len() && command[at].is_ascii_whitespace() {
        at += 1;
    }
    (at < command.len()).then_some(&command[at..])
}

// ------------------------=
// FUNC: unquote_command_tail
// DESC: Removes one matching quote pair from a validated declarative alias template.
// ------------------=
fn unquote_command_tail(value: &[u8]) -> &[u8] {
    if value.len() >= 2
        && matches!(
            (value.first(), value.last()),
            (Some(b'"'), Some(b'"')) | (Some(b'\''), Some(b'\''))
        )
    {
        &value[1..value.len() - 1]
    } else {
        value
    }
}

// ------------------------=
// FUNC: immediate_namespace_child
// DESC: Distinguishes direct children from deeper bounded tree projections.
// ------------------=
fn immediate_namespace_child(parent: &[u8], candidate: &[u8]) -> bool {
    if !candidate.starts_with(parent) || candidate.len() <= parent.len() {
        return false;
    }
    let suffix = if parent == b"/" {
        &candidate[1..]
    } else if candidate.get(parent.len()) == Some(&b'/') {
        &candidate[parent.len() + 1..]
    } else {
        return false;
    };
    !suffix.is_empty() && !suffix.contains(&b'/')
}

// ------------------------=
// FUNC: navigator_child_nth
// DESC: Resolves one direct child from the active namespace without exposing descendants as rows.
// ------------------=
fn navigator_child_nth(
    parent: &[u8],
    requested: usize,
) -> Option<crate::storage::object::NamespaceListResult> {
    let requested = requested
        .checked_sub(crate::runtime::object_navigation::FILE_NAVIGATOR_NAVIGATION_ENTRY_COUNT)?;
    let descending = crate::runtime::with_runtime(|runtime| {
        runtime
            .file_navigator
            .map(|navigator| navigator.sort_descending)
    })
    .flatten()
    .unwrap_or(false);
    crate::storage::namespace_child_nth_sorted(parent, requested, descending)
        .ok()
        .flatten()
}

// ------------------------=
// FUNC: navigator_child_count
// DESC: Counts direct children for File Navigator status and bounded keyboard selection.
// ------------------=
fn navigator_child_count(parent: &[u8]) -> usize {
    crate::storage::namespace_child_count(parent)
        .unwrap_or(0)
        .saturating_add(crate::runtime::object_navigation::FILE_NAVIGATOR_NAVIGATION_ENTRY_COUNT)
}

// ------------------------=
// FUNC: write_decimal
// DESC: Writes a positive decimal suffix into a bounded File Navigator name buffer.
// ------------------=
fn write_decimal(destination: &mut [u8], mut value: usize) -> usize {
    let mut reversed = [0u8; 20];
    let mut length = 0usize;
    loop {
        reversed[length] = b'0' + (value % 10) as u8;
        length += 1;
        value /= 10;
        if value == 0 || length == reversed.len() {
            break;
        }
    }
    let written = length.min(destination.len());
    for index in 0..written {
        destination[index] = reversed[length - index - 1];
    }
    written
}

// ------------------------=
// FUNC: ascii_contains_case_insensitive
// DESC: Performs bounded capability-safe ASCII search matching without locale ambiguity.
// ------------------=
fn ascii_contains_case_insensitive(value: &[u8], query: &[u8]) -> bool {
    query.is_empty()
        || value.windows(query.len()).any(|window| {
            window
                .iter()
                .zip(query.iter())
                .all(|(left, right)| left.eq_ignore_ascii_case(right))
        })
}

// ------------------------=
// FUNC: profile_id_text
// DESC: Formats a stable Shell Profile ID for transparent alias diagnostics.
// ------------------=
fn profile_id_text(value: u32) -> [u8; 10] {
    let mut output = *b"0000000000";
    let mut remaining = value;
    for index in (0..output.len()).rev() {
        output[index] = b'0' + (remaining % 10) as u8;
        remaining /= 10;
    }
    output
}

// ------------------------=
// FUNC: home_location_path
// DESC: Maps File Navigator sidebar rows to configured native Namespace references.
// ------------------=
fn home_location_path(location: usize) -> &'static [u8] {
    [
        b"/home/default".as_slice(),
        b"/home/default",
        b"/home/default/documents",
        b"/home/default/downloads",
        b"/home/default/pictures",
        b"/home/default/media",
        b"/home/default/media",
        b"/home/default/projects",
        b"/trash",
    ][location.min(8)]
}

// ------------------------=
// FUNC: number_pair
// DESC: Formats a bounded Console result index as two decimal bytes.
// ------------------=
fn number_pair(value: usize) -> [u8; 2] {
    [b'0' + ((value / 10) % 10) as u8, b'0' + (value % 10) as u8]
}
// ------------------------=
// FUNC: parse_u32
// DESC: Implements the parse u32 operation.
// ------------------=
fn parse_u32(bytes: &[u8]) -> Option<u32> {
    let mut value = 0u32;
    if bytes.is_empty() {
        return None;
    }
    for b in bytes {
        if !b.is_ascii_digit() {
            return None;
        }
        value = value.checked_mul(10)?.checked_add((b - b'0') as u32)?;
    }
    Some(value)
}

// ------------------------=
// FUNC: command_correlation
// DESC: Derives a stable non-secret correlation identifier for one console request.
// ------------------=
fn command_correlation(bytes: &[u8]) -> u64 {
    let mut value = 0xcbf2_9ce4_8422_2325u64;
    for byte in bytes {
        value = (value ^ *byte as u64).wrapping_mul(0x100_0000_01b3);
    }
    value
}

// ------------------------=
// FUNC: system_operation_from_iop
// DESC: Converts the local model's closed IOP proposal into an executable console operation.
// ------------------=
fn system_operation_from_iop(
    operation: crate::runtime::iop::OperationId,
) -> Option<SystemOperation> {
    use crate::runtime::iop::OperationId;
    match operation {
        OperationId::SystemStatus => Some(SystemOperation::SystemStatus),
        OperationId::DeviceList => Some(SystemOperation::DeviceList),
        OperationId::SystemInfo => Some(SystemOperation::SystemInfo),
        OperationId::SystemBootStatus => Some(SystemOperation::SystemBootStatus),
        OperationId::MemoryStatus => Some(SystemOperation::MemoryStatus),
        _ => None,
    }
}
// ------------------------=
// FUNC: service_id
// DESC: Implements the service id operation.
// ------------------=
fn service_id(name: &[u8]) -> Option<u32> {
    use crate::runtime::service::*;
    match name {
        b"runtime" => Some(SERVICE_RUNTIME),
        b"device" => Some(SERVICE_DEVICE),
        b"storage" => Some(SERVICE_STORAGE),
        b"object" => Some(SERVICE_OBJECT),
        b"namespace" => Some(SERVICE_NAMESPACE),
        b"event" => Some(SERVICE_EVENT),
        b"console" => Some(SERVICE_CONSOLE),
        b"installer" => Some(SERVICE_INSTALLER),
        b"local-ml" => Some(SERVICE_LOCAL_ML),
        b"infinity-ai" | b"ai" => Some(SERVICE_AI),
        b"voice" => Some(SERVICE_VOICE),
        b"agent" => Some(SERVICE_AGENT),
        b"organization" => Some(SERVICE_ORGANIZATION),
        _ => None,
    }
}
// ------------------------=
// FUNC: service_name
// DESC: Implements the service name operation.
// ------------------=
fn service_name(id: u32) -> &'static [u8] {
    use crate::runtime::service::*;
    match id {
        SERVICE_RUNTIME => b"runtime",
        SERVICE_DEVICE => b"device",
        SERVICE_STORAGE => b"storage",
        SERVICE_OBJECT => b"object",
        SERVICE_NAMESPACE => b"namespace",
        SERVICE_EVENT => b"event",
        SERVICE_CONSOLE => b"console",
        SERVICE_INSTALLER => b"installer",
        SERVICE_LOCAL_ML => b"local-ml",
        SERVICE_AI => b"infinity-ai",
        SERVICE_VOICE => b"voice",
        SERVICE_AGENT => b"agent",
        SERVICE_ORGANIZATION => b"organization",
        _ => b"unknown",
    }
}
// ------------------------=
// FUNC: service_state_text
// DESC: Implements the service state text operation.
// ------------------=
fn service_state_text(state: crate::runtime::service::ServiceState) -> &'static [u8] {
    use crate::runtime::service::ServiceState::*;
    match state {
        Defined => b"Defined",
        Starting => b"Starting",
        Ready => b"Ready",
        Degraded => b"Degraded",
        Stopping => b"Stopping",
        Stopped => b"Stopped",
        Failed => b"Failed",
        Restarting => b"Restarting",
    }
}

// ------------------------=
// FUNC: task_image_id
// DESC: Resolves an installed application name to its typed executable image identity.
// ------------------=
fn task_image_id(name: &[u8]) -> Option<u32> {
    use crate::runtime::task_manager::*;
    match name {
        b"file-navigator" | b"files" => Some(IMAGE_FILE_NAVIGATOR),
        b"text-editor" | b"editor" => Some(IMAGE_TEXT_EDITOR),
        b"command-window" | b"console" => Some(IMAGE_COMMAND_WINDOW),
        b"task-manager" | b"tasks" => Some(IMAGE_TASK_MANAGER),
        _ => None,
    }
}

// ------------------------=
// FUNC: task_name
// DESC: Maps typed service or application identities to a stable presentation label.
// ------------------=
fn task_name(service: u32, image: u32) -> &'static [u8] {
    use crate::runtime::task_manager::*;
    if service != APPLICATION_SERVICE_ID {
        return service_name(service);
    }
    match image {
        IMAGE_FILE_NAVIGATOR => b"File Navigator",
        IMAGE_TEXT_EDITOR => b"Text Editor",
        IMAGE_COMMAND_WINDOW => b"Command Window",
        IMAGE_TASK_MANAGER => b"Task Manager",
        _ => b"Unknown Application",
    }
}

// ------------------------=
// FUNC: context_state_text
// DESC: Projects typed execution state into Task Manager and console presentation.
// ------------------=
fn context_state_text(state: crate::runtime::execution::ContextState) -> &'static [u8] {
    use crate::runtime::execution::ContextState::*;
    match state {
        Defined => b"Defined",
        Runnable => b"Runnable",
        Running => b"Running",
        Waiting => b"Paused",
        Stopped => b"Stopped",
        Failed => b"Failed",
    }
}
// ------------------------=
// FUNC: object_type_text
// DESC: Implements the object type text operation.
// ------------------=
fn object_type_text(kind: crate::storage::object::ObjectType) -> &'static [u8] {
    use crate::storage::object::ObjectType::*;
    match kind {
        Blob => b"Blob",
        Text => b"Text",
        NamespaceNode => b"Namespace Node",
        SystemComponent => b"System Component",
        ApplicationData => b"Application Data",
        Metadata => b"Metadata",
        Collection => b"Collection",
        Project => b"Project",
        Model => b"Model",
        IdentityData => b"Identity Data",
        DeviceData => b"Device Data",
    }
}

// ------------------------=
// FUNC: console_object_type
// DESC: Resolves canonical human object-type names to stable native storage types.
// ------------------=
fn console_object_type(value: &[u8]) -> Option<crate::storage::object::ObjectType> {
    use crate::storage::object::ObjectType;
    match value {
        b"content" | b"blob" => Some(ObjectType::Blob),
        b"document" | b"note" | b"text" => Some(ObjectType::Text),
        b"project" => Some(ObjectType::Project),
        b"collection" => Some(ObjectType::Collection),
        b"model" => Some(ObjectType::Model),
        b"application-data" => Some(ObjectType::ApplicationData),
        b"system-object" => Some(ObjectType::SystemComponent),
        b"identity-data" => Some(ObjectType::IdentityData),
        b"device-data" => Some(ObjectType::DeviceData),
        _ => None,
    }
}

// ------------------------=
// FUNC: console_space
// DESC: Resolves a Console space predicate to the native Infinity Pool space identity.
// ------------------=
fn console_space(value: &[u8]) -> Option<crate::storage::object::Space> {
    use crate::storage::object::Space;
    match value {
        b"system" => Some(Space::System),
        b"personal" => Some(Space::Personal),
        b"applications" => Some(Space::Applications),
        b"recovery" => Some(Space::Recovery),
        _ => None,
    }
}

// ------------------------=
// FUNC: language_error_text
// DESC: Maps typed Console language failures to discoverable human guidance.
// ------------------=
fn language_error_text(
    error: crate::runtime::console_language::ConsoleLanguageError,
) -> &'static [u8] {
    use crate::runtime::console_language::ConsoleLanguageError::*;
    match error {
        Empty => b"No operation was entered.",
        UnknownDomain => b"UnknownDomain: type help to list domains.",
        UnknownOperation => b"UnknownOperation: that domain does not provide this action.",
        MissingArgument => b"MissingArgument: this operation needs more information.",
        InvalidArgument => b"InvalidArgument: use readable key=value arguments.",
        InvalidArgumentType => b"InvalidArgumentType: the value does not match its schema.",
        DuplicateArgument => b"DuplicateArgument: provide each key once.",
        UnterminatedQuote => b"InvalidArgument: quoted value was not closed.",
        TooManyStages => b"Operation plan exceeds the bounded stage limit.",
        TooManyArguments => b"Operation exceeds the bounded argument limit.",
        TypeMismatch => b"TypeMismatch: operation result and input types are incompatible.",
        InvalidAssignment => {
            b"InvalidAssignment: variable names use letters, digits, or underscore."
        }
        ReferenceNotFound => b"ReferenceNotFound: contextual reference is not in this session.",
    }
}

// ------------------------=
// FUNC: value_type_text
// DESC: Returns the human projection of a typed operation result identity.
// ------------------=
fn value_type_text(value: crate::runtime::console_language::ValueType) -> &'static [u8] {
    use crate::runtime::console_language::ValueType::*;
    match value {
        Unit => b"Unit",
        SystemStatus => b"SystemStatus",
        DeviceSet => b"DeviceSet",
        StorageStatus => b"StorageStatus",
        Object => b"Object",
        ObjectSet => b"ObjectSet",
        NamespaceResult => b"NamespaceResult",
        Project => b"Project",
        ProjectSet => b"ProjectSet",
        Collection => b"Collection",
        CollectionSet => b"CollectionSet",
        ServiceSet => b"ServiceSet",
        RuntimeContextSet => b"RuntimeContextSet",
        CapabilitySet => b"CapabilitySet",
        EventSet => b"EventSet",
        ModelSet => b"ModelSet",
        AgentSet => b"AgentSet",
        VoiceStatus => b"VoiceStatus",
        OperationPlan => b"OperationPlan",
        User => b"User",
        UserSet => b"UserSet",
        Machine => b"Machine",
        CredentialSet => b"CredentialSet",
        Session => b"Session",
        SessionSet => b"SessionSet",
        Profile => b"Profile",
        PersonalSpace => b"PersonalSpace",
        Settings => b"Settings",
        SkinSet => b"SkinSet",
        UiTree => b"UiTree",
        WindowSet => b"WindowSet",
        ClipboardData => b"ClipboardData",
        NetworkStatus => b"NetworkStatus",
        NetworkInterfaceSet => b"NetworkInterfaceSet",
        NetworkAddressSet => b"NetworkAddressSet",
        NetworkRouteSet => b"NetworkRouteSet",
        NetworkConnectionSet => b"NetworkConnectionSet",
        NetworkPolicySet => b"NetworkPolicySet",
        NetworkProfileSet => b"NetworkProfileSet",
        NetworkDiagnostics => b"NetworkDiagnostics",
        ServiceDiscoverySet => b"ServiceDiscoverySet",
        NodeSet => b"NodeSet",
        NodeSessionSet => b"NodeSessionSet",
        NodePolicy => b"NodePolicy",
        NodeAuditSet => b"NodeAuditSet",
        MeshDomainSet => b"MeshDomainSet",
    }
}

// ------------------------=
// FUNC: side_effect_text
// DESC: Returns the human policy projection of an operation side-effect class.
// ------------------=
fn side_effect_text(value: crate::runtime::console_language::SideEffectClass) -> &'static [u8] {
    use crate::runtime::console_language::SideEffectClass::*;
    match value {
        Query => b"QUERY",
        ReversibleChange => b"REVERSIBLE_CHANGE",
        DestructiveChange => b"DESTRUCTIVE_CHANGE",
        SecurityChange => b"SECURITY_CHANGE",
        ExternalEffect => b"EXTERNAL_EFFECT",
    }
}

// ------------------------=
// FUNC: parse_reference_number
// DESC: Parses the compact numeric projection from a typed human reference.
// ------------------=
fn parse_reference_number(value: &[u8]) -> Option<u64> {
    let digits = value
        .iter()
        .position(|byte| *byte == b':')
        .map(|index| &value[index + 1..])
        .unwrap_or(value);
    if digits.is_empty() || !digits.iter().all(u8::is_ascii_digit) {
        return None;
    }
    Some(digits.iter().fold(0u64, |number, digit| {
        number
            .saturating_mul(10)
            .saturating_add((digit - b'0') as u64)
    }))
}

// ------------------------=
// FUNC: ai_policy_text
// DESC: Projects a typed AI provider policy without changing its machine identity.
// ------------------=
fn ai_policy_text(policy: crate::runtime::identity::AiProviderPolicy) -> &'static [u8] {
    use crate::runtime::identity::AiProviderPolicy::*;
    match policy {
        LocalOnly => b"local-only",
        PreferLocal => b"prefer-local",
        AskBeforeRemote => b"ask-before-remote",
        RemoteAllowed => b"remote-allowed",
    }
}

// ------------------------=
// FUNC: identity_error_text
// DESC: Projects typed identity errors without revealing credential existence or verifier data.
// ------------------=
fn identity_error_text(error: crate::runtime::identity::IdentityError) -> &'static [u8] {
    use crate::runtime::identity::IdentityError::*;
    match error {
        InvalidInput => b"INVALID INPUT",
        Full => b"RESOURCE LIMIT",
        NotFound => b"NOT FOUND",
        Conflict => b"CONFLICT",
        AccessDenied => b"ACCESS DENIED",
        InvalidCredential => b"AUTHENTICATION FAILED",
        RateLimited => b"TRY AGAIN LATER",
        InvalidState => b"INVALID STATE",
        CorruptState => b"CORRUPT STATE",
        PersonalSpaceUnavailable => b"PERSONAL SPACE UNAVAILABLE",
    }
}
// ------------------------=
// FUNC: space_text
// DESC: Implements the space text operation.
// ------------------=
fn space_text(space: crate::storage::object::Space) -> &'static [u8] {
    use crate::storage::object::Space::*;
    match space {
        System => b"System",
        Personal => b"Personal",
        Applications => b"Applications",
        Recovery => b"Recovery",
    }
}
// ------------------------=
// FUNC: object_error_text
// DESC: Implements the object error text operation.
// ------------------=
fn object_error_text(error: crate::storage::object::ObjectError) -> &'static [u8] {
    use crate::storage::object::ObjectError::*;
    match error {
        NotFound => b"ObjectNotFound",
        NamespaceNotFound => b"NamespaceNotFound",
        NameConflict => b"NameConflict",
        InvalidObject => b"InvalidObject",
        InvalidPath => b"InvalidPath",
        InsufficientCapacity => b"InsufficientCapacity",
        CorruptMetadata => b"CorruptMetadata",
        CorruptContent => b"CorruptContent",
        UnsupportedFormat => b"UnsupportedFormat",
        TransactionFailed => b"TransactionFailed",
        ChecksumMismatch => b"ChecksumMismatch",
        InvalidVersion => b"InvalidVersion",
        SpaceUnavailable => b"SpaceUnavailable",
        Busy => b"TransactionBusy",
        Unauthorized => b"Unauthorized",
    }
}

static mut RUNTIME: Option<ConsoleRuntime> = None;

// ------------------------=
// FUNC: initialize
// DESC: Initializes initialize state.
// ------------------=
pub fn initialize(system: SystemSnapshot) {
    let mut runtime = ConsoleRuntime::new(system);
    crate::output_text(b"[console] subsystem online\n");
    crate::output_text(b"[intent] runtime online\n");
    if runtime.system.live_profile {
        runtime.show_startup();
    } else {
        crate::output_text(b"InfinityOS Native Boot\nBoot source: installed system\n");
        let onboarding = crate::runtime::with_runtime(|system| system.identity.onboarding_state())
            .unwrap_or(crate::runtime::identity::OnboardingState::Required);
        if onboarding == crate::runtime::identity::OnboardingState::Complete {
            runtime.mode = ConsoleMode::Authentication;
            runtime.system_focus = 1;
            runtime.system_step = 0;
            runtime.reset_input();
            crate::output_text(b"[authentication] login ready\n");
        } else {
            runtime.enter_onboarding();
        }
    }
    runtime.redraw();
    unsafe {
        RUNTIME = Some(runtime);
    }
}

// ------------------------=
// FUNC: input
// DESC: Implements the input operation.
// ------------------=
pub fn input(key: ConsoleKey) {
    unsafe {
        let slot = &raw mut RUNTIME;
        if let Some(runtime) = (*slot).as_mut() {
            runtime.input(key);
        }
    }
}

// ------------------------=
// FUNC: pointer
// DESC: Implements the pointer operation.
// ------------------=
pub fn pointer(delta_x: i16, delta_y: i16, left_button: bool) {
    pointer_buttons(delta_x, delta_y, u8::from(left_button));
}

// ------------------------=
// FUNC: pointer_buttons
// DESC: Routes complete relative pointer button state, including native secondary clicks.
// ------------------=
pub fn pointer_buttons(delta_x: i16, delta_y: i16, buttons: u8) {
    unsafe {
        let slot = &raw mut RUNTIME;
        if let Some(runtime) = (*slot).as_mut() {
            runtime.pointer(delta_x, delta_y, buttons);
        }
    }
}

// ------------------------=
// FUNC: pointer_scroll
// DESC: Routes vertical wheel input to a scrollable native surface and reports whether it consumed the gesture.
// ------------------=
pub fn pointer_scroll(vertical: i8) -> bool {
    unsafe {
        let slot = &raw mut RUNTIME;
        (*slot)
            .as_mut()
            .map(|runtime| runtime.pointer_scroll(vertical))
            .unwrap_or(false)
    }
}

// ------------------------=
// FUNC: pointer_absolute
// DESC: Handles pointer absolute input or state transitions.
// ------------------=
pub fn pointer_absolute(x: i32, y: i32, left_button: bool) {
    pointer_absolute_buttons(x, y, u8::from(left_button));
}

// ------------------------=
// FUNC: pointer_absolute_buttons
// DESC: Routes complete absolute pointer button state, including native secondary clicks.
// ------------------=
pub fn pointer_absolute_buttons(x: i32, y: i32, buttons: u8) {
    unsafe {
        let slot = &raw mut RUNTIME;
        if let Some(runtime) = (*slot).as_mut() {
            runtime.pointer_absolute(x, y, buttons);
        }
    }
}

// ------------------------=
// FUNC: ui_animation_tick
// DESC: Advances launcher transitions and eased Settings scrolling on the display refresh clock.
// ------------------=
pub fn ui_animation_tick() -> bool {
    unsafe {
        let slot = &raw mut RUNTIME;
        let Some(runtime) = (*slot).as_mut() else {
            return false;
        };
        let motion_frame = runtime.continuous_motion_frames.take_for_tick();
        let mut frame_changed = motion_frame;
        if runtime.mode == ConsoleMode::Settings {
            let layout = SystemLayout::new(
                runtime.system.framebuffer_width,
                runtime.system.framebuffer_height,
            );
            let maximum = layout
                .settings_window_geometry_for_section(runtime.settings_window, runtime.system_focus)
                .maximum_scroll;
            runtime.settings_scroll_target = runtime.settings_scroll_target.min(maximum);
            let next = crate::ui::system_layout::eased_scroll_offset(
                runtime.settings_window.scroll_offset,
                runtime.settings_scroll_target,
                maximum,
            );
            if next != runtime.settings_window.scroll_offset {
                runtime.settings_window.scroll_offset = next;
                frame_changed = true;
            }
            if frame_changed {
                runtime.presenting_fast_motion_frame = motion_frame;
                runtime.redraw();
                runtime.presenting_fast_motion_frame = false;
            }
            return frame_changed;
        }
        if runtime.mode != ConsoleMode::AppLauncher {
            if frame_changed {
                runtime.presenting_fast_motion_frame = motion_frame;
                runtime.redraw();
                runtime.presenting_fast_motion_frame = false;
            }
            return frame_changed;
        }
        let layout = SystemLayout::new(
            runtime.system.framebuffer_width,
            runtime.system.framebuffer_height,
        );
        let visible = launcher_visible_count(&runtime.command[..runtime.command_length]);
        let maximum = layout.app_launcher_scroll_geometry(visible).maximum_scroll;
        let now = crate::ui::performance::monotonic_ns();
        let elapsed_ms = now
            .zip(runtime.launcher_tick_ns)
            .map(|(now, previous)| now.saturating_sub(previous) / 1_000_000)
            .unwrap_or(16)
            .max(1)
            .min(160) as usize;
        runtime.launcher_tick_ns = now;
        let tick = crate::ui::app_launcher::launcher_animation_advance(maximum, elapsed_ms);
        if tick.closed {
            runtime.enter_desktop();
            runtime.redraw();
            return true;
        }
        if tick.changed {
            frame_changed = true;
        }
        if frame_changed {
            runtime.presenting_fast_motion_frame = true;
            runtime.redraw();
            runtime.presenting_fast_motion_frame = false;
        }
        frame_changed
    }
}

// ------------------------=
// FUNC: clock_tick
// DESC: Refreshes the installed desktop clock from the live firmware wall clock.
// ------------------=
pub fn clock_tick() {
    unsafe {
        let slot = &raw mut RUNTIME;
        if let Some(runtime) = (*slot).as_mut() {
            if !matches!(
                runtime.mode,
                ConsoleMode::Onboarding
                    | ConsoleMode::Authentication
                    | ConsoleMode::Locked
                    | ConsoleMode::Desktop
                    | ConsoleMode::AppLauncher
                    | ConsoleMode::SystemMenu
                    | ConsoleMode::Settings
            ) {
                return;
            }
            if runtime.text_input_focused() {
                runtime.caret_visible = !runtime.caret_visible;
            }
            let timeout_seconds = u32::from(runtime.user_no_activity_timeout_minutes()) * 60;
            if matches!(
                runtime.mode,
                ConsoleMode::Desktop
                    | ConsoleMode::AppLauncher
                    | ConsoleMode::SystemMenu
                    | ConsoleMode::Settings
            ) && runtime
                .session_idle
                .tick(!runtime.current_session.is_zero(), timeout_seconds)
            {
                if runtime.lock_session_preserving_desktop(true) {
                    runtime.redraw();
                }
                return;
            }
            let next = firmware_date_time(runtime.system.firmware_runtime_services);
            let task_manager_live = runtime.mode == ConsoleMode::Desktop
                && runtime.desktop_app == DesktopAppKind::TaskManager;
            if task_manager_live {
                runtime.refresh_task_manager_output();
            }
            if next != runtime.desktop_clock || runtime.text_input_focused() || task_manager_live {
                runtime.desktop_clock = next;
                runtime.redraw();
            }
        }
    }
}

// ------------------------=
// FUNC: write_minutes_label
// DESC: Formats a bounded minute preference for direct use in the Settings summary row.
// ------------------=
fn write_minutes_label(output: &mut [u8; 48], minutes: u8) -> usize {
    let mut reverse = [0u8; 3];
    let mut value = minutes;
    let mut digits = 0usize;
    while value > 0 {
        reverse[digits] = b'0' + value % 10;
        value /= 10;
        digits += 1;
    }
    for index in 0..digits {
        output[index] = reverse[digits - index - 1];
    }
    let unit = if minutes == 1 {
        b" minute".as_slice()
    } else {
        b" minutes".as_slice()
    };
    output[digits..digits + unit.len()].copy_from_slice(unit);
    digits + unit.len()
}

// ------------------------=
// FUNC: firmware_date_time
// DESC: Reads the UEFI wall clock for installer defaults and falls back to a valid UTC value.
// ------------------=
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
fn firmware_date_time(firmware_runtime_services: u64) -> DateTimeConfiguration {
    if firmware_runtime_services == 0 {
        return DateTimeConfiguration::utc_default();
    }
    let services = firmware_runtime_services as usize as *const EfiRuntimeServices;
    let mut time = EfiTime::zero();
    let status = unsafe { ((*services).get_time)(&mut time, core::ptr::null_mut()) };
    if status != 0 {
        return DateTimeConfiguration::utc_default();
    }
    let firmware_offset = if (-14 * 60..=14 * 60).contains(&time.time_zone) {
        time.time_zone
    } else {
        0
    };
    let zone = TIME_ZONES[time_zone_index(firmware_offset)];
    let configuration = DateTimeConfiguration {
        year: time.year,
        month: time.month,
        day: time.day,
        hour: time.hour,
        minute: time.minute,
        second: time.second,
        time_zone_id: zone.id,
        utc_offset_minutes: zone.offset_minutes,
    };
    if configuration.is_valid() {
        configuration
    } else {
        DateTimeConfiguration::utc_default()
    }
}

// ------------------------=
// FUNC: firmware_date_time
// DESC: Supplies a safe UTC installer default on the legacy BIOS architecture.
// ------------------=
#[cfg(target_arch = "x86")]
fn firmware_date_time(_firmware_runtime_services: u64) -> DateTimeConfiguration {
    DateTimeConfiguration::utc_default()
}

#[cfg(target_arch = "x86_64")]
// ------------------------=
// FUNC: reboot
// DESC: Implements the reboot operation.
// ------------------=
fn reboot(_firmware_runtime_services: u64) -> ! {
    unsafe {
        core::arch::asm!("out dx, ax", in("dx") 0x604u16, in("ax") 0x2000u16, options(nomem, nostack));
    }
    for _ in 0..100_000 {
        core::hint::spin_loop();
    }
    unsafe {
        core::arch::asm!("out dx, al", in("dx") 0x64u16, in("al") 0xfeu8, options(nomem, nostack));
    }
    loop {
        core::hint::spin_loop();
    }
}

#[cfg(target_arch = "aarch64")]
// ------------------------=
// FUNC: reboot
// DESC: Implements the reboot operation.
// ------------------=
fn reboot(firmware_runtime_services: u64) -> ! {
    if firmware_runtime_services != 0 {
        let services = firmware_runtime_services as usize as *const EfiRuntimeServices;
        unsafe {
            ((*services).reset_system)(0, 0, 0, core::ptr::null());
        }
    }
    crate::output_text(b"ERROR: firmware reset returned unexpectedly\n");
    crate::output::idle()
}

#[cfg(target_arch = "x86")]
// ------------------------=
// FUNC: reboot
// DESC: Leaves legacy x86 in a safe idle state because native reset is not yet available.
// ------------------=
fn reboot(_firmware_runtime_services: u64) -> ! {
    crate::output::idle()
}

#[cfg(target_arch = "x86_64")]
// ------------------------=
// FUNC: shutdown
// DESC: Requests ACPI power-off from x86_64 virtual-machine firmware and safely idles if it returns.
// ------------------=
fn shutdown(_firmware_runtime_services: u64) -> ! {
    unsafe {
        core::arch::asm!("out dx, ax", in("dx") 0x604u16, in("ax") 0x2000u16, options(nomem, nostack));
    }
    crate::output::idle()
}

#[cfg(target_arch = "aarch64")]
// ------------------------=
// FUNC: shutdown
// DESC: Requests the UEFI shutdown reset type on AArch64 and safely idles if firmware returns.
// ------------------=
fn shutdown(firmware_runtime_services: u64) -> ! {
    if firmware_runtime_services != 0 {
        let services = firmware_runtime_services as usize as *const EfiRuntimeServices;
        unsafe {
            ((*services).reset_system)(2, 0, 0, core::ptr::null());
        }
    }
    crate::output_text(b"ERROR: firmware shutdown returned unexpectedly\n");
    crate::output::idle()
}

#[cfg(target_arch = "x86")]
// ------------------------=
// FUNC: shutdown
// DESC: Leaves legacy x86 in a safe idle state because native power-off is not yet available.
// ------------------=
fn shutdown(_firmware_runtime_services: u64) -> ! {
    crate::output::idle()
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
#[repr(C)]
struct EfiTableHeader {
    signature: u64,
    revision: u32,
    header_size: u32,
    crc32: u32,
    reserved: u32,
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
#[repr(C)]
struct EfiTime {
    year: u16,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
    pad1: u8,
    nanosecond: u32,
    time_zone: i16,
    daylight: u8,
    pad2: u8,
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
impl EfiTime {
    // ------------------------=
    // FUNC: zero
    // DESC: Creates a zeroed UEFI time record for a firmware GetTime call.
    // ------------------=
    const fn zero() -> Self {
        Self {
            year: 0,
            month: 0,
            day: 0,
            hour: 0,
            minute: 0,
            second: 0,
            pad1: 0,
            nanosecond: 0,
            time_zone: 0,
            daylight: 0,
            pad2: 0,
        }
    }
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
#[repr(C)]
struct EfiRuntimeServices {
    header: EfiTableHeader,
    get_time: unsafe extern "efiapi" fn(*mut EfiTime, *mut u8) -> u64,
    services_after_get_time: [usize; 9],
    reset_system: unsafe extern "efiapi" fn(u32, u64, usize, *const u16),
}

// ------------------------=
// FUNC: report_storage_error
// DESC: Implements the report storage error operation.
// ------------------=
fn report_storage_error(error: StorageError) {
    crate::output_text(b"ERROR: ");
    crate::output_text(storage_error_text(error));
    crate::output_text(b"\n");
}

// ------------------------=
// FUNC: storage_error_text
// DESC: Implements the storage error text operation.
// ------------------=
fn storage_error_text(error: StorageError) -> &'static [u8] {
    match error {
        StorageError::NoDevice => b"no supported storage device",
        StorageError::InvalidPlan => b"provisioning plan validation failed",
        StorageError::InsufficientCapacity => b"insufficient capacity",
        StorageError::Arithmetic => b"storage plan arithmetic overflow",
        StorageError::WriteGpt => b"unable to write GPT header or entries",
        StorageError::WriteBootRegion => b"bootloader installation failed",
        StorageError::WriteContainer => b"unable to write Infinity metadata",
        StorageError::WriteKernel => b"installed kernel write failed",
        StorageError::VerifyGpt => b"GPT verification failed",
        StorageError::VerifyBootEnvironment => b"bootloader verification failed",
        StorageError::VerifyContainer => b"Infinity container checksum mismatch",
        StorageError::VerifyPool => b"Infinity pool verification failed",
        StorageError::VerifySpaces => b"Infinity Spaces verification failed",
        StorageError::VerifyKernel => b"installed kernel verification failed",
        StorageError::WriteSystemGeneration => b"system generation write failed",
        StorageError::VerifySystemGeneration => b"system generation verification failed",
        StorageError::VerifySystemManifest => b"system manifest checksum mismatch",
        StorageError::VerifySystemComponents => b"required CORE component missing or invalid",
        StorageError::ActivateGeneration => b"generation activation failed",
    }
}
