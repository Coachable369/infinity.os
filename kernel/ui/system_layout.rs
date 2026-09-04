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
    HomeToolbar(usize),
    HomeSidebar(usize),
    HomeItem(usize),
    Dock(usize),
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
            scale: if width >= 2560 && height >= 1440 { 2 } else { 1 },
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
        (46 * self.scale).min(self.height / 12).max(40)
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
    pub fn authentication_target(
        self,
        normalized_x: i32,
        normalized_y: i32,
    ) -> Option<usize> {
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
        for index in 0..4usize {
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
        let point = self.point(normalized_x, normalized_y);
        let top_bar = self.top_bar_height();
        let brand_width = (150 * self.scale).min(self.width / 5);
        if rect(8 * self.scale, 5 * self.scale, brand_width, top_bar.saturating_sub(10 * self.scale)).contains(point) {
            return Some(DesktopTarget::InfinityMenu);
        }
        let menu_bounds = [
            (178usize, 60usize),
            (238, 60),
            (298, 62),
            (360, 84),
            (444, 62),
        ];
        for (index, (left, width)) in menu_bounds.iter().enumerate() {
            if rect(left * self.scale, 5 * self.scale, width * self.scale, top_bar.saturating_sub(10 * self.scale)).contains(point) {
                return Some(DesktopTarget::TopMenu(index + 1));
            }
        }
        let status_width = 32 * self.scale;
        let clock_width = 88 * self.scale;
        let status_left = self.width.saturating_sub(7 * status_width + clock_width + 10 * self.scale);
        for index in 0..7usize {
            if rect(status_left + index * status_width, 4 * self.scale, status_width, top_bar.saturating_sub(8 * self.scale)).contains(point) {
                return Some(DesktopTarget::Status(index));
            }
        }
        if rect(self.width.saturating_sub(clock_width + 8 * self.scale), 4 * self.scale, clock_width, top_bar.saturating_sub(8 * self.scale)).contains(point) {
            return Some(DesktopTarget::Status(6));
        }

        if window_visible {
            let (browser_left, browser_top, browser_width, _) = self.home_window_geometry(window_x, window_y, window_maximized);
            let title_height = 34 * self.scale;
            for index in 0..3usize {
                let control_left = browser_left + browser_width.saturating_sub((28 + (2 - index) * 27) * self.scale);
                if rect(control_left, browser_top + 7 * self.scale, 20 * self.scale, 20 * self.scale).contains(point) {
                    return Some(DesktopTarget::HomeControl(index));
                }
            }
            if rect(browser_left, browser_top, browser_width, title_height).contains(point) {
                return Some(DesktopTarget::HomeTitle);
            }
            let tool_top = browser_top + title_height;
            for index in 0..2usize {
                if rect(browser_left + (7 + index * 28) * self.scale, tool_top + 4 * self.scale, 26 * self.scale, 30 * self.scale).contains(point) {
                    return Some(DesktopTarget::HomeToolbar(index));
                }
            }
            let sidebar_width = browser_width * 27 / 100;
            for index in 0..9usize {
                let item_y = tool_top + (72 + index * 20) * self.scale;
                if rect(browser_left + 7, item_y.saturating_sub(3), sidebar_width.saturating_sub(14), 20 * self.scale).contains(point) {
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
                if rect(grid_x + column * gap.saturating_sub(6 * self.scale), grid_y + row * tile_step.saturating_sub(8 * self.scale), gap.max(44 * self.scale), tile_step.max(54 * self.scale)).contains(point) {
                    return Some(DesktopTarget::HomeItem(index));
                }
            }
        }

        let dock_width = self.width * 54 / 100;
        let dock_height = 72 * self.scale;
        let dock_left = self.width.saturating_sub(dock_width) / 2;
        let dock_top = self.height.saturating_sub(dock_height + 10 * self.scale);
        if rect(dock_left, dock_top, dock_width, dock_height).contains(point) {
            let icon_gap = dock_width / 9;
            let relative = point.x.saturating_sub(dock_left as i32) as usize;
            return Some(DesktopTarget::Dock((relative / icon_gap).min(7)));
        }
        None
    }

    // ------------------------=
    // FUNC: home_window_geometry
    // DESC: Returns the shared restored or maximized Home window geometry.
    // ------------------=
    pub fn home_window_geometry(self, window_x: i32, window_y: i32, maximized: bool) -> (usize, usize, usize, usize) {
        if maximized {
            let left = 10 * self.scale;
            let top = self.top_bar_height() + 10 * self.scale;
            let bottom = self.height.saturating_sub(90 * self.scale);
            return (left, top, self.width.saturating_sub(left * 2), bottom.saturating_sub(top));
        }
        (
            self.width * window_x.clamp(10, 540) as usize / 1000,
            self.height * window_y.clamp(80, 550) as usize / 1000,
            self.width * 43 / 100,
            (self.height * 38 / 100).min(430 * self.scale),
        )
    }

    // ------------------------=
    // FUNC: system_menu_geometry
    // DESC: Returns the shared top-menu origin, bounds, and row count for the selected native menu.
    // ------------------=
    pub fn system_menu_geometry(self, menu_kind: usize) -> (usize, usize, usize, usize, usize) {
        let (anchor, width, count) = match menu_kind {
            1 => (176usize, 248usize, 5usize),
            2 => (236, 230, 6),
            3 => (296, 238, 5),
            4 => (358, 242, 4),
            5 => (442, 252, 4),
            _ => (16, 268, 10),
        };
        let x = (anchor * self.scale).min(self.width.saturating_sub(width * self.scale + 8));
        let y = self.top_bar_height() + 6 * self.scale;
        let menu_width = (width * self.scale).min(self.width.saturating_sub(x + 8));
        let menu_height = (22 + count * 34) * self.scale;
        (x, y, menu_width, menu_height, count)
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
            if rect(menu_x + 7 * self.scale, row_y, menu_w.saturating_sub(14 * self.scale), 30 * self.scale).contains(point) {
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
            (inset, top_bar + inset, self.width.saturating_sub(inset * 2), self.height.saturating_sub(top_bar + inset * 2))
        } else {
            (self.width.saturating_sub(restored_width) / 2, top_bar + self.height.saturating_sub(top_bar + restored_height) / 2, restored_width, restored_height)
        };
        let title_height = 54 * self.scale;
        for index in 0..3usize {
            let control_left = left + width.saturating_sub((26 + (2 - index) * 25) * self.scale);
            if rect(control_left, top + 12 * self.scale, 20 * self.scale, 24 * self.scale).contains(point) {
                return Some(SettingsTarget::WindowControl(index));
            }
        }
        let nav_w = width * 28 / 100;
        for index in 0..8usize {
            let y = top + title_height + (25 + index * 43) * self.scale;
            if rect(left + 10 * self.scale, y.saturating_sub(10 * self.scale), nav_w.saturating_sub(20 * self.scale), 36 * self.scale).contains(point) {
                return Some(SettingsTarget::Section(index));
            }
        }
        let content_x = left + nav_w + 34 * self.scale;
        let content_width = width.saturating_sub(nav_w + 68 * self.scale);
        let content_y = top + title_height + 29 * self.scale;
        for index in 0..4usize {
            let y = content_y + (78 + index * 58) * self.scale;
            if rect(content_x, y, content_width, 46 * self.scale).contains(point) {
                return Some(SettingsTarget::ContentRow(index));
            }
        }
        None
    }
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
