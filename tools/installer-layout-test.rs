#[path = "../kernel/ui/bitmap.rs"]
mod bitmap;

#[path = "../kernel/ui/installer_template.rs"]
mod installer_template;

#[path = "../kernel/ui/installer_layout.rs"]
mod installer_layout;

use installer_layout::{
    configuration_template_input_rect, configuration_template_input_variable,
    configuration_template_step_for_input_variable, installer_confirmation_button_frame,
    installer_confirmation_target, installer_wizard_layout, InstallerConfirmationTarget,
};
use installer_template::InstallerTemplateVariable;

// ------------------------=
// FUNC: main
// DESC: Verifies every installer state shares one centered, contained gold-standard frame.
// ------------------=
fn main() {
    for (display_width, display_height) in [(1024usize, 768usize), (2048, 1238), (2560, 1600)] {
        let reference = installer_wizard_layout(2, display_width, display_height);
        for screen in 1..=11 {
            let layout = installer_wizard_layout(screen, display_width, display_height);
            assert_eq!(layout, reference, "screen {screen} changed installer geometry");
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
    println!("PASS installer layout: all wizard states share one symmetric frame");
}
