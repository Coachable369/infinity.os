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
use crate::ui::system_layout::{
    AppLauncherTarget, DesktopAppWindowState, DesktopAppWindowTarget, DesktopTarget, OnboardingTarget,
    SettingsAccentTarget, SettingsTarget, SettingsWindowState, SystemLayout, SystemMenuTarget,
};
use crate::ui::text_editor::TextDocument;

const OUTPUT_ROWS: usize = 6;
const LINE_CAPACITY: usize = 96;
const COMMAND_CAPACITY: usize = 160;
const EDITOR_DOCUMENT_PATH: &[u8] = b"/personal/documents/text-editor-document";

#[derive(Clone, Copy, PartialEq, Eq)]
enum DesktopAppKind {
    None,
    TextEditor,
    CommandWindow,
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
    Help,
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
    intent: IntentRuntime,
    language_session: crate::runtime::console_language::ConsoleSession,
    system: SystemSnapshot,
    pointer_x: i32,
    pointer_y: i32,
    pointer_x_remainder: i32,
    pointer_y_remainder: i32,
    pointer_pressed: bool,
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
    settings_window: SettingsWindowState,
    settings_window_dragging: bool,
    settings_window_resizing: Option<usize>,
    settings_window_drag_offset_x: i32,
    settings_window_drag_offset_y: i32,
    settings_accent_dirty: bool,
    settings_primary_dirty: bool,
    settings_scroll_dragging: bool,
    settings_scroll_grab_offset: i32,
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
    editor_window: DesktopAppWindowState,
    command_window: DesktopAppWindowState,
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
            intent: IntentRuntime::new(),
            language_session: crate::runtime::console_language::ConsoleSession::new(),
            system,
            pointer_x: 500,
            pointer_y: 500,
            pointer_x_remainder: 0,
            pointer_y_remainder: 0,
            pointer_pressed: false,
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
            settings_window: SettingsWindowState {
                x: 160,
                y: 210,
                width: 680,
                height: 620,
                maximized: false,
                expanded_row: None,
                scroll_offset: 0,
                row_count: 6,
            },
            settings_window_dragging: false,
            settings_window_resizing: None,
            settings_window_drag_offset_x: 0,
            settings_window_drag_offset_y: 0,
            settings_accent_dirty: false,
            settings_primary_dirty: false,
            settings_scroll_dragging: false,
            settings_scroll_grab_offset: 0,
            onboarding_validation_error: false,
            home_window_x: 30,
            home_window_y: 400,
            home_window_width: 430,
            home_window_height: 480,
            home_window_visible: true,
            home_window_maximized: false,
            home_window_restore_x: 30,
            home_window_restore_y: 400,
            home_window_restore_width: 430,
            home_window_restore_height: 480,
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
            editor_window: DesktopAppWindowState::new(190, 160, 600, 620),
            command_window: DesktopAppWindowState::new(240, 210, 600, 620),
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
            if self.mode == ConsoleMode::Settings && !self.settings_editing {
                if let Some(machine) =
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
            let (editor_window, command_window) = self.desktop_app_windows();
            crate::bootstrap::system_ui_present(
                screen,
                self.system_step,
                displayed_input,
                self.mode == ConsoleMode::Locked
                    || self.mode == ConsoleMode::Authentication
                    || (self.mode == ConsoleMode::Onboarding && self.system_step == 4),
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
        if self.mode == ConsoleMode::Desktop {
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
    // FUNC: input_text_editor
    // DESC: Applies bounded multiline editing and native close keyboard behavior.
    // ------------------=
    fn input_text_editor(&mut self, key: ConsoleKey) {
        match key {
            ConsoleKey::Character(character) if (32..=126).contains(&character) => {
                let _ = self.editor_document.insert(character);
            }
            ConsoleKey::Enter => {
                let _ = self.editor_document.insert(b'\n');
            }
            ConsoleKey::Backspace => {
                let _ = self.editor_document.backspace();
            }
            ConsoleKey::Escape => self.enter_desktop(),
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
        self.shell_menu = 0;
        self.settings_editing = false;
        self.settings_accent_dirty = false;
        self.settings_primary_dirty = false;
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
        self.refresh_desktop_items();
        self.reset_input();
        crate::output_text(b"[shell] top bar ready\n[shell] Infinity menu ready\n[settings] graphical settings ready\n");
    }

    // ------------------------=
    // FUNC: open_text_editor
    // DESC: Opens the native multiline Text Editor as an authenticated desktop window.
    // ------------------=
    fn open_text_editor(&mut self) {
        if self.mode != ConsoleMode::Desktop {
            self.enter_desktop();
        }
        self.store_active_app_window();
        self.desktop_app = DesktopAppKind::TextEditor;
        self.editor_window.visible = true;
        self.load_active_app_window();
        self.app_window_dragging = false;
        self.app_window_resizing = None;
    }

    // ------------------------=
    // FUNC: open_command_window
    // DESC: Opens the native Infinity Console language inside a desktop command window.
    // ------------------=
    fn open_command_window(&mut self) {
        if self.mode != ConsoleMode::Desktop {
            self.enter_desktop();
        }
        self.store_active_app_window();
        self.desktop_app = DesktopAppKind::CommandWindow;
        self.command_window.visible = true;
        self.load_active_app_window();
        self.app_window_dragging = false;
        self.app_window_resizing = None;
        self.reset_input();
        self.output.clear();
        self.output.write_line(b"Infinity Command Window");
        self.output
            .write_line(b"Type help or describe what you want.");
        crate::output_text(b"[ui] desktop command window opened\n");
    }

    // ------------------------=
    // FUNC: close_desktop_app
    // DESC: Dismisses the active desktop application without changing session or desktop state.
    // ------------------=
    fn close_desktop_app(&mut self) {
        match self.desktop_app {
            DesktopAppKind::TextEditor => self.editor_window.visible = false,
            DesktopAppKind::CommandWindow => self.command_window.visible = false,
            DesktopAppKind::None => {}
        }
        self.desktop_app = DesktopAppKind::None;
        self.app_window_dragging = false;
        self.app_window_resizing = None;
        self.reset_input();
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
            DesktopAppKind::None => return,
        };
        self.app_window_x = state.x;
        self.app_window_y = state.y;
        self.app_window_width = state.width;
        self.app_window_height = state.height;
        self.app_window_maximized = state.maximized;
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
    }

    // ------------------------=
    // FUNC: desktop_app_windows
    // DESC: Returns both open window states with the focused window's live geometry applied.
    // ------------------=
    fn desktop_app_windows(&self) -> (DesktopAppWindowState, DesktopAppWindowState) {
        let mut editor = self.editor_window;
        let mut command = self.command_window;
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
            DesktopAppKind::None => {}
        }
        (editor, command)
    }

    // ------------------------=
    // FUNC: inactive_app_at_pointer
    // DESC: Hit-tests visible non-focused application windows for click-to-raise behavior.
    // ------------------=
    fn inactive_app_at_pointer(&self, layout: SystemLayout) -> Option<DesktopAppKind> {
        let (editor, command) = self.desktop_app_windows();
        for (app, state, is_editor) in [
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
        let content = self.editor_document.bytes();
        let saved = crate::storage::object_write_path(EDITOR_DOCUMENT_PATH, content)
            .map(|_| ())
            .or_else(|error| {
                if error == crate::storage::object::ObjectError::NotFound {
                    crate::storage::object_create_note_at(
                        b"text-editor-document",
                        content,
                        EDITOR_DOCUMENT_PATH,
                    )
                    .map(|_| ())
                } else {
                    Err(error)
                }
            })
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
        let mut content = [0u8; crate::ui::text_editor::DOCUMENT_CAPACITY];
        if let Ok((_, length)) = crate::storage::object_read_path(EDITOR_DOCUMENT_PATH, None, &mut content) {
            if self.editor_document.open(&content[..length]) {
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
        if crate::storage::object_remove_path(EDITOR_DOCUMENT_PATH).is_ok() {
            self.editor_document.clear();
            crate::output_text(b"[editor] document deleted\n");
        } else {
            crate::output_text(b"[editor] delete failed\n");
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
        layout.desktop_target_sized(
            self.pointer_x,
            self.pointer_y,
            self.home_window_x,
            self.home_window_y,
            self.home_window_width,
            self.home_window_height,
            self.home_window_visible,
            self.home_window_maximized,
        )
    }

    // ------------------------=
    // FUNC: open_settings
    // DESC: Opens one Settings section without accidentally activating its first value.
    // ------------------=
    fn open_settings(&mut self, section: usize) {
        self.store_active_app_window();
        self.mode = ConsoleMode::Settings;
        self.system_focus = section.min(7);
        self.settings_window.row_count = if self.system_focus == 1 { 6 } else { 5 };
        self.settings_editing = false;
        self.settings_window.expanded_row = None;
        self.settings_window.scroll_offset = 0;
        self.settings_window_dragging = false;
        self.settings_window_resizing = None;
        self.settings_accent_dirty = false;
        self.settings_primary_dirty = false;
        self.settings_scroll_dragging = false;
        self.reset_input();
    }

    // ------------------------=
    // FUNC: toggle_settings_row
    // DESC: Opens one inline Settings detail well, closes it on a second activation, and reveals lower rows safely.
    // ------------------=
    fn toggle_settings_row(&mut self, row: usize) {
        self.settings_window.expanded_row = if self.settings_window.expanded_row == Some(row) {
            None
        } else {
            Some(row.min(self.settings_window.row_count.saturating_sub(1)))
        };
        self.settings_window.scroll_offset = 0;
        let layout = SystemLayout::new(
            self.system.framebuffer_width,
            self.system.framebuffer_height,
        );
        let window = layout.settings_window_geometry(self.settings_window);
        let desired_scroll = self
            .settings_window
            .expanded_row
            .map(|expanded| {
                layout
                    .settings_row_geometry(self.settings_window, expanded)
                    .detail
                    .bottom()
                    .saturating_sub(window.viewport.bottom())
                    .max(0) as usize
                    / layout.scale().max(1)
            })
            .unwrap_or(0);
        let maximum_scroll = window.maximum_scroll;
        self.settings_window.scroll_offset = desired_scroll.min(maximum_scroll);
    }

    // ------------------------=
    // FUNC: scroll_settings
    // DESC: Moves the Settings accordion by bounded logical increments while leaving navigation focus unchanged.
    // ------------------=
    fn scroll_settings(&mut self, direction: i8) {
        let distance = direction.unsigned_abs() as usize * 28;
        let maximum_scroll = SystemLayout::new(
            self.system.framebuffer_width,
            self.system.framebuffer_height,
        )
        .settings_window_geometry(self.settings_window)
        .maximum_scroll;
        if direction < 0 {
            self.settings_window.scroll_offset =
                self.settings_window.scroll_offset.saturating_sub(distance);
        } else {
            self.settings_window.scroll_offset = self
                .settings_window
                .scroll_offset
                .saturating_add(distance)
                .min(maximum_scroll);
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
        const PRESETS: [u32; 6] = [
            0x0d2238, 0x162a46, 0x251f42, 0x142f36, 0x35233d, 0x273041,
        ];
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
            (1, 3) => self.cycle_primary(),
            (1, 4) => self.cycle_accent(),
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
            _ => {}
        }
    }

    // ------------------------=
    // FUNC: open_shell_menu
    // DESC: Opens one native top-bar menu and selects its first actionable row.
    // ------------------=
    fn open_shell_menu(&mut self, menu: usize) {
        self.mode = ConsoleMode::SystemMenu;
        self.shell_menu = menu.min(5);
        self.system_focus = 0;
    }

    // ------------------------=
    // FUNC: open_app_launcher
    // DESC: Opens the native installed application launcher with an empty live search query.
    // ------------------=
    fn open_app_launcher(&mut self) {
        self.store_active_app_window();
        self.mode = ConsoleMode::AppLauncher;
        self.system_focus = 0;
        self.reset_input();
    }

    // ------------------------=
    // FUNC: activate_launcher_action
    // DESC: Routes one typed launcher entry into a real Home, Settings, Text Editor, or Command surface.
    // ------------------=
    fn activate_launcher_action(&mut self, action: LauncherAction) {
        match action {
            LauncherAction::Home(location) => {
                self.home_window_visible = true;
                self.home_previous_location = self.home_location;
                self.home_location = location.min(8);
                self.home_selected_item = None;
                self.enter_desktop();
            }
            LauncherAction::Settings(section) => self.open_settings(section),
            LauncherAction::TextEditor => self.open_text_editor(),
            LauncherAction::CommandWindow => self.open_command_window(),
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
    // FUNC: shell_menu_item_count
    // DESC: Returns the bounded row count for the currently open native menu.
    // ------------------=
    fn shell_menu_item_count(&self) -> usize {
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
        match (self.shell_menu, self.system_focus) {
            (0, 0) => self.open_settings(7),
            (0, 1) => self.open_settings(0),
            (0, 2) => self.open_settings(2),
            (0, 3) => self.open_settings(3),
            (0, 4) => self.open_settings(5),
            (0, 5) => self.open_settings(4),
            (0, 6) => {
                let _ = crate::runtime::with_runtime(|runtime| {
                    runtime
                        .identity
                        .lock_session(self.current_session, self.current_user)
                });
                self.mode = ConsoleMode::Locked;
                self.reset_input();
            }
            (0, 7) => {
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
                crate::output_text(b"[shell] restart requested\n");
                reboot(self.system.firmware_runtime_services);
            }
            (0, 9) => {
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
            (5, 0 | 3) => self.open_settings(7),
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
        match item {
            0..=2 => self.open_settings(5),
            3 | 4 => self.open_settings(7),
            5 => self.show_shell_notice(b"Search objects with: object find name=..."),
            _ => self.open_shell_menu(0),
        }
    }

    // ------------------------=
    // FUNC: edit_system_text
    // DESC: Applies bounded private text editing without sending credentials to logs or events.
    // ------------------=
    fn edit_system_text(&mut self, key: ConsoleKey) -> bool {
        match key {
            ConsoleKey::Character(character)
                if self.command_length < COMMAND_CAPACITY && character >= 0x20 =>
            {
                self.command[self.command_length] = character;
                self.command_length += 1;
                true
            }
            ConsoleKey::Backspace if self.command_length > 0 => {
                self.command_length -= 1;
                self.command[self.command_length] = 0;
                true
            }
            _ => false,
        }
    }

    // ------------------------=
    // FUNC: input_onboarding
    // DESC: Advances modular first-boot steps and commits each authoritative identity change.
    // ------------------=
    fn input_onboarding(&mut self, key: ConsoleKey) {
        if matches!(
            key,
            ConsoleKey::Tab(_) | ConsoleKey::Left | ConsoleKey::Right
        ) && self.system_step > 0
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
        if self.system_focus == 1 && self.edit_system_text(key) {
            self.onboarding_validation_error = false;
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
            1 => {
                if self.command_length == 0 {
                    self.onboarding_validation_error = true;
                    return;
                }
                self.onboarding_machine_length = self.command_length.min(48);
                self.onboarding_machine[..self.onboarding_machine_length]
                    .copy_from_slice(&self.command[..self.onboarding_machine_length]);
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
                .unwrap_or(Err(crate::runtime::identity::IdentityError::InvalidState));
                if created.is_err() {
                    self.onboarding_validation_error = true;
                    return;
                }
                self.system_step = 2;
            }
            2 => {
                if self.command_length == 0 {
                    self.onboarding_validation_error = true;
                    return;
                }
                self.onboarding_handle_length = self.command_length.min(48);
                self.onboarding_handle[..self.onboarding_handle_length]
                    .copy_from_slice(&self.command[..self.onboarding_handle_length]);
                self.system_step = 3;
            }
            3 => {
                if self.command_length == 0 {
                    self.onboarding_validation_error = true;
                    return;
                }
                self.onboarding_name_length = self.command_length.min(48);
                self.onboarding_name[..self.onboarding_name_length]
                    .copy_from_slice(&self.command[..self.onboarding_name_length]);
                let created = crate::runtime::with_runtime(|runtime| {
                    runtime.identity.create_user(
                        &self.onboarding_handle[..self.onboarding_handle_length],
                        &self.onboarding_name[..self.onboarding_name_length],
                        2,
                    )
                })
                .unwrap_or(Err(crate::runtime::identity::IdentityError::InvalidState));
                let Ok(user) = created else {
                    self.onboarding_validation_error = true;
                    return;
                };
                self.current_user = user.id;
                self.system_step = 4;
            }
            4 => {
                if self.command_length < 8 || self.command_length > self.onboarding_secret.len() {
                    self.onboarding_validation_error = true;
                    return;
                }
                self.onboarding_secret_length = self.command_length;
                self.onboarding_secret[..self.onboarding_secret_length]
                    .copy_from_slice(&self.command[..self.onboarding_secret_length]);
                let created = crate::runtime::with_runtime(|runtime| {
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
                .unwrap_or(Err(crate::runtime::identity::IdentityError::InvalidState));
                if created.is_err() {
                    self.onboarding_validation_error = true;
                    return;
                }
                self.system_step = 5;
            }
            5 => self.system_step = 6,
            _ => {
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
        self.system_focus = 1;
        self.onboarding_validation_error = false;
        let _ = crate::runtime::persist_identity_state();
    }

    // ------------------------=
    // FUNC: restore_onboarding_input
    // DESC: Restores previously committed first-boot values when navigating backward without exposing secrets externally.
    // ------------------=
    fn restore_onboarding_input(&mut self) {
        self.reset_input();
        let (source, length): (&[u8], usize) = match self.system_step {
            1 => (&self.onboarding_machine, self.onboarding_machine_length),
            2 => (&self.onboarding_handle, self.onboarding_handle_length),
            3 => (&self.onboarding_name, self.onboarding_name_length),
            4 => (&self.onboarding_secret, self.onboarding_secret_length),
            _ => (&[], 0),
        };
        let copied = length.min(COMMAND_CAPACITY).min(source.len());
        self.command[..copied].copy_from_slice(&source[..copied]);
        self.command_length = copied;
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
                self.enter_desktop();
                return;
            }
            if matches!(key, ConsoleKey::Character(b'/')) && self.command_length == 0 {
                self.system_focus = 0;
                return;
            }
            if matches!(key, ConsoleKey::Character(_) | ConsoleKey::Backspace) {
                if self.edit_system_text(key) {
                    self.system_focus =
                        if launcher_visible_count(&self.command[..self.command_length]) > 0 {
                            1
                        } else {
                            0
                        };
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
                return;
            }
            if matches!(
                key,
                ConsoleKey::Down | ConsoleKey::Right | ConsoleKey::Tab(false)
            ) {
                self.system_focus = (self.system_focus + 1) % focus_count;
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
                8
            };
            self.system_focus = (self.system_focus + count - 1) % count;
            if self.mode == ConsoleMode::Settings {
                self.settings_window.row_count = if self.system_focus == 1 { 6 } else { 5 };
                self.settings_window.expanded_row = None;
                self.settings_window.scroll_offset = 0;
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
                8
            };
            self.system_focus = (self.system_focus + 1) % count;
            if self.mode == ConsoleMode::Settings {
                self.settings_window.row_count = if self.system_focus == 1 { 6 } else { 5 };
                self.settings_window.expanded_row = None;
                self.settings_window.scroll_offset = 0;
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
        match key {
            ConsoleKey::Backspace => {
                self.command_length = self.command_length.saturating_sub(1);
                true
            }
            ConsoleKey::Character(character) if (32..=126).contains(&character) => {
                if self.command_length < COMMAND_CAPACITY {
                    self.command[self.command_length] = character;
                    self.command_length += 1;
                }
                true
            }
            _ => false,
        }
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
    fn pointer(&mut self, delta_x: i16, delta_y: i16, left_button: bool) {
        if matches!(self.mode, ConsoleMode::Console | ConsoleMode::Repair) {
            return;
        }
        let button_changed = left_button != self.pointer_pressed;
        if delta_x == 0 && delta_y == 0 && !button_changed {
            return;
        }
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
        // The pointer is a screen-level device, not a console-panel device.
        // Keep only a small edge inset so the cursor remains visible.
        self.pointer_x = (self.pointer_x + accelerated_x).clamp(8, 992);
        self.pointer_y = (self.pointer_y + accelerated_y).clamp(8, 992);
        self.pointer_interaction(left_button);
    }

    // ------------------------=
    // FUNC: pointer_scroll
    // DESC: Consumes wheel motion inside Settings as content scrolling instead of changing the selected navigation section.
    // ------------------=
    fn pointer_scroll(&mut self, vertical: i8) -> bool {
        if self.mode != ConsoleMode::Settings || vertical == 0 {
            return false;
        }
        self.scroll_settings(vertical);
        self.redraw();
        true
    }

    // ------------------------=
    // FUNC: pointer_interaction
    // DESC: Handles pointer interaction input or state transitions.
    // ------------------=
    fn pointer_interaction(&mut self, left_button: bool) {
        // Activate on the press edge. VirtualBox can consume the release packet
        // used to capture a relative USB pointer, so release-edge activation
        // makes a visibly moving mouse appear unable to click.
        let clicked = left_button && !self.pointer_pressed;
        let released = !left_button && self.pointer_pressed;
        self.pointer_pressed = left_button;
        let layout = SystemLayout::new(
            self.system.framebuffer_width,
            self.system.framebuffer_height,
        );
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
            let welcome = self.installer_step == InstallerStep::Welcome;
            let hierarchy = self.installer_step == InstallerStep::Hierarchy;
            let over_back = self.installer_has_back()
                && if popup {
                    (310..=480).contains(&self.pointer_x) && (570..=630).contains(&self.pointer_y)
                } else if welcome {
                    (145..=470).contains(&self.pointer_x) && (810..=865).contains(&self.pointer_y)
                } else if hierarchy {
                    (135..=485).contains(&self.pointer_x) && (820..=875).contains(&self.pointer_y)
                } else {
                    (190..=490).contains(&self.pointer_x) && (830..=885).contains(&self.pointer_y)
                };
            let over_primary = self.installer_has_primary()
                && if popup {
                    (520..=730).contains(&self.pointer_x) && (570..=630).contains(&self.pointer_y)
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
                }
                if clicked && !matches!(target, OnboardingTarget::Input) {
                    self.input_onboarding(ConsoleKey::Enter);
                }
            }
        } else if self.mode == ConsoleMode::Desktop {
            if let Some(corner) = self.app_window_resizing {
                if left_button {
                    let resized = crate::ui::system_layout::resize_native_window(
                        self.app_window_x,
                        self.app_window_y,
                        self.app_window_width,
                        self.app_window_height,
                        corner,
                        self.pointer_x,
                        self.pointer_y,
                        420,
                        360,
                    );
                    self.app_window_x = resized.0;
                    self.app_window_y = resized.1;
                    self.app_window_width = resized.2;
                    self.app_window_height = resized.3;
                }
                if released {
                    self.app_window_resizing = None;
                }
            } else if self.app_window_dragging {
                if left_button {
                    self.app_window_x = (self.pointer_x - self.app_window_drag_offset_x)
                        .clamp(0, 1000i32.saturating_sub(self.app_window_width));
                    self.app_window_y = (self.pointer_y - self.app_window_drag_offset_y)
                        .clamp(50, 900i32.saturating_sub(self.app_window_height));
                }
                if released {
                    self.app_window_dragging = false;
                }
            } else if self.desktop_app != DesktopAppKind::None {
                if clicked {
                    match layout.desktop_app_window_target(
                        self.pointer_x,
                        self.pointer_y,
                        self.app_window_x,
                        self.app_window_y,
                        self.app_window_width,
                        self.app_window_height,
                        self.app_window_maximized,
                        self.desktop_app == DesktopAppKind::TextEditor,
                    ) {
                        DesktopAppWindowTarget::Resize(corner) if !self.app_window_maximized => {
                            self.app_window_resizing = Some(corner);
                        }
                        DesktopAppWindowTarget::Title if !self.app_window_maximized => {
                            self.app_window_dragging = true;
                            self.app_window_drag_offset_x = self.pointer_x - self.app_window_x;
                            self.app_window_drag_offset_y = self.pointer_y - self.app_window_y;
                        }
                        DesktopAppWindowTarget::Minimize | DesktopAppWindowTarget::Close => {
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
                        }
                        DesktopAppWindowTarget::NewDocument => {
                            self.editor_document.clear();
                        }
                        DesktopAppWindowTarget::OpenDocument => self.open_editor_document(),
                        DesktopAppWindowTarget::SaveDocument => {
                            self.save_editor_document();
                        }
                        DesktopAppWindowTarget::DeleteDocument => self.delete_editor_document(),
                        DesktopAppWindowTarget::None => {
                            if let Some(app) = self.inactive_app_at_pointer(layout) {
                                self.focus_desktop_app(app);
                            } else {
                                match self.desktop_target(layout) {
                                    Some(DesktopTarget::InfinityMenu) => self.open_shell_menu(0),
                                    Some(DesktopTarget::TopMenu(menu)) => self.open_shell_menu(menu),
                                    Some(DesktopTarget::Status(item)) => self.activate_status_item(item),
                                    Some(DesktopTarget::Dock(0)) => self.open_app_launcher(),
                                    _ => {}
                                }
                            }
                        }
                        DesktopAppWindowTarget::Content
                        | DesktopAppWindowTarget::Title
                        | DesktopAppWindowTarget::Resize(_) => {}
                    }
                }
            } else if clicked {
                if let Some(app) = self.inactive_app_at_pointer(layout) {
                    self.focus_desktop_app(app);
                    self.redraw();
                    return;
                }
            }
            if let Some(corner) = self.home_window_resizing {
                if released {
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
                    self.home_window_resizing = None;
                }
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
                    } else if !self.home_drag_moved && item < 6 {
                        self.home_previous_location = self.home_location;
                        self.home_location = item + 2;
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
                }
            } else if self.home_window_dragging {
                if left_button {
                    self.home_window_x = (self.pointer_x - self.home_window_drag_offset_x)
                        .clamp(0, 1000i32.saturating_sub(self.home_window_width));
                    self.home_window_y = (self.pointer_y - self.home_window_drag_offset_y)
                        .clamp(50, 920i32.saturating_sub(self.home_window_height));
                }
                if released {
                    self.home_window_dragging = false;
                }
            } else if clicked {
                match self.desktop_target(layout) {
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
                    Some(DesktopTarget::HomeControl(0)) => self.home_window_visible = false,
                    Some(DesktopTarget::HomeControl(2)) => {
                        self.home_window_visible = false;
                        self.home_selected_item = None;
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
                    }
                    Some(DesktopTarget::HomeToolbar(_)) => {
                        core::mem::swap(&mut self.home_location, &mut self.home_previous_location);
                        self.home_selected_item = None;
                    }
                    Some(DesktopTarget::HomeSidebar(location)) => {
                        self.home_previous_location = self.home_location;
                        self.home_location = location;
                        self.home_selected_item = None;
                    }
                    Some(DesktopTarget::HomeItem(item)) => {
                        self.home_selected_item = Some(item);
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
                            Some(DockAction::Files) => self.home_window_visible = true,
                            Some(DockAction::Settings) => self.open_settings(0),
                            Some(DockAction::About) => self.open_settings(7),
                            Some(DockAction::AiVoice) => self.open_settings(3),
                            Some(DockAction::Appearance) => self.open_settings(1),
                            Some(DockAction::Network) => self.open_settings(5),
                            Some(DockAction::Trash) => {
                                self.home_window_visible = true;
                                self.home_location = 8;
                                crate::output_text(b"[objects] recycle collection opened\n")
                            }
                            None => {}
                        }
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
            match layout.app_launcher_target(self.pointer_x, self.pointer_y, visible) {
                AppLauncherTarget::Search => self.system_focus = 0,
                AppLauncherTarget::App(index) => {
                    self.system_focus = index + 1;
                    if clicked {
                        self.activate_launcher_focus();
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
                    self.enter_desktop();
                }
                AppLauncherTarget::Panel
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
            if self.settings_scroll_dragging {
                if left_button {
                    self.settings_window.scroll_offset = layout.settings_scroll_offset_for_thumb(
                        self.pointer_y,
                        self.settings_window,
                        self.settings_scroll_grab_offset,
                    );
                }
                if released {
                    self.settings_scroll_dragging = false;
                }
                self.redraw();
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
                }
                self.redraw();
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
                }
                self.redraw();
                return;
            } else if self.system_focus == 1 && left_button {
                if let Some(target) = layout.settings_primary_target(
                    self.pointer_x,
                    self.pointer_y,
                    self.settings_window,
                ) {
                    self.adjust_primary(target);
                    self.redraw();
                    return;
                }
                if let Some(target) = layout.settings_accent_target(
                    self.pointer_x,
                    self.pointer_y,
                    self.settings_window,
                ) {
                    self.adjust_accent(target);
                    self.redraw();
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
            if let Some(target) =
                layout.settings_target(self.pointer_x, self.pointer_y, self.settings_window)
            {
                match target {
                    SettingsTarget::Section(index) if clicked => {
                        self.system_focus = index;
                        self.settings_window.row_count = if index == 1 { 6 } else { 5 };
                        self.settings_editing = false;
                        self.settings_window.expanded_row = None;
                        self.settings_window.scroll_offset = 0;
                        self.reset_input();
                    }
                    SettingsTarget::ContentRow(row) if clicked => self.toggle_settings_row(row),
                    SettingsTarget::ExpandedAction if clicked => {
                        if let Some(row) = self.settings_window.expanded_row {
                            self.activate_settings_content_row(row);
                        }
                    }
                    SettingsTarget::ScrollPage(down) if clicked => {
                        self.scroll_settings(if down { 4 } else { -4 })
                    }
                    SettingsTarget::ScrollThumb if clicked => {
                        let geometry = layout.settings_window_geometry(self.settings_window);
                        let pointer_y = self.system.framebuffer_height as i32 * self.pointer_y / 1000;
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
                    SettingsTarget::WindowControl(0 | 2) if clicked => self.enter_desktop(),
                    SettingsTarget::WindowControl(1) if clicked => {
                        self.settings_window.maximized = !self.settings_window.maximized
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
        self.redraw();
    }

    // ------------------------=
    // FUNC: pointer_absolute
    // DESC: Handles pointer absolute input or state transitions.
    // ------------------=
    fn pointer_absolute(&mut self, x: i32, y: i32, left_button: bool) {
        if matches!(self.mode, ConsoleMode::Console | ConsoleMode::Repair) {
            return;
        }
        let next_x = x.clamp(0, 1000);
        let next_y = y.clamp(0, 1000);
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
        crate::bootstrap::note_pointer_activity();
        self.pointer_x = next_x;
        self.pointer_y = next_y;
        self.pointer_interaction(left_button);
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
                        .write_line(b"service runtime capability event ai model agent voice");
                }
            }
            ParseOutcome::Graph(graph) => {
                crate::output_text(b"[console] typed operation graph validated\n");
                if graph.plan_only || graph.maximum_effect != SideEffectClass::Query {
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
            _ => {
                // Existing service-specific handlers remain the typed operation adapters
                // until all services accept native IOP payloads directly.
                return self.execute_runtime_command_for_operation(node.schema.operation);
            }
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
                #[cfg(any(feature = "installer", target_arch = "x86"))]
                self.show_startup();
                #[cfg(all(not(feature = "installer"), not(target_arch = "x86")))]
                self.output
                    .write_line(b"Installed system console cannot exit to Recovery Node.");
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
        #[cfg(all(not(feature = "installer"), not(target_arch = "x86")))]
        {
            self.output.write_line(b"Active generation: 1");
            self.output.write_line(b"State: ACTIVE");
            self.output.write_line(b"Build: 1");
            self.output
                .write_segments(&[b"Architecture: ", self.system.architecture]);
            self.output
                .write_line(b"Integrity: valid (verified by Infinity EFI)");
        }
        #[cfg(any(feature = "installer", target_arch = "x86"))]
        self.output
            .write_line(b"No installed generation is active in recovery/development boot mode.");
    }

    // ------------------------=
    // FUNC: system_boot
    // DESC: Implements the system boot operation.
    // ------------------=
    fn system_boot(&mut self) {
        crate::output_text(b"[operation] System.BootStatus\n");
        #[cfg(all(not(feature = "installer"), not(target_arch = "x86")))]
        {
            self.output.write_line(b"Boot mode: Installed");
            self.output.write_line(b"Bootloader: Infinity EFI");
            self.output.write_line(b"Boot device: storage0");
            self.output.write_line(b"System Space: online");
            self.output.write_line(b"Generation: 1");
        }
        #[cfg(any(feature = "installer", target_arch = "x86"))]
        {
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
    #[cfg(any(feature = "installer", target_arch = "x86"))]
    runtime.show_startup();
    #[cfg(all(not(feature = "installer"), not(target_arch = "x86")))]
    {
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
    unsafe {
        let slot = &raw mut RUNTIME;
        if let Some(runtime) = (*slot).as_mut() {
            runtime.pointer(delta_x, delta_y, left_button);
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
    unsafe {
        let slot = &raw mut RUNTIME;
        if let Some(runtime) = (*slot).as_mut() {
            runtime.pointer_absolute(x, y, left_button);
        }
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
                ConsoleMode::Desktop
                    | ConsoleMode::AppLauncher
                    | ConsoleMode::SystemMenu
                    | ConsoleMode::Settings
            ) {
                return;
            }
            let next = firmware_date_time(runtime.system.firmware_runtime_services);
            if next != runtime.desktop_clock {
                runtime.desktop_clock = next;
                runtime.redraw();
            }
        }
    }
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
