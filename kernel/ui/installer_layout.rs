//! Shared, architecture-neutral geometry for every InfinityOS installer step.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InstallerRect {
    pub left: usize,
    pub top: usize,
    pub width: usize,
    pub height: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InstallerWizardLayout {
    pub masthead: InstallerRect,
    pub panel: InstallerRect,
    pub content: InstallerRect,
    pub navigation_rail: InstallerRect,
    pub back_button: InstallerRect,
    pub primary_button: InstallerRect,
    pub footer_rail: InstallerRect,
}

impl InstallerRect {
    // ------------------------=
    // FUNC: right
    // DESC: Returns the exclusive right edge of an installer rectangle.
    // ------------------=
    pub const fn right(self) -> usize {
        self.left + self.width
    }

    // ------------------------=
    // FUNC: bottom
    // DESC: Returns the exclusive bottom edge of an installer rectangle.
    // ------------------=
    pub const fn bottom(self) -> usize {
        self.top + self.height
    }

    // ------------------------=
    // FUNC: contains
    // DESC: Reports whether one installer rectangle completely contains another.
    // ------------------=
    pub const fn contains(self, other: Self) -> bool {
        other.left >= self.left
            && other.top >= self.top
            && other.right() <= self.right()
            && other.bottom() <= self.bottom()
    }
}

// ------------------------=
// FUNC: installer_wizard_layout
// DESC: Resolves the invariant gold-standard installer frame for any wizard screen.
// ------------------=
pub fn installer_wizard_layout(
    _screen: u8,
    display_width: usize,
    display_height: usize,
) -> InstallerWizardLayout {
    let masthead_height = display_height * 30 / 100;
    let masthead_width = (masthead_height * 3).min(display_width * 82 / 100);
    let masthead_left = display_width.saturating_sub(masthead_width) / 2;
    let panel_left = display_width * 2 / 100;
    let panel = InstallerRect {
        left: panel_left,
        top: display_height * 32 / 100,
        width: display_width.saturating_sub(panel_left * 2),
        height: display_height * 66 / 100,
    };
    InstallerWizardLayout {
        masthead: InstallerRect {
            left: masthead_left,
            top: 0,
            width: display_width.saturating_sub(masthead_left * 2),
            height: masthead_height,
        },
        panel,
        content: InstallerRect {
            left: display_width * 38 / 1000,
            top: display_height * 396 / 1000,
            width: display_width * 924 / 1000,
            height: display_height * 404 / 1000,
        },
        navigation_rail: InstallerRect {
            left: display_width * 8 / 100,
            top: display_height * 90 / 100,
            width: display_width * 84 / 100,
            height: display_height * 4 / 100,
        },
        back_button: InstallerRect {
            left: display_width * 135 / 1000,
            top: display_height * 820 / 1000,
            width: display_width * 350 / 1000,
            height: display_height * 55 / 1000,
        },
        primary_button: InstallerRect {
            left: display_width * 510 / 1000,
            top: display_height * 820 / 1000,
            width: display_width * 350 / 1000,
            height: display_height * 55 / 1000,
        },
        footer_rail: InstallerRect {
            left: display_width * 10 / 100,
            top: display_height * 900 / 1000,
            width: display_width * 80 / 100,
            height: display_height * 70 / 1000,
        },
    }
}
