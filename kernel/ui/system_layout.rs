//! Shared pixel-exact hit geometry for the installed InfinityOS experience.
//!
//! The framebuffer renderer and input runtime both use physical pixels. This
//! module converts normalized device coordinates once, then derives hit regions
//! from the same formulas used by the polished onboarding, authentication,
//! desktop, menu, and Settings surfaces.

use super::geometry::{Point, Rect};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OnboardingTarget {
    Back,
    Primary,
    Input,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DesktopTarget {
    InfinityMenu,
    TopMenu(usize),
    Status(usize),
    HomeTitle,
    HomeControl(usize),
    HomeResize(usize),
    HomeToolbar(usize),
    HomeSidebar(usize),
    HomeItem(usize),
    Dock(usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppLauncherTarget {
    Search,
    App(usize),
    Category(usize),
    Close,
    DockToggle,
    Panel,
    Dismiss,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DesktopAppWindowTarget {
    Title,
    Minimize,
    Maximize,
    Close,
    NewDocument,
    SaveDocument,
    Content,
    None,
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
pub struct AppLauncherGeometry {
    pub panel: Rect,
    pub search: Rect,
    pub close: Rect,
    pub grid_left: usize,
    pub grid_top: usize,
    pub grid_cell_width: usize,
    pub grid_row_height: usize,
    pub category_left: usize,
    pub category_top: usize,
    pub category_width: usize,
    pub category_height: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DesktopForegroundGeometry {
    pub widgets: Rect,
    pub dock: Rect,
}

pub(crate) const DESKTOP_FOREGROUND_WIDGETS: u8 = 1;
pub(crate) const DESKTOP_FOREGROUND_DOCK: u8 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SystemMenuTarget {
    Item(usize),
    Dismiss,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsTarget {
    Section(usize),
    ContentRow(usize),
    WindowControl(usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SystemLayout {
    width: usize,
    height: usize,
    scale: usize,
}

impl SystemLayout {
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
        let ai_height = (250 * self.scale).min(self.height * 24 / 100);
        let dock_width = self.width * 54 / 100;
        let dock_height = 72 * self.scale;
        DesktopForegroundGeometry {
            widgets: rect(
                widget_left,
                overview_top,
                widget_width,
                ai_top + ai_height - overview_top,
            ),
            dock: rect(
                self.width.saturating_sub(dock_width) / 2,
                self.height.saturating_sub(dock_height + 10 * self.scale),
                dock_width,
                dock_height,
            ),
        }
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
        let top_bar = self.top_bar_height();
        let card_width = (self.width * 34 / 100).clamp(500, 600 * self.scale);
        let card_height = (self.height * 68 / 100)
            .clamp(560, 680 * self.scale)
            .min(self.height.saturating_sub(top_bar + 24));
        let card_left = self.width * 4 / 100;
        let card_top = top_bar + self.height.saturating_sub(top_bar + card_height) / 2;
        let inner_left = card_left + 32 * self.scale;
        let inner_width = card_width.saturating_sub(64 * self.scale);

        if (1..=4).contains(&step) {
            let field_top = card_top + (94 + 132 + 28) * self.scale;
            if rect(inner_left, field_top, inner_width, 50 * self.scale).contains(point) {
                return Some(OnboardingTarget::Input);
            }
        }

        let button_top = card_top + card_height.saturating_sub(72 * self.scale);
        if step == 0 {
            return rect(inner_left, button_top, inner_width, 48 * self.scale)
                .contains(point)
                .then_some(OnboardingTarget::Primary);
        }
        let back_width = inner_width * 30 / 100;
        if rect(inner_left, button_top, back_width, 48 * self.scale).contains(point) {
            return Some(OnboardingTarget::Back);
        }
        let primary_left = inner_left + back_width + 12 * self.scale;
        rect(
            primary_left,
            button_top,
            inner_width.saturating_sub(back_width + 12 * self.scale),
            48 * self.scale,
        )
        .contains(point)
        .then_some(OnboardingTarget::Primary)
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
            (2, card_y + sw(439), sw(56)),
            (3, card_y + sw(557), sw(55)),
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
        let menu_bounds = [
            (300usize, 60usize),
            (360, 60),
            (420, 62),
            (482, 84),
            (610, 62),
        ];
        for (index, (left, width)) in menu_bounds.iter().enumerate() {
            if rect(
                left * self.scale,
                4 * self.scale,
                width * self.scale,
                top_bar.saturating_sub(8 * self.scale),
            )
            .contains(point)
            {
                return Some(DesktopTarget::TopMenu(index + 1));
            }
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
            return Some(DesktopTarget::Status(6));
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
            if !window_maximized {
                let handle = (12 * self.scale).max(12);
                let corners = [
                    rect(browser_left, browser_top, handle, handle),
                    rect(
                        browser_left + browser_width.saturating_sub(handle),
                        browser_top,
                        handle,
                        handle,
                    ),
                    rect(
                        browser_left,
                        browser_top + browser_height.saturating_sub(handle),
                        handle,
                        handle,
                    ),
                    rect(
                        browser_left + browser_width.saturating_sub(handle),
                        browser_top + browser_height.saturating_sub(handle),
                        handle,
                        handle,
                    ),
                ];
                for (index, bounds) in corners.iter().enumerate() {
                    if bounds.contains(point) {
                        return Some(DesktopTarget::HomeResize(index));
                    }
                }
            }
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
            if rect(browser_left, browser_top, browser_width, title_height).contains(point) {
                return Some(DesktopTarget::HomeTitle);
            }
            let tool_top = browser_top + title_height;
            for index in 0..2usize {
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
        let panel_top = dock_top
            .saturating_sub(14 * self.scale)
            .saturating_sub(panel_height);
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
            grid_cell_width: inner_width / 6,
            grid_row_height,
            category_left,
            category_top: panel_top + panel_height * 76 / 100,
            category_width,
            category_height: panel_height * 16 / 100,
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
        for index in 0..visible_apps.min(12) {
            let column = index % 6;
            let row = index / 6;
            if rect(
                geometry.grid_left + column * geometry.grid_cell_width,
                geometry.grid_top + row * geometry.grid_row_height,
                geometry.grid_cell_width,
                geometry.grid_row_height,
            )
            .contains(point)
            {
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
        if geometry.title.contains(point) {
            return DesktopAppWindowTarget::Title;
        }
        if text_editor && geometry.toolbar.contains(point) {
            let relative = point.x.saturating_sub(geometry.toolbar.x) as usize;
            if relative < 108 * self.scale {
                return DesktopAppWindowTarget::NewDocument;
            }
            if relative < 216 * self.scale {
                return DesktopAppWindowTarget::SaveDocument;
            }
        }
        if geometry.content.contains(point) {
            return DesktopAppWindowTarget::Content;
        }
        DesktopAppWindowTarget::None
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
    // FUNC: settings_target
    // DESC: Resolves Settings navigation, value rows, and close affordance without side effects on hover.
    // ------------------=
    pub fn settings_target(
        self,
        normalized_x: i32,
        normalized_y: i32,
        maximized: bool,
    ) -> Option<SettingsTarget> {
        let point = self.point(normalized_x, normalized_y);
        let top_bar = self.top_bar_height();
        let restored_width = (self.width * 68 / 100)
            .clamp(900, 1200 * self.scale)
            .min(self.width.saturating_sub(40));
        let restored_height = (self.height * 62 / 100)
            .clamp(560, 760 * self.scale)
            .min(self.height.saturating_sub(top_bar + 28));
        let (left, top, width, _height) = if maximized {
            let inset = 10 * self.scale;
            (
                inset,
                top_bar + inset,
                self.width.saturating_sub(inset * 2),
                self.height.saturating_sub(top_bar + inset * 2),
            )
        } else {
            (
                self.width.saturating_sub(restored_width) / 2,
                top_bar + self.height.saturating_sub(top_bar + restored_height) / 2,
                restored_width,
                restored_height,
            )
        };
        let title_height = 54 * self.scale;
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
        let nav_w = width * 28 / 100;
        for index in 0..8usize {
            let y = top + title_height + (25 + index * 43) * self.scale;
            if rect(
                left + 10 * self.scale,
                y.saturating_sub(10 * self.scale),
                nav_w.saturating_sub(20 * self.scale),
                36 * self.scale,
            )
            .contains(point)
            {
                return Some(SettingsTarget::Section(index));
            }
        }
        let content_x = left + nav_w + 34 * self.scale;
        let content_width = width.saturating_sub(nav_w + 68 * self.scale);
        let content_y = top + title_height + 29 * self.scale;
        for index in 0..5usize {
            let y = content_y + (78 + index * 58) * self.scale;
            if rect(content_x, y, content_width, 46 * self.scale).contains(point) {
                return Some(SettingsTarget::ContentRow(index));
            }
        }
        None
    }

    // ------------------------=
    // FUNC: settings_icon_theme_target
    // DESC: Hit-tests the three explicit icon-family preview cards in Themes and Skins.
    // ------------------=
    pub fn settings_icon_theme_target(
        self,
        normalized_x: i32,
        normalized_y: i32,
        maximized: bool,
    ) -> Option<u8> {
        let point = self.point(normalized_x, normalized_y);
        let top_bar = self.top_bar_height();
        let restored_width = (self.width * 68 / 100)
            .clamp(900, 1200 * self.scale)
            .min(self.width.saturating_sub(40));
        let restored_height = (self.height * 62 / 100)
            .clamp(560, 760 * self.scale)
            .min(self.height.saturating_sub(top_bar + 28));
        let (left, top, width) = if maximized {
            let inset = 10 * self.scale;
            (inset, top_bar + inset, self.width.saturating_sub(inset * 2))
        } else {
            (
                self.width.saturating_sub(restored_width) / 2,
                top_bar + self.height.saturating_sub(top_bar + restored_height) / 2,
                restored_width,
            )
        };
        let title_height = 54 * self.scale;
        let nav_width = width * 28 / 100;
        let content_x = left + nav_width + 34 * self.scale;
        let content_width = width.saturating_sub(nav_width + 68 * self.scale);
        let content_y = top + title_height + 29 * self.scale;
        let preview_top = content_y + 380 * self.scale;
        let preview_gap = content_width / 3;
        for theme in 0..3usize {
            if rect(
                content_x + theme * preview_gap + 4 * self.scale,
                preview_top,
                preview_gap.saturating_sub(8 * self.scale),
                78 * self.scale,
            )
            .contains(point)
            {
                return Some(theme as u8);
            }
        }
        None
    }
}

// ------------------------=
// FUNC: resize_home_window
// DESC: Applies traditional four-corner resizing while preserving minimum size and the visible work area.
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
    let right = x.saturating_add(width);
    let bottom = y.saturating_add(height);
    let (next_x, next_width) = if matches!(corner, 0 | 2) {
        let next_x = pointer_x.clamp(0, right.saturating_sub(300));
        (next_x, right.saturating_sub(next_x))
    } else {
        (
            x,
            pointer_x
                .saturating_sub(x)
                .clamp(300, 1000i32.saturating_sub(x)),
        )
    };
    let (next_y, next_height) = if matches!(corner, 0 | 1) {
        let next_y = pointer_y.clamp(50, bottom.saturating_sub(260));
        (next_y, bottom.saturating_sub(next_y))
    } else {
        (
            y,
            pointer_y
                .saturating_sub(y)
                .clamp(260, 920i32.saturating_sub(y)),
        )
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
