//! Shared, architecture-neutral geometry for every InfinityOS installer step.

use super::installer_template::{InstallerTemplate, InstallerTemplateRect, InstallerTemplateRole};

pub const INSTALLER_TEMPLATE_BYTES: &[u8] =
    include_bytes!("../../assets/boot/installer-screens.iuit");
pub const CONFIGURATION_TEMPLATE_BYTES: &[u8] =
    include_bytes!("../../assets/boot/configuration-screens.iuit");

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
    screen: u8,
    display_width: usize,
    display_height: usize,
) -> InstallerWizardLayout {
    if let Ok(template) = InstallerTemplate::parse(INSTALLER_TEMPLATE_BYTES) {
        if let (Some(masthead), Some(panel), Some(content), Some(back), Some(primary), Some(footer)) = (
            template.element(screen, InstallerTemplateRole::Masthead),
            template.element(screen, InstallerTemplateRole::Console),
            template.element(screen, InstallerTemplateRole::Content),
            template.element(screen, InstallerTemplateRole::BackButton),
            template.element(screen, InstallerTemplateRole::PrimaryButton),
            template.element(screen, InstallerTemplateRole::Footer),
        ) {
            return InstallerWizardLayout {
                masthead: scale_template_rect(masthead.frame, display_width, display_height),
                panel: scale_template_rect(panel.frame, display_width, display_height),
                content: scale_template_rect(content.frame, display_width, display_height),
                navigation_rail: InstallerRect {
                    left: display_width * 8 / 100,
                    top: display_height * 90 / 100,
                    width: display_width * 84 / 100,
                    height: display_height * 4 / 100,
                },
                back_button: scale_template_rect(back.frame, display_width, display_height),
                primary_button: scale_template_rect(primary.frame, display_width, display_height),
                footer_rail: scale_template_rect(footer.frame, display_width, display_height),
            };
        }
    }
    fallback_installer_wizard_layout(display_width, display_height)
}

// ------------------------=
// FUNC: installer_template_text
// DESC: Returns authored copy for a validated installer screen role.
// ------------------=
pub fn installer_template_text(screen: u8, role: InstallerTemplateRole) -> Option<&'static [u8]> {
    InstallerTemplate::parse(INSTALLER_TEMPLATE_BYTES)
        .ok()?
        .element(screen, role)
        .map(|element| element.text)
}

// ------------------------=
// FUNC: configuration_template_text
// DESC: Returns authored copy for one validated post-install OS configuration screen role.
// ------------------=
pub fn configuration_template_text(
    step: usize,
    role: InstallerTemplateRole,
) -> Option<&'static [u8]> {
    InstallerTemplate::parse(CONFIGURATION_TEMPLATE_BYTES)
        .ok()?
        .element(step.saturating_add(1) as u8, role)
        .map(|element| element.text)
}

// ------------------------=
// FUNC: configuration_template_rect
// DESC: Scales an authored first-boot element into the active installed-system display.
// ------------------=
pub fn configuration_template_rect(
    step: usize,
    role: InstallerTemplateRole,
    display_width: usize,
    display_height: usize,
) -> Option<InstallerRect> {
    let element = InstallerTemplate::parse(CONFIGURATION_TEMPLATE_BYTES)
        .ok()?
        .element(step.saturating_add(1) as u8, role)?;
    (!element.hidden).then(|| scale_template_rect(element.frame, display_width, display_height))
}

// ------------------------=
// FUNC: scale_template_rect
// DESC: Scales normalized editor geometry into the active display dimensions.
// ------------------=
fn scale_template_rect(
    frame: InstallerTemplateRect,
    display_width: usize,
    display_height: usize,
) -> InstallerRect {
    let left = display_width * frame.x as usize / 1000;
    let top = display_height * frame.y as usize / 1000;
    let right = display_width * (frame.x as usize + frame.width as usize) / 1000;
    let bottom = display_height * (frame.y as usize + frame.height as usize) / 1000;
    let width = if frame.x as usize * 2 + frame.width as usize == 1000 {
        display_width.saturating_sub(left * 2)
    } else {
        right.saturating_sub(left)
    };
    InstallerRect {
        left,
        top,
        width,
        height: bottom.saturating_sub(top),
    }
}

// ------------------------=
// FUNC: fallback_installer_wizard_layout
// DESC: Provides safe compiled geometry when persisted template validation fails.
// ------------------=
fn fallback_installer_wizard_layout(
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
