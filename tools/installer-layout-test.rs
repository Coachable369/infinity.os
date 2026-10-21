#[path = "../kernel/ui/bitmap.rs"]
mod bitmap;
#[path="installer-work-test.rs"]
mod installer_work_test;

#[path = "../kernel/ui/installer_template.rs"]
mod installer_template;

#[path = "../kernel/ui/installer_layout.rs"]
mod installer_layout;

use installer_layout::{
    configuration_template_input_rect, configuration_template_input_variable,
    configuration_template_step_for_input_variable, installer_confirmation_button_frame,
    installer_confirmation_target, installer_wizard_layout, InstallerConfirmationTarget,
};
use installer_template::{InstallerTemplate, InstallerTemplateRole as Role, InstallerTemplateVariable};

// ------------------------=
// FUNC: main
// DESC: Verifies authored screens retain their exact Studio geometry and the compiled fallback remains centered and contained.
// ------------------=
fn main() {
    installer_work_test::run();
    for (display_width, display_height) in [(1024usize, 768usize), (2048, 1238), (2560, 1600)] {
        let template = InstallerTemplate::parse(installer_layout::INSTALLER_TEMPLATE_BYTES).unwrap();
        for screen in 1..=template.screen_count() as u8 {
            let layout = installer_wizard_layout(screen, display_width, display_height);
            for (role, actual) in [
                (Role::Masthead, layout.masthead), (Role::Console, layout.panel),
                (Role::Content, layout.content), (Role::BackButton, layout.back_button),
                (Role::PrimaryButton, layout.primary_button), (Role::Footer, layout.footer_rail),
            ] {
                let frame = template.element(screen, role).unwrap().frame;
                assert_eq!(actual.left, display_width * usize::from(frame.x) / 1000);
                assert_eq!(actual.top, display_height * usize::from(frame.y) / 1000);
                if usize::from(frame.x) * 2 + usize::from(frame.width) == 1000 {
                    assert_eq!(actual.left * 2 + actual.width, display_width);
                } else {
                    assert_eq!(actual.right(), display_width * usize::from(frame.x + frame.width) / 1000);
                }
                assert_eq!(actual.bottom(), display_height * usize::from(frame.y + frame.height) / 1000);
                assert!(actual.right() <= display_width && actual.bottom() <= display_height);
                assert!(actual.width > 0 && actual.height > 0);
            }
        }
        // User-authored frames need not be symmetric or identical across screens.
        // Those invariants belong to the compiled safety fallback, not Studio's editable input.
        for screen in [0, 254, 255] {
            let layout = installer_wizard_layout(screen, display_width, display_height);
            assert_eq!(layout, installer_wizard_layout(255, display_width, display_height));
            assert_eq!(layout.masthead.left * 2 + layout.masthead.width, display_width);
            assert_eq!(layout.panel.left * 2 + layout.panel.width, display_width);
            assert!(layout.masthead.bottom() <= layout.panel.top);
            assert!(layout.panel.right() <= display_width);
            assert!(layout.panel.bottom() <= display_height);
            assert!(layout.panel.contains(layout.content));
            assert!(layout.panel.contains(layout.navigation_rail));
            assert!(layout.panel.contains(layout.back_button));
            assert!(layout.panel.contains(layout.primary_button));
            assert!(layout.panel.contains(layout.footer_rail));
            assert!(layout.content.bottom() <= layout.back_button.top);
            assert_eq!(layout.back_button.top, layout.primary_button.top);
            assert_eq!(layout.back_button.width, layout.primary_button.width);
            assert_eq!(layout.back_button.height, layout.primary_button.height);
        }
    }

    for target in [
        InstallerConfirmationTarget::Cancel,
        InstallerConfirmationTarget::Install,
    ] {
        let frame = installer_confirmation_button_frame(target);
        for x in frame.left..frame.right() {
            for y in frame.top..frame.bottom() {
                assert_eq!(installer_confirmation_target(x as i32, y as i32), Some(target));
            }
        }
    }

    let install = installer_confirmation_button_frame(InstallerConfirmationTarget::Install);
    assert_eq!(
        installer_confirmation_target(
            (install.left + install.width / 2) as i32,
            (install.top + install.height / 2) as i32,
        ),
        Some(InstallerConfirmationTarget::Install),
        "one complete click in the rendered Erase & Install button must resolve immediately",
    );
    assert_eq!(installer_confirmation_target(500, 565), None);

    for (step, variable) in [
        (1, InstallerTemplateVariable::MachineNodeName),
        (2, InstallerTemplateVariable::ProfileName),
        (3, InstallerTemplateVariable::DisplayName),
        (4, InstallerTemplateVariable::Password),
    ] {
        assert_eq!(configuration_template_input_variable(step), variable);
        assert_eq!(configuration_template_step_for_input_variable(variable), Some(step));
        let field = configuration_template_input_rect(step, 1760, 992)
            .expect("bound field must have one authored runtime rectangle");
        assert!(field.width > 0 && field.height > 0);
    }
    assert_eq!(configuration_template_input_rect(0, 1760, 992), None);
    assert_eq!(configuration_template_input_rect(5, 1760, 992), None);
    println!("PASS installer layout: authored geometry, safe fallback, and one-shot interactive provisioning");
}
