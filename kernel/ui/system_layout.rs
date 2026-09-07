//! Shared pixel-exact hit geometry for the installed InfinityOS experience.
//!
//! The framebuffer renderer and input runtime both use physical pixels. This
//! module converts normalized device coordinates once, then derives hit regions
//! from the same formulas used by the polished onboarding, authentication,
//! desktop, menu, and Settings surfaces.

use super::geometry::{Point, Rect};
use super::installer_template::InstallerTemplateRole;

pub const SETTINGS_SECTION_ICON_SIZE: usize = 25;
pub const SETTINGS_NETWORK_SECTION: usize = 6;
pub const SETTINGS_NODE_SECTION: usize = 7;
pub const SETTINGS_DASHBOARD_CONTENT_HEIGHT: usize = 670;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OnboardingTarget {
    Back,
    Primary,
    Input,
    NetworkChoice(usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DesktopTarget {
    InfinityMenu,
    TopMenu(usize),
    Status(usize),
    HomeTitle,
    HomeControl(usize),
    HomeResize(usize),
    HomeMenu(usize),
    HomeMenuItem(usize),
    HomeDialogAction(usize),
    HomeToolbar(usize),
    HomeLocation,
    HomeSidebar(usize),
    HomeItem(usize),
    HomeContent,
    Dock(usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AiChatTarget {
    Model,
    Timeline,
    Composer,
    Send,
    Minimize,
    Close,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AiChatGeometry {
    pub panel: Rect,
    pub header: Rect,
    pub model: Rect,
    pub timeline: Rect,
    pub composer: Rect,
    pub send: Rect,
    pub minimize: Rect,
    pub close: Rect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppLauncherTarget {
    Search,
    App(usize),
    Category(usize),
    ScrollbarThumb,
    ScrollbarTrack,
    Close,
    DockToggle,
    Panel,
    Dismiss,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DesktopAppWindowTarget {
    Title,
    Resize(usize),
    Minimize,
    Maximize,
    Close,
    NewDocument,
    OpenDocument,
    SaveDocument,
    SaveAsDocument,
    DeleteDocument,
    Content,
    None,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorScrollTarget {
    Page(bool),
    Thumb,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorDialogTarget {
    NameField,
    Row(usize),
    Cancel,
    Accept,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DesktopAppWindowGeometry {
    pub window: Rect,
    pub title: Rect,
    pub minimize: Rect,
    pub maximize: Rect,
    pub close: Rect,
    pub toolbar: Rect,
    pub content: Rect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DesktopToolbarGeometry {
    pub actions: [Rect; 5],
    pub status: Rect,
}

pub const UI_GUTTER: usize = 16;
pub const UI_CONTROL_GAP: usize = 12;
pub const UI_TOOLBAR_ACTION_HEIGHT: usize = 34;
pub const UI_COMPACT_ACTION_HEIGHT: usize = 40;
pub const UI_STANDARD_ACTION_HEIGHT: usize = 48;
pub const UI_HERO_ACTION_HEIGHT: usize = 56;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EditorScrollGeometry {
    pub track: Rect,
    pub thumb: Rect,
    pub maximum_scroll: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DesktopAppWindowState {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub maximized: bool,
    pub visible: bool,
}

impl DesktopAppWindowState {
    // ------------------------=
    // FUNC: new
    // DESC: Creates one independently manipulable restored desktop application window.
    // ------------------=
    pub const fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
            maximized: false,
            visible: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AppLauncherGeometry {
    pub panel: Rect,
    pub search: Rect,
    pub close: Rect,
    pub grid_left: usize,
    pub grid_top: usize,
    pub grid_cell_width: usize,
    pub grid_row_height: usize,
    pub grid_viewport: Rect,
    pub scrollbar_track: Rect,
    pub category_left: usize,
    pub category_top: usize,
    pub category_width: usize,
    pub category_height: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AppLauncherScrollGeometry {
    pub maximum_scroll: usize,
    pub thumb: Rect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DesktopForegroundGeometry {
    pub widgets: Rect,
    pub dock: Rect,
}

pub(crate) const DESKTOP_FOREGROUND_WIDGETS: u8 = 1;
pub(crate) const DESKTOP_FOREGROUND_DOCK: u8 = 2;

// ------------------------=
// FUNC: window_transition_damage
// DESC: Returns padded and clipped old-plus-new bounds for bounded scene reconstruction during movement or resize.
// ------------------=
pub fn window_transition_damage(old: Rect, new: Rect, display: Rect, padding: u32) -> Rect {
    let union = old.union(new);
    Rect {
        x: union.x.saturating_sub(padding as i32),
        y: union.y.saturating_sub(padding as i32),
        width: union.width.saturating_add(padding.saturating_mul(2)),
        height: union.height.saturating_add(padding.saturating_mul(2)),
    }
    .intersection(display)
}

// ------------------------=
// FUNC: window_motion_damage_regions
// DESC: Returns separate padded old and new regions so distant window movement never repaints the space between them.
// ------------------=
pub fn window_motion_damage_regions(
    old: Rect,
    new: Rect,
    display: Rect,
    padding: u32,
) -> [Rect; 2] {
    [
        window_transition_damage(old, old, display, padding),
        window_transition_damage(new, new, display, padding),
    ]
}

// ------------------------=
// FUNC: eased_scroll_offset
// DESC: Advances a logical scroll position toward a bounded target with a short decelerating step.
// ------------------=
pub fn eased_scroll_offset(current: usize, target: usize, maximum: usize) -> usize {
    let current = current.min(maximum);
    let target = target.min(maximum);
    if current == target {
        return current;
    }
    let distance = current.abs_diff(target);
    let step = (distance / 4).clamp(1, 24);
    if current < target {
        current.saturating_add(step).min(target)
    } else {
        current.saturating_sub(step).max(target)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SystemMenuTarget {
    Item(usize),
    Dismiss,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsTarget {
    Section(usize),
    ContentRow(usize),
    ExpandedAction,
    ScrollPage(bool),
    ScrollThumb,
    Title,
    Resize(usize),
    WindowControl(usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NetworkSettingsGeometry {
    pub tabs: [Rect; 7],
    pub summary: Rect,
    pub main: Rect,
    pub sidebar: Rect,
    pub controls: [Rect; 6],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NetworkSettingsTarget {
    Page(usize),
    Control(usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SettingsWindowState {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub maximized: bool,
    pub expanded_row: Option<usize>,
    pub scroll_offset: usize,
    pub control_focus: usize,
    pub row_count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SettingsWindowGeometry {
    pub window: Rect,
    pub title: Rect,
    pub navigation: Rect,
    pub content: Rect,
    pub viewport: Rect,
    pub scrollbar_track: Rect,
    pub scrollbar_thumb: Rect,
    pub total_content_height: usize,
    pub maximum_scroll: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SettingsRowGeometry {
    pub summary: Rect,
    pub detail: Rect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SettingsSliderGeometry {
    pub track: Rect,
    pub thumb: Rect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsAccentTarget {
    Spectrum { saturation: u8, value: u8 },
    Hue(u16),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SettingsAccentGeometry {
    pub spectrum: Rect,
    pub hue: Rect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SystemLayout {
    width: usize,
    height: usize,
    scale: usize,
}

impl SystemLayout {
    // ------------------------=
    // FUNC: file_navigator_menu_geometry
    // DESC: Returns the shared app-local drop-down bounds for one File Navigator menu.
    // ------------------=
    pub fn file_navigator_menu_geometry(
        self,
        browser_left: usize,
        browser_top: usize,
        menu: usize,
        item_count: usize,
    ) -> Rect {
        let (offset, width) = match menu {
            1 => (156usize, 190usize),
            2 => (207, 232),
            3 => (318, 190),
            4 => (369, 224),
            _ => (454, 196),
        };
        rect(
            browser_left + offset * self.scale,
            browser_top + 32 * self.scale,
            width * self.scale,
            (12 + item_count * 30) * self.scale,
        )
    }

    // ------------------------=
    // FUNC: file_navigator_dialog_geometry
    // DESC: Centers a bounded modal surface within the active File Navigator window.
    // ------------------=
    pub fn file_navigator_dialog_geometry(
        self,
        browser_left: usize,
        browser_top: usize,
        browser_width: usize,
        browser_height: usize,
    ) -> Rect {
        let width = (460 * self.scale).min(browser_width.saturating_sub(32 * self.scale));
        let height = (218 * self.scale).min(browser_height.saturating_sub(32 * self.scale));
        rect(
            browser_left + browser_width.saturating_sub(width) / 2,
            browser_top + browser_height.saturating_sub(height) / 2,
            width,
            height,
        )
    }

    // ------------------------=
    // FUNC: file_navigator_overlay_target
    // DESC: Resolves app menu labels, open menu rows, and modal actions before underlying window controls.
    // ------------------=
    pub fn file_navigator_overlay_target(
        self,
        normalized_x: i32,
        normalized_y: i32,
        window_x: i32,
        window_y: i32,
        window_width: i32,
        window_height: i32,
        window_maximized: bool,
        menu_open: usize,
        menu_items: usize,
        dialog_open: usize,
    ) -> Option<DesktopTarget> {
        let point = self.point(normalized_x, normalized_y);
        let (left, top, width, height) = self.home_window_geometry_sized(
            window_x,
            window_y,
            window_width,
            window_height,
            window_maximized,
        );
        if dialog_open != 0 {
            let dialog = self.file_navigator_dialog_geometry(left, top, width, height);
            if dialog_open == 4 {
                let field = rect(
                    dialog.x.max(0) as usize + 24 * self.scale,
                    dialog.y.max(0) as usize + 82 * self.scale,
                    dialog.width as usize - 48 * self.scale,
                    38 * self.scale,
                );
                if field.contains(point) {
                    return Some(DesktopTarget::HomeDialogAction(2));
                }
            }
            let button_top = dialog.bottom().saturating_sub((54 * self.scale) as i32);
            if matches!(dialog_open, 1 | 4) {
                let half = dialog.width as usize / 2;
                let cancel = rect(
                    dialog.x.max(0) as usize + 20 * self.scale,
                    button_top.max(0) as usize,
                    half.saturating_sub(26 * self.scale),
                    38 * self.scale,
                );
                let primary = rect(
                    dialog.x.max(0) as usize + half + 6 * self.scale,
                    button_top.max(0) as usize,
                    half.saturating_sub(26 * self.scale),
                    38 * self.scale,
                );
                if cancel.contains(point) {
                    return Some(DesktopTarget::HomeDialogAction(1));
                }
                if primary.contains(point) {
                    return Some(DesktopTarget::HomeDialogAction(0));
                }
            } else {
                let close = rect(
                    dialog.x.max(0) as usize + dialog.width as usize / 2 - 74 * self.scale,
                    button_top.max(0) as usize,
                    148 * self.scale,
                    38 * self.scale,
                );
                if close.contains(point) {
                    return Some(DesktopTarget::HomeDialogAction(1));
                }
            }
            return dialog.contains(point).then_some(DesktopTarget::HomeContent);
        }
        if menu_open != 0 {
            let menu = self.file_navigator_menu_geometry(left, top, menu_open, menu_items);
            if menu.contains(point) {
                let row_top = menu.y + (6 * self.scale) as i32;
                if point.y >= row_top {
                    let row = (point.y - row_top) as usize / (30 * self.scale).max(1);
                    if row < menu_items {
                        return Some(DesktopTarget::HomeMenuItem(row));
                    }
                }
                return Some(DesktopTarget::HomeContent);
            }
        }
        for (index, (offset, width)) in [
            (156usize, 46usize),
            (207, 106),
            (318, 46),
            (369, 80),
            (454, 46),
        ]
        .iter()
        .enumerate()
        {
            if rect(
                left + offset * self.scale,
                top + 4 * self.scale,
                width * self.scale,
                27 * self.scale,
            )
            .contains(point)
            {
                return Some(DesktopTarget::HomeMenu(index + 1));
            }
        }
        None
    }
    // ------------------------=
    // FUNC: new
    // DESC: Creates resolution-aware system UI hit geometry for one framebuffer.
    // ------------------=
    pub const fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            scale: if width >= 2560 && height >= 1440 {
                2
            } else {
                1
            },
        }
    }

    // ------------------------=
    // FUNC: scale
    // DESC: Exposes the integer UI scale used by shared framebuffer geometry.
    // ------------------=
    pub const fn scale(self) -> usize {
        self.scale
    }

    // ------------------------=
    // FUNC: point
    // DESC: Converts a normalized device position into a clipped framebuffer point.
    // ------------------=
    pub fn point(self, normalized_x: i32, normalized_y: i32) -> Point {
        Point {
            x: (self.width as i32 * normalized_x.clamp(0, 1000) / 1000),
            y: (self.height as i32 * normalized_y.clamp(0, 1000) / 1000),
        }
    }

    // ------------------------=
    // FUNC: top_bar_height
    // DESC: Returns the exact shared system top-bar height used by the renderer.
    // ------------------=
    pub fn top_bar_height(self) -> usize {
        (38 * self.scale).min(self.height / 14).max(34 * self.scale)
    }

    // ------------------------=
    // FUNC: desktop_foreground_geometry
    // DESC: Returns the shared right-widget and dock bounds used to preserve desktop chrome during window motion.
    // ------------------=
    pub(crate) fn desktop_foreground_geometry(self) -> DesktopForegroundGeometry {
        let widget_left = self.width * 76 / 100;
        let widget_width = self.width * 22 / 100;
        let overview_top = self.height * 7 / 100;
        let overview_height = (330 * self.scale).min(self.height * 30 / 100);
        let ai_top = overview_top + overview_height + 20 * self.scale;
        let dock_width = self.width * 54 / 100;
        let dock_height = 72 * self.scale;
        let dock_top = self.height.saturating_sub(dock_height + 10 * self.scale);
        let ai_height = dock_top.saturating_sub(ai_top + 20 * self.scale);
        DesktopForegroundGeometry {
            widgets: rect(
                widget_left,
                overview_top,
                widget_width,
                ai_top + ai_height - overview_top,
            ),
            dock: rect(
                self.width.saturating_sub(dock_width) / 2,
                dock_top,
                dock_width,
                dock_height,
            ),
        }
    }

    // ------------------------=
    // FUNC: ai_chat_geometry
    // DESC: Returns the shared responsive geometry for the right-side desktop AI chat surface.
    // ------------------=
    pub fn ai_chat_geometry(self, minimized: bool) -> AiChatGeometry {
        let foreground = self.desktop_foreground_geometry();
        let overview_height = (330 * self.scale).min(self.height * 30 / 100);
        let left = foreground.widgets.x.max(0) as usize;
        let width = foreground.widgets.width as usize;
        let top = foreground.widgets.y.max(0) as usize + overview_height + 20 * self.scale;
        let available_height = foreground
            .widgets
            .bottom()
            .saturating_sub(top as i32)
            .max(0) as usize;
        let height = if minimized {
            50 * self.scale
        } else {
            available_height
        };
        let header_height = 46 * self.scale;
        let model_top = top + header_height + 8 * self.scale;
        let model_height = 42 * self.scale;
        let composer_height = 46 * self.scale;
        let send_width = 70 * self.scale;
        let composer_top = top + height.saturating_sub(composer_height + 12 * self.scale);
        AiChatGeometry {
            panel: rect(left, top, width, height),
            header: rect(left, top, width, header_height),
            model: rect(
                left + 12 * self.scale,
                model_top,
                width.saturating_sub(24 * self.scale),
                model_height,
            ),
            timeline: rect(
                left + 12 * self.scale,
                model_top + model_height + 10 * self.scale,
                width.saturating_sub(24 * self.scale),
                composer_top.saturating_sub(model_top + model_height + 18 * self.scale),
            ),
            composer: rect(
                left + 12 * self.scale,
                composer_top,
                width.saturating_sub(send_width + 30 * self.scale),
                composer_height,
            ),
            send: rect(
                left + width.saturating_sub(send_width + 12 * self.scale),
                composer_top,
                send_width,
                composer_height,
            ),
            minimize: rect(
                left + width.saturating_sub(58 * self.scale),
                top + 10 * self.scale,
                20 * self.scale,
                20 * self.scale,
            ),
            close: rect(
                left + width.saturating_sub(30 * self.scale),
                top + 10 * self.scale,
                20 * self.scale,
                20 * self.scale,
            ),
        }
    }

    // ------------------------=
    // FUNC: ai_chat_target
    // DESC: Resolves pointer input against the exact rendered AI chat controls.
    // ------------------=
    pub fn ai_chat_target(
        self,
        normalized_x: i32,
        normalized_y: i32,
        minimized: bool,
    ) -> Option<AiChatTarget> {
        let point = self.point(normalized_x, normalized_y);
        let geometry = self.ai_chat_geometry(minimized);
        if geometry.close.contains(point) {
            return Some(AiChatTarget::Close);
        }
        if geometry.minimize.contains(point) {
            return Some(AiChatTarget::Minimize);
        }
        if minimized || !geometry.panel.contains(point) {
            return None;
        }
        if geometry.model.contains(point) {
            return Some(AiChatTarget::Model);
        }
        if geometry.composer.contains(point) {
            return Some(AiChatTarget::Composer);
        }
        if geometry.send.contains(point) {
            return Some(AiChatTarget::Send);
        }
        geometry
            .timeline
            .contains(point)
            .then_some(AiChatTarget::Timeline)
    }

    // ------------------------=
    // FUNC: file_navigator_context_geometry
    // DESC: Returns the exact clamped pixel bounds used by the native File Navigator context menu.
    // ------------------=
    pub fn file_navigator_context_geometry(self, normalized_x: i32, normalized_y: i32) -> Rect {
        let left = (self.width * normalized_x.max(0) as usize / 1000)
            .min(self.width.saturating_sub(210 * self.scale));
        let top = (self.height * normalized_y.max(0) as usize / 1000)
            .min(self.height.saturating_sub(150 * self.scale));
        rect(left, top, 190 * self.scale, 120 * self.scale)
    }

    // ------------------------=
    // FUNC: file_navigator_context_action
    // DESC: Resolves a context-menu click from the same scaled and clamped pixel geometry used for rendering.
    // ------------------=
    pub fn file_navigator_context_action(
        self,
        context_x: i32,
        context_y: i32,
        pointer_x: i32,
        pointer_y: i32,
    ) -> Option<usize> {
        let menu = self.file_navigator_context_geometry(context_x, context_y);
        let point = self.point(pointer_x, pointer_y);
        if !menu.contains(point) {
            return None;
        }
        let row_top = menu.y + (6 * self.scale) as i32;
        if point.y < row_top {
            return None;
        }
        let row = (point.y - row_top) as usize / (28 * self.scale).max(1);
        (row < 4).then_some(row)
    }

    // ------------------------=
    // FUNC: desktop_foreground_layers_for_rect
    // DESC: Reports which persistent desktop chrome layers intersect a damaged framebuffer region.
    // ------------------=
    pub(crate) fn desktop_foreground_layers_for_rect(self, damage: Rect) -> u8 {
        let geometry = self.desktop_foreground_geometry();
        let mut layers = 0;
        if geometry.widgets.intersects(damage) {
            layers |= DESKTOP_FOREGROUND_WIDGETS;
        }
        if geometry.dock.intersects(damage) {
            layers |= DESKTOP_FOREGROUND_DOCK;
        }
        layers
    }

    // ------------------------=
    // FUNC: onboarding_target
    // DESC: Resolves visible onboarding fields and actions at any supported resolution.
    // ------------------=
    pub fn onboarding_target(
        self,
        step: usize,
        normalized_x: i32,
        normalized_y: i32,
    ) -> Option<OnboardingTarget> {
        let point = self.point(normalized_x, normalized_y);
        let authored_card = self.onboarding_template_rect(step, InstallerTemplateRole::Console);
        if step != 6 {
            if let Some(field) = self.onboarding_template_rect(step, InstallerTemplateRole::Input) {
                if field.contains(point) {
                    return Some(OnboardingTarget::Input);
                }
            }
        }
        if step == 6 && authored_card.is_some() {
            return (0..3)
                .find_map(|index| {
                    let row = crate::ui::installer_layout::configuration_network_row_rect(
                        index,
                        self.width,
                        self.height,
                    )?;
                    rect(row.left, row.top, row.width, row.height)
                        .contains(point)
                        .then_some(OnboardingTarget::NetworkChoice(index))
                })
                .or_else(|| {
                    [
                        (InstallerTemplateRole::BackButton, OnboardingTarget::Back),
                        (
                            InstallerTemplateRole::PrimaryButton,
                            OnboardingTarget::Primary,
                        ),
                    ]
                    .into_iter()
                    .find_map(|(role, target)| {
                        self.onboarding_template_rect(step, role)
                            .filter(|frame| frame.contains(point))
                            .map(|_| target)
                    })
                });
        }
        if step > 0 {
            if let Some(back) =
                self.onboarding_template_rect(step, InstallerTemplateRole::BackButton)
            {
                if back.contains(point) {
                    return Some(OnboardingTarget::Back);
                }
            }
        }
        if let Some(primary) =
            self.onboarding_template_rect(step, InstallerTemplateRole::PrimaryButton)
        {
            if primary.contains(point) {
                return Some(OnboardingTarget::Primary);
            }
        }
        let top_bar = self.top_bar_height();
        let fallback_card_width = (self.width * 34 / 100).clamp(500, 600 * self.scale);
        let fallback_card_height = (self.height * 68 / 100)
            .clamp(560, 680 * self.scale)
            .min(self.height.saturating_sub(top_bar + 24));
        let card_width = authored_card
            .map(|frame| frame.width.max(0) as usize)
            .unwrap_or(fallback_card_width);
        let card_height = authored_card
            .map(|frame| frame.height.max(0) as usize)
            .unwrap_or(fallback_card_height);
        let card_left = authored_card
            .map(|frame| frame.x.max(0) as usize)
            .unwrap_or(self.width * 4 / 100);
        let card_top = authored_card
            .map(|frame| frame.y.max(0) as usize)
            .unwrap_or(top_bar + self.height.saturating_sub(top_bar + card_height) / 2);
        let inner_left = card_left + 32 * self.scale;
        let inner_width = card_width.saturating_sub(64 * self.scale);

        if authored_card.is_none() && (1..=4).contains(&step) {
            let field_top = card_top + (94 + 132 + 28) * self.scale;
            if rect(inner_left, field_top, inner_width, 50 * self.scale).contains(point) {
                return Some(OnboardingTarget::Input);
            }
        }
        if step == 6 {
            let body_top = card_top + (94 + 132) * self.scale;
            for index in 0..3usize {
                if rect(
                    inner_left,
                    body_top + index * 58 * self.scale,
                    inner_width,
                    48 * self.scale,
                )
                .contains(point)
                {
                    return Some(OnboardingTarget::NetworkChoice(index));
                }
            }
        }

        if authored_card.is_some() {
            return None;
        }

        let button_top = card_top + card_height.saturating_sub(72 * self.scale);
        if step == 0 {
            return rect(
                inner_left,
                button_top,
                inner_width,
                UI_STANDARD_ACTION_HEIGHT * self.scale,
            )
            .contains(point)
            .then_some(OnboardingTarget::Primary);
        }
        let back_width = inner_width * 30 / 100;
        if rect(
            inner_left,
            button_top,
            back_width,
            UI_STANDARD_ACTION_HEIGHT * self.scale,
        )
        .contains(point)
        {
            return Some(OnboardingTarget::Back);
        }
        let primary_left = inner_left + back_width + UI_CONTROL_GAP * self.scale;
        rect(
            primary_left,
            button_top,
            inner_width.saturating_sub(back_width + UI_CONTROL_GAP * self.scale),
            UI_STANDARD_ACTION_HEIGHT * self.scale,
        )
        .contains(point)
        .then_some(OnboardingTarget::Primary)
    }

    // ------------------------=
    // FUNC: onboarding_input_geometry
    // DESC: Returns the exact editable first-boot field rectangle for click-to-caret placement.
    // ------------------=
    pub fn onboarding_input_geometry(self, step: usize) -> Option<Rect> {
        if !(1..=4).contains(&step) {
            return None;
        }
        if let Some(input) = self.onboarding_template_rect(step, InstallerTemplateRole::Input) {
            return Some(input);
        }
        let top_bar = self.top_bar_height();
        let card_width = (self.width * 34 / 100).clamp(500, 600 * self.scale);
        let card_height = (self.height * 68 / 100)
            .clamp(560, 680 * self.scale)
            .min(self.height.saturating_sub(top_bar + 24));
        let card_left = self.width * 4 / 100;
        let card_top = top_bar + self.height.saturating_sub(top_bar + card_height) / 2;
        Some(rect(
            card_left + 32 * self.scale,
            card_top + (94 + 132 + 28) * self.scale,
            card_width.saturating_sub(64 * self.scale),
            50 * self.scale,
        ))
    }

    // ------------------------=
    // FUNC: onboarding_template_rect
    // DESC: Resolves one saved OS configuration element into shared framebuffer hit geometry.
    // ------------------=
    fn onboarding_template_rect(self, step: usize, role: InstallerTemplateRole) -> Option<Rect> {
        let authored = crate::ui::installer_layout::configuration_template_rect(
            step,
            role,
            self.width,
            self.height,
        )?;
        Some(rect(
            authored.left,
            authored.top,
            authored.width,
            authored.height,
        ))
    }

    // ------------------------=
    // FUNC: authentication_target
    // DESC: Resolves account, credential, recovery, and session utility controls exactly.
    // ------------------=
    pub fn authentication_target(self, normalized_x: i32, normalized_y: i32) -> Option<usize> {
        let point = self.point(normalized_x, normalized_y);
        let fit = (self.width.saturating_mul(1000) / 1536)
            .min(self.height.saturating_mul(1000) / 1024)
            .max(1);
        let content_width = 1536usize.saturating_mul(fit) / 1000;
        let content_height = 1024usize.saturating_mul(fit) / 1000;
        let offset_x = self.width.saturating_sub(content_width) / 2;
        let offset_y = self.height.saturating_sub(content_height) / 2;
        let sx = |value: usize| offset_x + value.saturating_mul(fit) / 1000;
        let sy = |value: usize| offset_y + value.saturating_mul(fit) / 1000;
        let sw = |value: usize| value.saturating_mul(fit) / 1000;
        let card_x = sx(54);
        let card_y = sy(123);
        let card_w = sw(521);
        let inner_x = card_x + sw(49);
        let inner_w = card_w.saturating_sub(sw(98));

        for (focus, top, height) in [
            (0usize, card_y + sw(253), sw(86)),
            (1, card_y + sw(356), sw(57)),
            (2, card_y + sw(439), sw(UI_HERO_ACTION_HEIGHT)),
            (3, card_y + sw(557), sw(UI_HERO_ACTION_HEIGHT)),
        ] {
            if rect(inner_x, top, inner_w, height).contains(point) {
                return Some(focus);
            }
        }

        let actions_y = card_y + sw(628);
        let action_height = sw(92);
        let third = inner_w / 3;
        for index in 0..3usize {
            if rect(inner_x + third * index, actions_y, third, action_height).contains(point) {
                return Some(index + 4);
            }
        }

        let tray_x = sx(529);
        let tray_y = sy(914);
        let tray_w = sw(478);
        let cell = tray_w / 4;
        for index in 0..5usize {
            if rect(tray_x + cell * index, tray_y, cell, sw(90)).contains(point) {
                return Some(index + 7);
            }
        }
        None
    }

    // ------------------------=
    // FUNC: authentication_password_geometry
    // DESC: Returns the exact secure text field rectangle for click-to-caret placement.
    // ------------------=
    pub fn authentication_password_geometry(self) -> Rect {
        let fit = (self.width.saturating_mul(1000) / 1536)
            .min(self.height.saturating_mul(1000) / 1024)
            .max(1);
        let content_width = 1536usize.saturating_mul(fit) / 1000;
        let content_height = 1024usize.saturating_mul(fit) / 1000;
        let offset_x = self.width.saturating_sub(content_width) / 2;
        let offset_y = self.height.saturating_sub(content_height) / 2;
        let card_x = offset_x + 54usize.saturating_mul(fit) / 1000;
        let card_y = offset_y + 123usize.saturating_mul(fit) / 1000;
        let card_w = 521usize.saturating_mul(fit) / 1000;
        rect(
            card_x + 49usize.saturating_mul(fit) / 1000,
            card_y + 356usize.saturating_mul(fit) / 1000,
            card_w.saturating_sub(98usize.saturating_mul(fit) / 1000),
            57usize.saturating_mul(fit) / 1000,
        )
    }

    // ------------------------=
    // FUNC: desktop_target
    // DESC: Resolves every functional top-bar control, draggable Home title, and application dock.
    // ------------------=
    pub fn desktop_target(
        self,
        normalized_x: i32,
        normalized_y: i32,
        window_x: i32,
        window_y: i32,
        window_visible: bool,
        window_maximized: bool,
    ) -> Option<DesktopTarget> {
        self.desktop_target_sized(
            normalized_x,
            normalized_y,
            window_x,
            window_y,
            430,
            380,
            window_visible,
            window_maximized,
        )
    }

    // ------------------------=
    // FUNC: desktop_target_sized
    // DESC: Resolves desktop controls against the live resizable Home window bounds.
    // ------------------=
    pub fn desktop_target_sized(
        self,
        normalized_x: i32,
        normalized_y: i32,
        window_x: i32,
        window_y: i32,
        window_width: i32,
        window_height: i32,
        window_visible: bool,
        window_maximized: bool,
    ) -> Option<DesktopTarget> {
        let point = self.point(normalized_x, normalized_y);
        let top_bar = self.top_bar_height();
        let brand_width = (280 * self.scale).min(self.width / 3);
        if rect(
            8 * self.scale,
            4 * self.scale,
            brand_width,
            top_bar.saturating_sub(8 * self.scale),
        )
        .contains(point)
        {
            return Some(DesktopTarget::InfinityMenu);
        }
        if rect(
            610 * self.scale,
            4 * self.scale,
            62 * self.scale,
            top_bar.saturating_sub(8 * self.scale),
        )
        .contains(point)
        {
            return Some(DesktopTarget::TopMenu(5));
        }
        let status_width = 32 * self.scale;
        let clock_width = 104 * self.scale;
        let status_left = self
            .width
            .saturating_sub(7 * status_width + clock_width + 10 * self.scale);
        for index in 0..7usize {
            if rect(
                status_left + index * status_width,
                4 * self.scale,
                status_width,
                top_bar.saturating_sub(8 * self.scale),
            )
            .contains(point)
            {
                return Some(DesktopTarget::Status(index));
            }
        }
        if rect(
            self.width.saturating_sub(clock_width + 8 * self.scale),
            4 * self.scale,
            clock_width,
            top_bar.saturating_sub(8 * self.scale),
        )
        .contains(point)
        {
            return Some(DesktopTarget::Status(7));
        }

        if window_visible {
            let (browser_left, browser_top, browser_width, browser_height) = self
                .home_window_geometry_sized(
                    window_x,
                    window_y,
                    window_width,
                    window_height,
                    window_maximized,
                );
            let title_height = 34 * self.scale;
            for index in 0..3usize {
                let control_left = browser_left
                    + browser_width.saturating_sub((28 + (2 - index) * 27) * self.scale);
                if rect(
                    control_left,
                    browser_top + 7 * self.scale,
                    20 * self.scale,
                    20 * self.scale,
                )
                .contains(point)
                {
                    return Some(DesktopTarget::HomeControl(index));
                }
            }
            if !window_maximized {
                if let Some(handle) = native_window_resize_target(
                    rect(browser_left, browser_top, browser_width, browser_height),
                    point,
                    self.scale,
                ) {
                    return Some(DesktopTarget::HomeResize(handle));
                }
            }
            if rect(browser_left, browser_top, browser_width, title_height).contains(point) {
                return Some(DesktopTarget::HomeTitle);
            }
            let tool_top = browser_top + title_height;
            for index in 0..3usize {
                if rect(
                    browser_left + (7 + index * 28) * self.scale,
                    tool_top + 4 * self.scale,
                    26 * self.scale,
                    30 * self.scale,
                )
                .contains(point)
                {
                    return Some(DesktopTarget::HomeToolbar(index));
                }
            }
            let location_left = browser_left + 100 * self.scale;
            let mode_controls_width = 96 * self.scale;
            let location_width =
                browser_width.saturating_sub(154 * self.scale + mode_controls_width);
            if rect(
                location_left,
                tool_top + 4 * self.scale,
                location_width,
                30 * self.scale,
            )
            .contains(point)
            {
                return Some(DesktopTarget::HomeLocation);
            }
            for index in 0..2usize {
                let control_left =
                    browser_left + browser_width.saturating_sub((100 - index * 46) * self.scale);
                let control_width = 42 * self.scale;
                if rect(
                    control_left,
                    tool_top + 4 * self.scale,
                    control_width,
                    30 * self.scale,
                )
                .contains(point)
                {
                    return Some(DesktopTarget::HomeToolbar(index + 3));
                }
            }
            let sidebar_width = browser_width * 27 / 100;
            for index in 0..9usize {
                let item_y = tool_top + (72 + index * 20) * self.scale;
                if rect(
                    browser_left + 7,
                    item_y.saturating_sub(3),
                    sidebar_width.saturating_sub(14),
                    20 * self.scale,
                )
                .contains(point)
                {
                    return Some(DesktopTarget::HomeSidebar(index));
                }
            }
            let grid_x = browser_left + sidebar_width + 28 * self.scale;
            let grid_y = tool_top + 58 * self.scale;
            let gap = (browser_width.saturating_sub(sidebar_width + 55 * self.scale)) / 4;
            let tile_step = (self.height / 23).max(34) + 40 * self.scale;
            for index in 0..7usize {
                let column = index % 4;
                let row = index / 4;
                if rect(
                    grid_x + column * gap.saturating_sub(6 * self.scale),
                    grid_y + row * tile_step.saturating_sub(8 * self.scale),
                    gap.max(44 * self.scale),
                    tile_step.max(54 * self.scale),
                )
                .contains(point)
                {
                    return Some(DesktopTarget::HomeItem(index));
                }
            }
            if rect(
                browser_left + sidebar_width,
                tool_top + 38 * self.scale,
                browser_width.saturating_sub(sidebar_width),
                browser_height.saturating_sub(title_height + 38 * self.scale),
            )
            .contains(point)
            {
                return Some(DesktopTarget::HomeContent);
            }
        }

        let dock_width = self.width * 54 / 100;
        let dock_height = 72 * self.scale;
        let dock_left = self.width.saturating_sub(dock_width) / 2;
        let dock_top = self.height.saturating_sub(dock_height + 10 * self.scale);
        if rect(dock_left, dock_top, dock_width, dock_height).contains(point) {
            let icon_gap = dock_width / super::app_launcher::DESKTOP_DOCK_ENTRIES.len();
            let relative = point.x.saturating_sub(dock_left as i32) as usize;
            return Some(DesktopTarget::Dock(
                (relative / icon_gap).min(super::app_launcher::DESKTOP_DOCK_ENTRIES.len() - 1),
            ));
        }
        None
    }

    // ------------------------=
    // FUNC: app_launcher_geometry
    // DESC: Derives the adaptive launcher panel, search, grid, and category geometry above the live dock.
    // ------------------=
    pub fn app_launcher_geometry(self) -> AppLauncherGeometry {
        let top_bar = self.top_bar_height();
        let dock_top = self.height.saturating_sub(82 * self.scale);
        let panel_width = (self.width * 86 / 100)
            .min(self.width.saturating_sub(32 * self.scale))
            .max(1);
        let available_height = dock_top.saturating_sub(top_bar + 24 * self.scale).max(1);
        let height_percent = if self.height.saturating_mul(4) >= self.width.saturating_mul(3) {
            84
        } else {
            72
        };
        let panel_height = (self.height * height_percent / 100)
            .min(available_height)
            .max(1);
        let panel_left = self.width.saturating_sub(panel_width) / 2;
        let final_panel_top = dock_top
            .saturating_sub(14 * self.scale)
            .saturating_sub(panel_height);
        let panel_top = final_panel_top;
        let panel = rect(panel_left, panel_top, panel_width, panel_height);
        let search_width = panel_width * 62 / 100;
        let search_height = (50 * self.scale)
            .min(panel_height / 10)
            .max(30 * self.scale);
        let search = rect(
            panel_left + panel_width.saturating_sub(search_width) / 2,
            panel_top + panel_height * 6 / 100,
            search_width,
            search_height,
        );
        let inset = (42 * self.scale).min(panel_width / 16);
        let inner_width = panel_width.saturating_sub(inset * 2);
        let grid_top = panel_top + panel_height * 21 / 100;
        let grid_row_height = panel_height * 22 / 100;
        let grid_bottom = panel_top + panel_height * 69 / 100;
        let scrollbar_width = (7 * self.scale).max(5);
        let category_left = panel_left + inset;
        let category_width = inner_width / 5;
        AppLauncherGeometry {
            panel,
            search,
            close: rect(
                panel_left + panel_width.saturating_sub(42 * self.scale),
                panel_top + 12 * self.scale,
                24 * self.scale,
                24 * self.scale,
            ),
            grid_left: panel_left + inset,
            grid_top,
            grid_cell_width: inner_width.saturating_sub(20 * self.scale) / 6,
            grid_row_height,
            grid_viewport: rect(
                panel_left + inset,
                grid_top,
                inner_width.saturating_sub(20 * self.scale),
                grid_bottom.saturating_sub(grid_top),
            ),
            scrollbar_track: rect(
                panel_left + panel_width.saturating_sub(inset + scrollbar_width),
                grid_top,
                scrollbar_width,
                grid_bottom.saturating_sub(grid_top),
            ),
            category_left,
            category_top: panel_top + panel_height * 76 / 100,
            category_width,
            category_height: panel_height * 16 / 100,
        }
    }

    // ------------------------=
    // FUNC: app_launcher_visible_region_for_progress
    // DESC: Returns the bottom-anchored portion of the fixed launcher surface revealed at one animation progress value.
    // ------------------=
    pub fn app_launcher_visible_region_for_progress(self, progress: u8) -> Rect {
        let panel = self.app_launcher_geometry().panel;
        let visible_height = (panel.height as usize).saturating_mul(usize::from(progress)) / 255;
        rect(
            panel.x.max(0) as usize,
            panel.bottom().max(panel.y) as usize - visible_height,
            panel.width as usize,
            visible_height,
        )
    }

    // ------------------------=
    // FUNC: app_launcher_visible_region
    // DESC: Returns the currently revealed fixed launcher surface for clipped composition and hit testing.
    // ------------------=
    pub fn app_launcher_visible_region(self) -> Rect {
        self.app_launcher_visible_region_for_progress(
            super::app_launcher::launcher_presentation().transition,
        )
    }

    // ------------------------=
    // FUNC: app_launcher_transition_damage
    // DESC: Bounds one launcher reveal step to only the changed horizontal strip plus shadow padding.
    // ------------------=
    pub fn app_launcher_transition_damage(
        self,
        previous_progress: u8,
        current_progress: u8,
        padding: u32,
    ) -> Rect {
        let previous = self.app_launcher_visible_region_for_progress(previous_progress);
        let current = self.app_launcher_visible_region_for_progress(current_progress);
        let upper = previous.y.min(current.y).max(0) as usize;
        let lower = previous.y.max(current.y).max(0) as usize;
        let panel = self.app_launcher_geometry().panel;
        let left = (panel.x - padding as i32).max(0) as usize;
        let top = upper.saturating_sub(padding as usize);
        let right = (panel.right() + padding as i32)
            .max(0)
            .min(self.width as i32) as usize;
        let bottom = lower.saturating_add(padding as usize).min(self.height);
        rect(
            left,
            top,
            right.saturating_sub(left),
            bottom.saturating_sub(top),
        )
    }

    // ------------------------=
    // FUNC: app_launcher_scroll_geometry
    // DESC: Computes bounded application content and proportional smooth-scroll thumb geometry.
    // ------------------=
    pub fn app_launcher_scroll_geometry(self, visible_apps: usize) -> AppLauncherScrollGeometry {
        let geometry = self.app_launcher_geometry();
        let rows = visible_apps.saturating_add(super::app_launcher::LAUNCHER_COLUMNS - 1)
            / super::app_launcher::LAUNCHER_COLUMNS;
        let content_height = rows.saturating_mul(geometry.grid_row_height);
        let viewport_height = geometry.grid_viewport.height as usize;
        let maximum_scroll = content_height.saturating_sub(viewport_height);
        let presentation = super::app_launcher::launcher_presentation();
        let thumb_height = if maximum_scroll == 0 {
            geometry.scrollbar_track.height as usize
        } else {
            viewport_height
                .saturating_mul(viewport_height)
                .checked_div(content_height.max(1))
                .unwrap_or(viewport_height)
                .max(32 * self.scale)
                .min(viewport_height)
        };
        let travel = viewport_height.saturating_sub(thumb_height);
        let thumb_offset = if maximum_scroll == 0 {
            0
        } else {
            presentation.scroll.min(maximum_scroll) * travel / maximum_scroll
        };
        AppLauncherScrollGeometry {
            maximum_scroll,
            thumb: rect(
                geometry.scrollbar_track.x.max(0) as usize,
                geometry.scrollbar_track.y.max(0) as usize + thumb_offset,
                geometry.scrollbar_track.width as usize,
                thumb_height,
            ),
        }
    }

    // ------------------------=
    // FUNC: app_launcher_target
    // DESC: Hit-tests the live launcher surface and dedicated dock toggle against shared rendered geometry.
    // ------------------=
    pub fn app_launcher_target(
        self,
        normalized_x: i32,
        normalized_y: i32,
        visible_apps: usize,
    ) -> AppLauncherTarget {
        let point = self.point(normalized_x, normalized_y);
        let dock_width = self.width * 54 / 100;
        let dock_height = 72 * self.scale;
        let dock_left = self.width.saturating_sub(dock_width) / 2;
        let dock_top = self.height.saturating_sub(dock_height + 10 * self.scale);
        let icon_gap = dock_width / super::app_launcher::DESKTOP_DOCK_ENTRIES.len();
        if rect(dock_left, dock_top, icon_gap, dock_height).contains(point) {
            return AppLauncherTarget::DockToggle;
        }
        let geometry = self.app_launcher_geometry();
        if !geometry.panel.contains(point) {
            return AppLauncherTarget::Dismiss;
        }
        let close_padding = 8 * self.scale;
        let close_target = rect(
            (geometry.close.x.max(0) as usize).saturating_sub(close_padding),
            (geometry.close.y.max(0) as usize).saturating_sub(close_padding),
            geometry.close.width as usize + close_padding * 2,
            geometry.close.height as usize + close_padding * 2,
        );
        if close_target.contains(point) {
            return AppLauncherTarget::Close;
        }
        if geometry.search.contains(point) {
            return AppLauncherTarget::Search;
        }
        let scroll = self.app_launcher_scroll_geometry(visible_apps);
        if scroll.maximum_scroll != 0 && scroll.thumb.contains(point) {
            return AppLauncherTarget::ScrollbarThumb;
        }
        if scroll.maximum_scroll != 0 && geometry.scrollbar_track.contains(point) {
            return AppLauncherTarget::ScrollbarTrack;
        }
        let scroll_offset = super::app_launcher::launcher_presentation()
            .scroll
            .min(scroll.maximum_scroll) as i32;
        for index in 0..visible_apps {
            let column = index % super::app_launcher::LAUNCHER_COLUMNS;
            let row = index / super::app_launcher::LAUNCHER_COLUMNS;
            let cell = Rect {
                x: (geometry.grid_left + column * geometry.grid_cell_width) as i32,
                y: geometry.grid_top as i32 + (row * geometry.grid_row_height) as i32
                    - scroll_offset,
                width: geometry.grid_cell_width as u32,
                height: geometry.grid_row_height as u32,
            };
            if geometry.grid_viewport.contains(point) && cell.contains(point) {
                return AppLauncherTarget::App(index);
            }
        }
        for index in 0..5usize {
            if rect(
                geometry.category_left + index * geometry.category_width,
                geometry.category_top,
                geometry.category_width,
                geometry.category_height,
            )
            .contains(point)
            {
                return AppLauncherTarget::Category(index);
            }
        }
        AppLauncherTarget::Panel
    }

    // ------------------------=
    // FUNC: desktop_app_window_geometry
    // DESC: Derives shared native Text Editor and Command Window bounds from normalized window state.
    // ------------------=
    pub fn desktop_app_window_geometry(
        self,
        window_x: i32,
        window_y: i32,
        window_width: i32,
        window_height: i32,
        maximized: bool,
    ) -> DesktopAppWindowGeometry {
        let (left, top, width, height) = if maximized {
            let inset = 10 * self.scale;
            let top = self.top_bar_height() + inset;
            (
                inset,
                top,
                self.width.saturating_sub(inset * 2),
                self.height.saturating_sub(top + 92 * self.scale),
            )
        } else {
            (
                self.width * window_x.clamp(0, 900) as usize / 1000,
                self.height * window_y.clamp(50, 850) as usize / 1000,
                (self.width * window_width.clamp(420, 900) as usize / 1000).min(self.width),
                (self.height * window_height.clamp(360, 820) as usize / 1000).min(self.height),
            )
        };
        let title_height = 48 * self.scale;
        let control_size = 26 * self.scale;
        let control_gap = 8 * self.scale;
        let close_left = left + width.saturating_sub(control_size + 12 * self.scale);
        let maximize_left = close_left.saturating_sub(control_size + control_gap);
        let minimize_left = maximize_left.saturating_sub(control_size + control_gap);
        let toolbar_top = top + title_height;
        let toolbar_height = 42 * self.scale;
        DesktopAppWindowGeometry {
            window: rect(left, top, width, height),
            title: rect(
                left + 12 * self.scale,
                top,
                minimize_left.saturating_sub(left + 20 * self.scale),
                title_height,
            ),
            minimize: rect(
                minimize_left,
                top + 11 * self.scale,
                control_size,
                control_size,
            ),
            maximize: rect(
                maximize_left,
                top + 11 * self.scale,
                control_size,
                control_size,
            ),
            close: rect(
                close_left,
                top + 11 * self.scale,
                control_size,
                control_size,
            ),
            toolbar: rect(left, toolbar_top, width, toolbar_height),
            content: rect(
                left + 16 * self.scale,
                toolbar_top + toolbar_height + 12 * self.scale,
                width.saturating_sub(32 * self.scale),
                height.saturating_sub(title_height + toolbar_height + 28 * self.scale),
            ),
        }
    }

    // ------------------------=
    // FUNC: desktop_toolbar_geometry
    // DESC: Derives five equal toolbar actions and a protected trailing status well from shared control metrics.
    // ------------------=
    pub fn desktop_toolbar_geometry(
        self,
        window: DesktopAppWindowGeometry,
    ) -> DesktopToolbarGeometry {
        let toolbar_left = window.toolbar.x.max(0) as usize;
        let toolbar_top = window.toolbar.y.max(0) as usize;
        let toolbar_width = window.toolbar.width as usize;
        let outer_gutter = 8 * self.scale;
        let gap = 4 * self.scale;
        let status_width = (112 * self.scale).min(toolbar_width / 4);
        let available = toolbar_width
            .saturating_sub(outer_gutter * 2 + status_width + 8 * self.scale + gap * 4);
        let action_width = (104 * self.scale).min(available / 5).max(1);
        let action_height = UI_TOOLBAR_ACTION_HEIGHT * self.scale;
        let action_top =
            toolbar_top + (window.toolbar.height as usize).saturating_sub(action_height) / 2;
        let mut actions = [rect(0, 0, 0, 0); 5];
        let mut index = 0usize;
        while index < actions.len() {
            actions[index] = rect(
                toolbar_left + outer_gutter + index * (action_width + gap),
                action_top,
                action_width,
                action_height,
            );
            index += 1;
        }
        DesktopToolbarGeometry {
            actions,
            status: rect(
                toolbar_left + toolbar_width.saturating_sub(outer_gutter + status_width),
                toolbar_top,
                status_width,
                window.toolbar.height as usize,
            ),
        }
    }

    // ------------------------=
    // FUNC: task_manager_process_row
    // DESC: Resolves only real process rows; title chrome and summary space never select a task.
    // ------------------=
    pub fn task_manager_process_row(
        self,
        pointer_x: i32,
        pointer_y: i32,
        geometry: DesktopAppWindowGeometry,
    ) -> Option<usize> {
        let point = self.point(pointer_x, pointer_y);
        if !geometry.content.contains(point) {
            return None;
        }
        let offset = point.y - geometry.content.y - 154 * self.scale as i32;
        if offset < 0 {
            return None;
        }
        let row = offset as usize / (38 * self.scale.max(1));
        (row < 5).then_some(row)
    }

    // ------------------------=
    // FUNC: desktop_app_window_target
    // DESC: Resolves native app title, controls, toolbar actions, and content using rendered geometry.
    // ------------------=
    pub fn desktop_app_window_target(
        self,
        normalized_x: i32,
        normalized_y: i32,
        window_x: i32,
        window_y: i32,
        window_width: i32,
        window_height: i32,
        maximized: bool,
        text_editor: bool,
    ) -> DesktopAppWindowTarget {
        let point = self.point(normalized_x, normalized_y);
        let geometry = self.desktop_app_window_geometry(
            window_x,
            window_y,
            window_width,
            window_height,
            maximized,
        );
        let padding = 6 * self.scale;
        for (target, control) in [
            (DesktopAppWindowTarget::Close, geometry.close),
            (DesktopAppWindowTarget::Maximize, geometry.maximize),
            (DesktopAppWindowTarget::Minimize, geometry.minimize),
        ] {
            if rect(
                (control.x.max(0) as usize).saturating_sub(padding),
                (control.y.max(0) as usize).saturating_sub(padding),
                control.width as usize + padding * 2,
                control.height as usize + padding * 2,
            )
            .contains(point)
            {
                return target;
            }
        }
        if !maximized {
            if let Some(handle) = native_window_resize_target(geometry.window, point, self.scale) {
                return DesktopAppWindowTarget::Resize(handle);
            }
        }
        if geometry.title.contains(point) {
            return DesktopAppWindowTarget::Title;
        }
        if text_editor && geometry.toolbar.contains(point) {
            let toolbar = self.desktop_toolbar_geometry(geometry);
            for (index, action) in toolbar.actions.iter().enumerate() {
                if action.contains(point) {
                    return [
                        DesktopAppWindowTarget::NewDocument,
                        DesktopAppWindowTarget::OpenDocument,
                        DesktopAppWindowTarget::SaveDocument,
                        DesktopAppWindowTarget::SaveAsDocument,
                        DesktopAppWindowTarget::DeleteDocument,
                    ][index];
                }
            }
        }
        if geometry.content.contains(point) {
            return DesktopAppWindowTarget::Content;
        }
        DesktopAppWindowTarget::None
    }

    // ------------------------=
    // FUNC: desktop_editor_scroll_geometry
    // DESC: Derives a proportional editor scrollbar from visual rows and the resizable content viewport.
    // ------------------=
    pub fn desktop_editor_scroll_geometry(
        self,
        window_x: i32,
        window_y: i32,
        window_width: i32,
        window_height: i32,
        maximized: bool,
        visual_rows: usize,
        scroll_row: usize,
    ) -> EditorScrollGeometry {
        let window = self.desktop_app_window_geometry(
            window_x,
            window_y,
            window_width,
            window_height,
            maximized,
        );
        let visible_rows = (window.content.height as usize / (24 * self.scale).max(1)).max(1);
        let maximum_scroll = visual_rows.saturating_sub(visible_rows);
        let track_height = (window.content.height as usize).saturating_sub(16 * self.scale);
        let track = rect(
            window
                .content
                .right()
                .saturating_sub((14 * self.scale) as i32)
                .max(0) as usize,
            window.content.y.max(0) as usize + 8 * self.scale,
            8 * self.scale,
            track_height,
        );
        let thumb_height = if maximum_scroll == 0 {
            0
        } else {
            (track_height.saturating_mul(visible_rows) / visual_rows.max(1))
                .max(28 * self.scale)
                .min(track_height)
        };
        let travel = track_height.saturating_sub(thumb_height);
        let thumb_top = track.y.max(0) as usize
            + if maximum_scroll == 0 {
                0
            } else {
                travel.saturating_mul(scroll_row.min(maximum_scroll)) / maximum_scroll
            };
        EditorScrollGeometry {
            track,
            thumb: rect(
                track.x.max(0) as usize,
                thumb_top,
                track.width as usize,
                thumb_height,
            ),
            maximum_scroll,
        }
    }

    // ------------------------=
    // FUNC: desktop_editor_scroll_target
    // DESC: Hit-tests the complete editor scrollbar with a forgiving pointer target.
    // ------------------=
    pub fn desktop_editor_scroll_target(
        self,
        normalized_x: i32,
        normalized_y: i32,
        geometry: EditorScrollGeometry,
    ) -> Option<EditorScrollTarget> {
        if geometry.maximum_scroll == 0 {
            return None;
        }
        let point = self.point(normalized_x, normalized_y);
        let hit_track = rect(
            geometry
                .track
                .x
                .saturating_sub((5 * self.scale) as i32)
                .max(0) as usize,
            geometry.track.y.max(0) as usize,
            geometry.track.width as usize + 10 * self.scale,
            geometry.track.height as usize,
        );
        if geometry.thumb.contains(point) {
            Some(EditorScrollTarget::Thumb)
        } else if hit_track.contains(point) {
            Some(EditorScrollTarget::Page(point.y >= geometry.thumb.y))
        } else {
            None
        }
    }

    // ------------------------=
    // FUNC: desktop_editor_scroll_offset_for_thumb
    // DESC: Converts editor thumb movement into a bounded visual-row offset.
    // ------------------=
    pub fn desktop_editor_scroll_offset_for_thumb(
        self,
        normalized_y: i32,
        geometry: EditorScrollGeometry,
        grab_offset: i32,
    ) -> usize {
        if geometry.maximum_scroll == 0 {
            return 0;
        }
        let pointer_y = self.height as i32 * normalized_y.clamp(0, 1000) / 1000;
        let travel = geometry
            .track
            .height
            .saturating_sub(geometry.thumb.height)
            .max(1);
        let thumb_y = pointer_y
            .saturating_sub(grab_offset)
            .saturating_sub(geometry.track.y)
            .clamp(0, travel as i32) as usize;
        thumb_y.saturating_mul(geometry.maximum_scroll) / travel as usize
    }

    // ------------------------=
    // FUNC: desktop_editor_dialog_target
    // DESC: Resolves mouse actions inside the modal Save As and Open object sheets from shared window geometry.
    // ------------------=
    pub fn desktop_editor_dialog_target(
        self,
        normalized_x: i32,
        normalized_y: i32,
        window_x: i32,
        window_y: i32,
        window_width: i32,
        window_height: i32,
        maximized: bool,
        open_picker: bool,
        row_count: usize,
    ) -> Option<EditorDialogTarget> {
        let point = self.point(normalized_x, normalized_y);
        let window = self.desktop_app_window_geometry(
            window_x,
            window_y,
            window_width,
            window_height,
            maximized,
        );
        let sheet_width =
            (420 * self.scale).min((window.content.width as usize).saturating_sub(40 * self.scale));
        let sheet_height = if open_picker {
            330 * self.scale
        } else {
            220 * self.scale
        };
        let left = window.content.x.max(0) as usize
            + (window.content.width as usize).saturating_sub(sheet_width) / 2;
        let top = window.content.y.max(0) as usize
            + (window.content.height as usize).saturating_sub(sheet_height) / 2;
        if open_picker {
            for index in 0..row_count.min(6) {
                if rect(
                    left + 24 * self.scale,
                    top + (62 + index * 34) * self.scale,
                    sheet_width.saturating_sub(48 * self.scale),
                    30 * self.scale,
                )
                .contains(point)
                {
                    return Some(EditorDialogTarget::Row(index));
                }
            }
        } else if rect(
            left + 24 * self.scale,
            top + 72 * self.scale,
            sheet_width.saturating_sub(48 * self.scale),
            46 * self.scale,
        )
        .contains(point)
        {
            return Some(EditorDialogTarget::NameField);
        }
        let button_top = top + sheet_height.saturating_sub(60 * self.scale);
        let button_width = (sheet_width.saturating_sub(60 * self.scale)) / 2;
        if rect(
            left + 24 * self.scale,
            button_top,
            button_width,
            UI_COMPACT_ACTION_HEIGHT * self.scale,
        )
        .contains(point)
        {
            return Some(EditorDialogTarget::Cancel);
        }
        if rect(
            left + (24 + UI_CONTROL_GAP) * self.scale + button_width,
            button_top,
            button_width,
            UI_COMPACT_ACTION_HEIGHT * self.scale,
        )
        .contains(point)
        {
            return Some(EditorDialogTarget::Accept);
        }
        None
    }

    // ------------------------=
    // FUNC: home_window_geometry
    // DESC: Returns the shared restored or maximized Home window geometry.
    // ------------------=
    pub fn home_window_geometry(
        self,
        window_x: i32,
        window_y: i32,
        maximized: bool,
    ) -> (usize, usize, usize, usize) {
        self.home_window_geometry_sized(window_x, window_y, 430, 380, maximized)
    }

    // ------------------------=
    // FUNC: home_window_geometry_sized
    // DESC: Converts the live normalized Home window position and dimensions into framebuffer bounds.
    // ------------------=
    pub fn home_window_geometry_sized(
        self,
        window_x: i32,
        window_y: i32,
        window_width: i32,
        window_height: i32,
        maximized: bool,
    ) -> (usize, usize, usize, usize) {
        if maximized {
            let left = 10 * self.scale;
            let top = self.top_bar_height() + 10 * self.scale;
            let bottom = self.height.saturating_sub(90 * self.scale);
            return (
                left,
                top,
                self.width.saturating_sub(left * 2),
                bottom.saturating_sub(top),
            );
        }
        (
            self.width * window_x.clamp(0, 900) as usize / 1000,
            self.height * window_y.clamp(50, 900) as usize / 1000,
            (self.width * window_width.clamp(300, 900) as usize / 1000).min(self.width),
            (self.height * window_height.clamp(260, 820) as usize / 1000).min(self.height),
        )
    }

    // ------------------------=
    // FUNC: system_menu_geometry
    // DESC: Returns the shared top-menu origin, bounds, and row count for the selected native menu.
    // ------------------=
    pub fn system_menu_geometry(self, menu_kind: usize) -> (usize, usize, usize, usize, usize) {
        if menu_kind >= 8 {
            let width = (310 * self.scale).min(self.width.saturating_sub(16 * self.scale));
            let x = self.width.saturating_sub(width + 8 * self.scale);
            let count = super::status_menu::items(menu_kind).len();
            let height = (22 + count * 34 + if menu_kind >= 15 { 238 } else { 0 }) * self.scale;
            return (
                x,
                self.top_bar_height() + 6 * self.scale,
                width,
                height,
                count,
            );
        }
        let (anchor, width, count) = match menu_kind {
            1 => (298usize, 248usize, 5usize),
            2 => (358, 230, 6),
            3 => (418, 238, 5),
            4 => (480, 242, 4),
            5 => (608, 252, 4),
            _ => (16, 268, 10),
        };
        let x = (anchor * self.scale).min(self.width.saturating_sub(width * self.scale + 8));
        let y = self.top_bar_height() + 6 * self.scale;
        let menu_width = (width * self.scale).min(self.width.saturating_sub(x + 8));
        let menu_height = (22 + count * 34) * self.scale;
        (x, y, menu_width, menu_height, count)
    }

    // ------------------------=
    // FUNC: system_menu_damage_geometry
    // DESC: Returns the complete saved region for a menu including its bounded glass shadow.
    // ------------------=
    pub fn system_menu_damage_geometry(self, menu_kind: usize) -> (usize, usize, usize, usize) {
        let (x, y, width, height, _) = self.system_menu_geometry(menu_kind);
        let shadow = 10 * self.scale;
        (
            x,
            y,
            width
                .saturating_add(shadow)
                .min(self.width.saturating_sub(x)),
            height
                .saturating_add(shadow)
                .min(self.height.saturating_sub(y)),
        )
    }

    // ------------------------=
    // FUNC: system_menu_target
    // DESC: Resolves menu rows from the active native menu geometry and identifies outside dismissal.
    // ------------------=
    pub fn system_menu_target(
        self,
        menu_kind: usize,
        normalized_x: i32,
        normalized_y: i32,
    ) -> SystemMenuTarget {
        let point = self.point(normalized_x, normalized_y);
        let (menu_x, menu_y, menu_w, menu_h, count) = self.system_menu_geometry(menu_kind);
        if !rect(menu_x, menu_y, menu_w, menu_h).contains(point) {
            return SystemMenuTarget::Dismiss;
        }
        for index in 0..count {
            let row_y = menu_y + (11 + index * 34) * self.scale;
            if rect(
                menu_x + 7 * self.scale,
                row_y,
                menu_w.saturating_sub(14 * self.scale),
                30 * self.scale,
            )
            .contains(point)
            {
                return SystemMenuTarget::Item(index);
            }
        }
        SystemMenuTarget::Dismiss
    }

    // ------------------------=
    // FUNC: settings_window_geometry
    // DESC: Derives the complete resizable Settings window, scroll viewport, and proportional scrollbar geometry.
    // ------------------=
    pub fn settings_window_geometry(self, state: SettingsWindowState) -> SettingsWindowGeometry {
        self.settings_window_geometry_for_section(state, usize::MAX)
    }

    // ------------------------=
    // FUNC: settings_window_geometry_for_section
    // DESC: Derives Settings chrome with section-specific intrinsic content height for accurate overflow behavior.
    // ------------------=
    pub fn settings_window_geometry_for_section(
        self,
        state: SettingsWindowState,
        section: usize,
    ) -> SettingsWindowGeometry {
        let top_bar = self.top_bar_height();
        let (left, top, width, height) = if state.maximized {
            let inset = 10 * self.scale;
            (
                inset,
                top_bar + inset,
                self.width.saturating_sub(inset * 2),
                self.height.saturating_sub(top_bar + inset * 2),
            )
        } else {
            (
                self.width * state.x.clamp(0, 850) as usize / 1000,
                self.height * state.y.clamp(50, 850) as usize / 1000,
                (self.width * state.width.clamp(600, 950) as usize / 1000).min(self.width),
                (self.height * state.height.clamp(420, 900) as usize / 1000).min(self.height),
            )
        };
        let title_height = 54 * self.scale;
        let navigation_width = width * 28 / 100;
        let content_left = left + navigation_width + 34 * self.scale;
        let scrollbar_width = 10 * self.scale;
        let content_right_padding = 30 * self.scale;
        let content_width = width.saturating_sub(navigation_width + 68 * self.scale);
        let viewport_top = top + title_height + 98 * self.scale;
        let viewport_height = height.saturating_sub(title_height + 116 * self.scale);
        let total_content_height =
            if matches!(section, SETTINGS_NETWORK_SECTION | SETTINGS_NODE_SECTION) {
                SETTINGS_DASHBOARD_CONTENT_HEIGHT
            } else {
                let detail_height = state.expanded_row.map(settings_detail_height).unwrap_or(0);
                state.row_count.clamp(1, 8) * 58 + detail_height
            };
        let visible_logical_height = viewport_height / self.scale.max(1);
        let maximum_scroll = total_content_height.saturating_sub(visible_logical_height);
        let track = rect(
            left + width.saturating_sub(22 * self.scale),
            viewport_top,
            scrollbar_width,
            viewport_height,
        );
        let thumb_height = if maximum_scroll == 0 {
            0
        } else {
            (viewport_height * visible_logical_height / total_content_height.max(1))
                .max(34 * self.scale)
                .min(viewport_height)
        };
        let clamped_scroll = state.scroll_offset.min(maximum_scroll);
        let thumb_top = if maximum_scroll == 0 {
            viewport_top
        } else {
            viewport_top
                + viewport_height.saturating_sub(thumb_height) * clamped_scroll / maximum_scroll
        };
        SettingsWindowGeometry {
            window: rect(left, top, width, height),
            title: rect(
                left + 12 * self.scale,
                top,
                width.saturating_sub(150 * self.scale),
                title_height,
            ),
            navigation: rect(
                left,
                top + title_height,
                navigation_width,
                height.saturating_sub(title_height),
            ),
            content: rect(
                content_left,
                top + title_height + 29 * self.scale,
                content_width.saturating_sub(content_right_padding),
                height.saturating_sub(title_height + 47 * self.scale),
            ),
            viewport: rect(
                content_left,
                viewport_top,
                content_width.saturating_sub(content_right_padding),
                viewport_height,
            ),
            scrollbar_track: track,
            scrollbar_thumb: rect(
                track.x.max(0) as usize,
                thumb_top,
                scrollbar_width,
                thumb_height,
            ),
            total_content_height,
            maximum_scroll,
        }
    }

    // ------------------------=
    // FUNC: settings_row_geometry
    // DESC: Returns one summary row and its inline detail well after applying the shared scroll offset.
    // ------------------=
    pub fn settings_row_geometry(
        self,
        state: SettingsWindowState,
        index: usize,
    ) -> SettingsRowGeometry {
        let window = self.settings_window_geometry(state);
        let prior_detail = state
            .expanded_row
            .filter(|expanded| *expanded < index)
            .map(settings_detail_height)
            .unwrap_or(0);
        let summary_top = window.viewport.y + ((index * 58 + prior_detail) * self.scale) as i32
            - (state.scroll_offset.min(window.maximum_scroll) * self.scale) as i32;
        let detail_height = if state.expanded_row == Some(index) {
            settings_detail_height(index) * self.scale
        } else {
            0
        };
        SettingsRowGeometry {
            summary: rect(
                window.viewport.x.max(0) as usize,
                summary_top.max(0) as usize,
                window.viewport.width as usize,
                46 * self.scale,
            ),
            detail: rect(
                window.viewport.x.max(0) as usize,
                summary_top.saturating_add((50 * self.scale) as i32).max(0) as usize,
                window.viewport.width as usize,
                detail_height,
            ),
        }
    }

    // ------------------------=
    // FUNC: settings_section_geometry
    // DESC: Fits all Settings sections inside the navigation viewport while preserving generous icon and pointer space.
    // ------------------=
    pub fn settings_section_geometry(self, state: SettingsWindowState, index: usize) -> Rect {
        let geometry = self.settings_window_geometry(state);
        let navigation = geometry.navigation;
        let inset = 10 * self.scale;
        let usable_height = (navigation.height as usize).saturating_sub(inset * 2);
        let row_height = (usable_height / 11).min(43 * self.scale).max(1);
        rect(
            navigation.x.max(0) as usize + inset,
            navigation.y.max(0) as usize + inset + index.min(10) * row_height,
            (navigation.width as usize).saturating_sub(inset * 2),
            row_height,
        )
    }

    // ------------------------=
    // FUNC: settings_target
    // DESC: Resolves Settings navigation, accordion, scrolling, chrome, and resize affordances from shared geometry.
    // ------------------=
    pub fn settings_target(
        self,
        normalized_x: i32,
        normalized_y: i32,
        state: SettingsWindowState,
    ) -> Option<SettingsTarget> {
        self.settings_target_for_section(normalized_x, normalized_y, state, usize::MAX)
    }

    // ------------------------=
    // FUNC: settings_target_for_section
    // DESC: Hit-tests Settings chrome and overflow using the active section's intrinsic content height.
    // ------------------=
    pub fn settings_target_for_section(
        self,
        normalized_x: i32,
        normalized_y: i32,
        state: SettingsWindowState,
        section: usize,
    ) -> Option<SettingsTarget> {
        let point = self.point(normalized_x, normalized_y);
        let geometry = self.settings_window_geometry_for_section(state, section);
        let left = geometry.window.x.max(0) as usize;
        let top = geometry.window.y.max(0) as usize;
        let width = geometry.window.width as usize;
        for index in 0..3usize {
            let control_left = left + width.saturating_sub((26 + (2 - index) * 25) * self.scale);
            if rect(
                control_left,
                top + 12 * self.scale,
                20 * self.scale,
                24 * self.scale,
            )
            .contains(point)
            {
                return Some(SettingsTarget::WindowControl(index));
            }
        }
        if !state.maximized {
            if let Some(handle) = native_window_resize_target(geometry.window, point, self.scale) {
                return Some(SettingsTarget::Resize(handle));
            }
        }
        if geometry.title.contains(point) {
            return Some(SettingsTarget::Title);
        }
        for index in 0..11usize {
            if self.settings_section_geometry(state, index).contains(point) {
                return Some(SettingsTarget::Section(index));
            }
        }
        let scroll_hit = rect(
            geometry
                .scrollbar_track
                .x
                .saturating_sub((5 * self.scale) as i32)
                .max(0) as usize,
            geometry.scrollbar_track.y.max(0) as usize,
            geometry.scrollbar_track.width as usize + 10 * self.scale,
            geometry.scrollbar_track.height as usize,
        );
        if geometry.maximum_scroll > 0 && geometry.scrollbar_thumb.contains(point) {
            return Some(SettingsTarget::ScrollThumb);
        }
        if geometry.maximum_scroll > 0 && scroll_hit.contains(point) {
            return Some(SettingsTarget::ScrollPage(
                point.y >= geometry.scrollbar_thumb.y,
            ));
        }
        if !geometry.viewport.contains(point) {
            return None;
        }
        for index in 0..state.row_count.clamp(1, 8) {
            let row = self.settings_row_geometry(state, index);
            if row.summary.contains(point) {
                return Some(SettingsTarget::ContentRow(index));
            }
            if state.expanded_row == Some(index) && row.detail.contains(point) {
                let action = rect(
                    row.detail.x.max(0) as usize + UI_GUTTER * self.scale,
                    row.detail
                        .bottom()
                        .saturating_sub(((UI_COMPACT_ACTION_HEIGHT + 10) * self.scale) as i32)
                        .max(0) as usize,
                    (170 * self.scale).min(
                        (row.detail.width as usize).saturating_sub(UI_GUTTER * 2 * self.scale),
                    ),
                    UI_COMPACT_ACTION_HEIGHT * self.scale,
                );
                if action.contains(point) {
                    return Some(SettingsTarget::ExpandedAction);
                }
            }
        }
        None
    }

    // ------------------------=
    // FUNC: network_settings_geometry
    // DESC: Derives a bounded responsive network editor with page navigation and non-overlapping controls.
    // ------------------=
    pub fn network_settings_geometry(self, state: SettingsWindowState) -> NetworkSettingsGeometry {
        self.paged_settings_geometry(state, 7, SETTINGS_NETWORK_SECTION)
    }

    // ------------------------=
    // FUNC: node_settings_geometry
    // DESC: Derives the five-page Nodes and Mesh dashboard with label-safe wrapped tabs.
    // ------------------=
    pub fn node_settings_geometry(self, state: SettingsWindowState) -> NetworkSettingsGeometry {
        self.paged_settings_geometry(state, 5, SETTINGS_NODE_SECTION)
    }

    // ------------------------=
    // FUNC: paged_settings_geometry
    // DESC: Builds one readable scrolling dashboard with wrapped tabs and fixed-height action cards.
    // ------------------=
    fn paged_settings_geometry(
        self,
        state: SettingsWindowState,
        page_count: usize,
        section: usize,
    ) -> NetworkSettingsGeometry {
        let window = self.settings_window_geometry_for_section(state, section);
        let content = window.content;
        let gap = 12 * self.scale;
        let top = content.y.max(0) as usize + 68 * self.scale;
        let left = content.x.max(0) as usize;
        let width = content.width as usize;
        let tab_gap = 6 * self.scale;
        let minimum_tab_width = 112 * self.scale;
        let columns =
            ((width + tab_gap) / (minimum_tab_width + tab_gap)).clamp(1, page_count.max(1));
        let tab_width = width.saturating_sub(tab_gap * columns.saturating_sub(1)) / columns;
        let tab_height = 38 * self.scale;
        let tab_rows = (page_count + columns - 1) / columns;
        let mut tabs = [rect(0, 0, 0, 0); 7];
        for (index, tab) in tabs.iter_mut().take(page_count).enumerate() {
            let row = index / columns;
            let column = index % columns;
            *tab = rect(
                left + column * (tab_width + tab_gap),
                top + row * (tab_height + tab_gap),
                tab_width,
                tab_height,
            );
        }
        let summary_top =
            top + tab_rows * tab_height + tab_rows.saturating_sub(1) * tab_gap + 12 * self.scale;
        let summary_height = 88 * self.scale;
        let body_top = summary_top + summary_height + gap;
        let preferred_body_height = 420 * self.scale;
        let available_body_height =
            content.bottom().saturating_sub(body_top as i32).max(0) as usize;
        let body_height = if available_body_height >= 120 * self.scale {
            preferred_body_height.min(available_body_height)
        } else {
            preferred_body_height
        };
        let main_width = width * 68 / 100;
        let main = rect(
            left,
            body_top,
            main_width.saturating_sub(gap / 2),
            body_height,
        );
        let sidebar = rect(
            left + main_width + gap / 2,
            body_top,
            width.saturating_sub(main_width + gap / 2),
            body_height,
        );
        let control_gap = 8 * self.scale;
        let control_height = if body_height < preferred_body_height {
            body_height
                .saturating_sub(24 * self.scale)
                .saturating_sub(control_gap * 5)
                / 6
        } else {
            58 * self.scale
        };
        let mut controls = [rect(0, 0, 0, 0); 6];
        for (index, control) in controls.iter_mut().enumerate() {
            *control = rect(
                main.x.max(0) as usize + 12 * self.scale,
                main.y.max(0) as usize + 12 * self.scale + index * (control_height + control_gap),
                (main.width as usize).saturating_sub(24 * self.scale),
                control_height,
            );
        }
        let scroll = (state.scroll_offset.min(window.maximum_scroll) * self.scale) as i32;
        for tab in tabs.iter_mut() {
            tab.y = tab.y.saturating_sub(scroll);
        }
        let mut summary = rect(left, summary_top, width, summary_height);
        let mut main = main;
        let mut sidebar = sidebar;
        summary.y = summary.y.saturating_sub(scroll);
        main.y = main.y.saturating_sub(scroll);
        sidebar.y = sidebar.y.saturating_sub(scroll);
        for control in controls.iter_mut() {
            control.y = control.y.saturating_sub(scroll);
        }
        NetworkSettingsGeometry {
            tabs,
            summary,
            main,
            sidebar,
            controls,
        }
    }

    // ------------------------=
    // FUNC: network_settings_target
    // DESC: Resolves network editor page tabs and page-local controls from shared geometry.
    // ------------------=
    pub fn network_settings_target(
        self,
        normalized_x: i32,
        normalized_y: i32,
        state: SettingsWindowState,
    ) -> Option<NetworkSettingsTarget> {
        self.paged_settings_target(
            normalized_x,
            normalized_y,
            state,
            7,
            SETTINGS_NETWORK_SECTION,
        )
    }

    // ------------------------=
    // FUNC: node_settings_target
    // DESC: Resolves Nodes and Mesh tabs and controls from the five-page dashboard geometry.
    // ------------------=
    pub fn node_settings_target(
        self,
        normalized_x: i32,
        normalized_y: i32,
        state: SettingsWindowState,
    ) -> Option<NetworkSettingsTarget> {
        self.paged_settings_target(normalized_x, normalized_y, state, 5, SETTINGS_NODE_SECTION)
    }

    // ------------------------=
    // FUNC: paged_settings_target
    // DESC: Hit-tests visible dashboard tabs and controls inside the clipped Settings viewport.
    // ------------------=
    fn paged_settings_target(
        self,
        normalized_x: i32,
        normalized_y: i32,
        state: SettingsWindowState,
        page_count: usize,
        section: usize,
    ) -> Option<NetworkSettingsTarget> {
        let point = self.point(normalized_x, normalized_y);
        if !self
            .settings_window_geometry_for_section(state, section)
            .viewport
            .contains(point)
        {
            return None;
        }
        let geometry = self.paged_settings_geometry(state, page_count, section);
        if let Some(index) = geometry
            .tabs
            .iter()
            .take(page_count)
            .position(|card| card.contains(point))
        {
            return Some(NetworkSettingsTarget::Page(index));
        }
        geometry
            .controls
            .iter()
            .position(|card| card.contains(point))
            .map(NetworkSettingsTarget::Control)
    }

    // ------------------------=
    // FUNC: network_profile_target
    // DESC: Resolves a pointer to one visible page control for compatibility with existing UI harnesses.
    // ------------------=
    pub fn network_profile_target(
        self,
        normalized_x: i32,
        normalized_y: i32,
        state: SettingsWindowState,
    ) -> Option<usize> {
        let point = self.point(normalized_x, normalized_y);
        self.network_settings_geometry(state)
            .controls
            .iter()
            .position(|card| card.contains(point))
    }

    // ------------------------=
    // FUNC: settings_scroll_offset_for_thumb
    // DESC: Converts a dragged scrollbar thumb position into a bounded logical content offset.
    // ------------------=
    pub fn settings_scroll_offset_for_thumb(
        self,
        normalized_y: i32,
        state: SettingsWindowState,
        grab_offset: i32,
    ) -> usize {
        self.settings_scroll_offset_for_thumb_in_section(
            normalized_y,
            state,
            grab_offset,
            usize::MAX,
        )
    }

    // ------------------------=
    // FUNC: settings_scroll_offset_for_thumb_in_section
    // DESC: Converts a Settings thumb drag using the active section's intrinsic overflow range.
    // ------------------=
    pub fn settings_scroll_offset_for_thumb_in_section(
        self,
        normalized_y: i32,
        state: SettingsWindowState,
        grab_offset: i32,
        section: usize,
    ) -> usize {
        let geometry = self.settings_window_geometry_for_section(state, section);
        if geometry.maximum_scroll == 0 {
            return 0;
        }
        let pointer_y = self.height as i32 * normalized_y.clamp(0, 1000) / 1000;
        let travel = geometry
            .scrollbar_track
            .height
            .saturating_sub(geometry.scrollbar_thumb.height)
            .max(1);
        let thumb_y = pointer_y
            .saturating_sub(grab_offset)
            .saturating_sub(geometry.scrollbar_track.y)
            .clamp(0, travel as i32) as usize;
        thumb_y.saturating_mul(geometry.maximum_scroll) / travel as usize
    }

    // ------------------------=
    // FUNC: settings_icon_theme_target
    // DESC: Hit-tests every installed icon-family preview card inside the Icon Set detail well.
    // ------------------=
    pub fn settings_icon_theme_target(
        self,
        normalized_x: i32,
        normalized_y: i32,
        state: SettingsWindowState,
    ) -> Option<u8> {
        if state.expanded_row != Some(1) {
            return None;
        }
        let point = self.point(normalized_x, normalized_y);
        let row = self.settings_row_geometry(state, 1);
        let theme_count = super::icon_theme::ICON_THEME_COUNT as usize;
        let preview_gap = row.detail.width as usize / theme_count;
        for theme in 0..theme_count {
            if rect(
                row.detail.x.max(0) as usize + theme * preview_gap + 4 * self.scale,
                row.detail.y.max(0) as usize + 8 * self.scale,
                preview_gap.saturating_sub(8 * self.scale),
                (row.detail.height as usize).saturating_sub(16 * self.scale),
            )
            .contains(point)
            {
                return Some(theme as u8);
            }
        }
        None
    }

    // ------------------------=
    // FUNC: settings_accent_geometry
    // DESC: Returns the inline HSV picker geometry owned by the expanded Accent row.
    // ------------------=
    pub fn settings_accent_geometry(self, state: SettingsWindowState) -> SettingsAccentGeometry {
        self.settings_color_geometry(state, 3)
    }

    // ------------------------=
    // FUNC: settings_primary_geometry
    // DESC: Returns the inline HSV picker geometry owned by the expanded Primary row.
    // ------------------=
    pub fn settings_primary_geometry(self, state: SettingsWindowState) -> SettingsAccentGeometry {
        self.settings_color_geometry(state, 2)
    }

    // ------------------------=
    // FUNC: settings_color_geometry
    // DESC: Derives shared inline HSV geometry for one expanded color row.
    // ------------------=
    fn settings_color_geometry(
        self,
        state: SettingsWindowState,
        index: usize,
    ) -> SettingsAccentGeometry {
        let row = self.settings_row_geometry(state, index);
        let hue_width = 24 * self.scale;
        let gap = 14 * self.scale;
        SettingsAccentGeometry {
            spectrum: rect(
                row.detail.x.max(0) as usize + 12 * self.scale,
                row.detail.y.max(0) as usize + 12 * self.scale,
                (row.detail.width as usize).saturating_sub(hue_width + gap + 24 * self.scale),
                78 * self.scale,
            ),
            hue: rect(
                row.detail
                    .right()
                    .saturating_sub((hue_width + 12 * self.scale) as i32)
                    .max(0) as usize,
                row.detail.y.max(0) as usize + 12 * self.scale,
                hue_width,
                78 * self.scale,
            ),
        }
    }

    // ------------------------=
    // FUNC: settings_accent_target
    // DESC: Maps a pointer position to typed HSV picker coordinates without using rendered text.
    // ------------------=
    pub fn settings_accent_target(
        self,
        normalized_x: i32,
        normalized_y: i32,
        state: SettingsWindowState,
    ) -> Option<SettingsAccentTarget> {
        self.settings_color_target(normalized_x, normalized_y, state, 3)
    }

    // ------------------------=
    // FUNC: settings_primary_target
    // DESC: Maps a pointer position in the Primary picker to typed HSV coordinates.
    // ------------------=
    pub fn settings_primary_target(
        self,
        normalized_x: i32,
        normalized_y: i32,
        state: SettingsWindowState,
    ) -> Option<SettingsAccentTarget> {
        self.settings_color_target(normalized_x, normalized_y, state, 2)
    }

    // ------------------------=
    // FUNC: settings_effect_slider_geometry
    // DESC: Returns the shared inline track and thumb geometry for an expanded opacity or blur row.
    // ------------------=
    pub fn settings_effect_slider_geometry(
        self,
        state: SettingsWindowState,
        index: usize,
        value: u8,
        maximum: u8,
    ) -> SettingsSliderGeometry {
        let row = self.settings_row_geometry(state, index);
        let track = rect(
            row.detail.x.max(0) as usize + 22 * self.scale,
            row.detail.y.max(0) as usize + 35 * self.scale,
            (row.detail.width as usize).saturating_sub(44 * self.scale),
            8 * self.scale,
        );
        let thumb_size = 20 * self.scale;
        let travel = (track.width as usize).saturating_sub(thumb_size);
        let thumb_left = track.x.max(0) as usize
            + travel.saturating_mul(value.min(maximum) as usize) / maximum.max(1) as usize;
        SettingsSliderGeometry {
            track,
            thumb: rect(
                thumb_left,
                track.y.saturating_sub((6 * self.scale) as i32).max(0) as usize,
                thumb_size,
                thumb_size,
            ),
        }
    }

    // ------------------------=
    // FUNC: settings_effect_slider_target
    // DESC: Hit-tests an opacity or blur track and returns its bounded typed value.
    // ------------------=
    pub fn settings_effect_slider_target(
        self,
        normalized_x: i32,
        normalized_y: i32,
        state: SettingsWindowState,
        index: usize,
        maximum: u8,
    ) -> Option<u8> {
        if !matches!(index, 4 | 5) {
            return None;
        }
        self.settings_slider_target(normalized_x, normalized_y, state, index, maximum)
    }

    // ------------------------=
    // FUNC: settings_slider_target
    // DESC: Hit-tests a requested expanded Settings slider and returns its bounded typed value.
    // ------------------=
    pub fn settings_slider_target(
        self,
        normalized_x: i32,
        normalized_y: i32,
        state: SettingsWindowState,
        index: usize,
        maximum: u8,
    ) -> Option<u8> {
        if state.expanded_row != Some(index) {
            return None;
        }
        let point = self.point(normalized_x, normalized_y);
        let geometry = self.settings_effect_slider_geometry(state, index, 0, maximum);
        let row = self.settings_row_geometry(state, index);
        if !row.detail.contains(point)
            || point.y < geometry.thumb.y
            || point.y >= geometry.thumb.bottom()
        {
            return None;
        }
        Some(self.settings_slider_drag_value(normalized_x, state, index, maximum))
    }

    // ------------------------=
    // FUNC: settings_effect_slider_drag_value
    // DESC: Converts captured horizontal pointer motion into a bounded typed slider value.
    // ------------------=
    pub fn settings_effect_slider_drag_value(
        self,
        normalized_x: i32,
        state: SettingsWindowState,
        index: usize,
        maximum: u8,
    ) -> u8 {
        self.settings_slider_drag_value(normalized_x, state, index, maximum)
    }

    // ------------------------=
    // FUNC: settings_slider_drag_value
    // DESC: Converts captured horizontal pointer motion for any expanded Settings slider into a bounded value.
    // ------------------=
    pub fn settings_slider_drag_value(
        self,
        normalized_x: i32,
        state: SettingsWindowState,
        index: usize,
        maximum: u8,
    ) -> u8 {
        let point = self.point(normalized_x, 0);
        let geometry = self.settings_effect_slider_geometry(state, index, 0, maximum);
        let half_thumb = 10 * self.scale;
        let start = geometry.track.x + half_thumb as i32;
        let travel = geometry
            .track
            .width
            .saturating_sub((half_thumb * 2) as u32)
            .max(1);
        let offset = point.x.saturating_sub(start).clamp(0, travel as i32) as u32;
        ((offset.saturating_mul(maximum as u32) + travel / 2) / travel).min(maximum as u32) as u8
    }

    // ------------------------=
    // FUNC: settings_color_target
    // DESC: Maps one color row pointer position to typed HSV picker coordinates.
    // ------------------=
    fn settings_color_target(
        self,
        normalized_x: i32,
        normalized_y: i32,
        state: SettingsWindowState,
        index: usize,
    ) -> Option<SettingsAccentTarget> {
        if state.expanded_row != Some(index) {
            return None;
        }
        let point = self.point(normalized_x, normalized_y);
        let geometry = self.settings_color_geometry(state, index);
        if geometry.spectrum.contains(point) {
            let x = point.x.saturating_sub(geometry.spectrum.x) as u32;
            let y = point.y.saturating_sub(geometry.spectrum.y) as u32;
            let width = geometry.spectrum.width.saturating_sub(1).max(1);
            let height = geometry.spectrum.height.saturating_sub(1).max(1);
            return Some(SettingsAccentTarget::Spectrum {
                saturation: (x.saturating_mul(255) / width).min(255) as u8,
                value: 255u8.saturating_sub((y.saturating_mul(255) / height).min(255) as u8),
            });
        }
        if geometry.hue.contains(point) {
            let y = point.y.saturating_sub(geometry.hue.y) as u32;
            let height = geometry.hue.height.saturating_sub(1).max(1);
            return Some(SettingsAccentTarget::Hue(
                (y.saturating_mul(359) / height).min(359) as u16,
            ));
        }
        None
    }
}

// ------------------------=
// FUNC: settings_detail_height
// DESC: Returns the logical inline well height needed by each Settings row's real controls or explanation.
// ------------------=
const fn settings_detail_height(index: usize) -> usize {
    match index {
        1 => 108,
        2 | 3 => 112,
        4 | 5 => 78,
        _ => 82,
    }
}

// ------------------------=
// FUNC: native_window_resize_target
// DESC: Resolves four corner grips plus left, right, and bottom resize borders without making the top edge resizable.
// ------------------=
pub fn native_window_resize_target(window: Rect, point: Point, scale: usize) -> Option<usize> {
    let grip = (18 * scale.max(1)) as i32;
    let halo = (6 * scale.max(1)) as i32;
    let corner_size = (grip + halo * 2).max(1) as u32;
    for (handle, x, y) in [
        (0usize, window.x - halo, window.y - halo),
        (1, window.right() - grip - halo, window.y - halo),
        (2, window.x - halo, window.bottom() - grip - halo),
        (
            3,
            window.right() - grip - halo,
            window.bottom() - grip - halo,
        ),
    ] {
        if (Rect {
            x,
            y,
            width: corner_size,
            height: corner_size,
        })
        .contains(point)
        {
            return Some(handle);
        }
    }
    let side_top = window.y + grip;
    let side_height = window.height.saturating_sub((grip * 2).max(0) as u32);
    for (handle, x) in [(5usize, window.x - halo), (6, window.right() - halo)] {
        if (Rect {
            x,
            y: side_top,
            width: (halo * 2 + 1) as u32,
            height: side_height,
        })
        .contains(point)
        {
            return Some(handle);
        }
    }
    let bottom_left = window.x + grip;
    let bottom_width = window.width.saturating_sub((grip * 2).max(0) as u32);
    if (Rect {
        x: bottom_left,
        y: window.bottom() - halo,
        width: bottom_width,
        height: (halo * 2 + 1) as u32,
    })
    .contains(point)
    {
        return Some(4);
    }
    None
}

// ------------------------=
// FUNC: resize_home_window
// DESC: Applies shared border resizing while preserving minimum size and the visible work area.
// ------------------=
pub fn resize_home_window(
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    corner: usize,
    pointer_x: i32,
    pointer_y: i32,
) -> (i32, i32, i32, i32) {
    resize_native_window(x, y, width, height, corner, pointer_x, pointer_y, 300, 260)
}

// ------------------------=
// FUNC: resize_native_window
// DESC: Applies corner, side, or bottom-edge resizing while preserving the minimum usable area.
// ------------------=
pub fn resize_native_window(
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    corner: usize,
    pointer_x: i32,
    pointer_y: i32,
    minimum_width: i32,
    minimum_height: i32,
) -> (i32, i32, i32, i32) {
    let right = x.saturating_add(width);
    let bottom = y.saturating_add(height);
    let (next_x, next_width) = if matches!(corner, 0 | 2 | 5) {
        let next_x = pointer_x.clamp(0, right.saturating_sub(minimum_width));
        (next_x, right.saturating_sub(next_x))
    } else if matches!(corner, 1 | 3 | 6) {
        (
            x,
            pointer_x
                .saturating_sub(x)
                .clamp(minimum_width, 1000i32.saturating_sub(x)),
        )
    } else {
        (x, width)
    };
    let (next_y, next_height) = if matches!(corner, 0 | 1) {
        let next_y = pointer_y.clamp(50, bottom.saturating_sub(minimum_height));
        (next_y, bottom.saturating_sub(next_y))
    } else if matches!(corner, 2 | 3 | 4) {
        (
            y,
            pointer_y
                .saturating_sub(y)
                .clamp(minimum_height, 920i32.saturating_sub(y)),
        )
    } else {
        (y, height)
    };
    (next_x, next_y, next_width, next_height)
}

// ------------------------=
// FUNC: rect
// DESC: Constructs a safely bounded rectangle from framebuffer layout values.
// ------------------=
fn rect(left: usize, top: usize, width: usize, height: usize) -> Rect {
    Rect {
        x: left.min(i32::MAX as usize) as i32,
        y: top.min(i32::MAX as usize) as i32,
        width: width.min(u32::MAX as usize) as u32,
        height: height.min(u32::MAX as usize) as u32,
    }
}
