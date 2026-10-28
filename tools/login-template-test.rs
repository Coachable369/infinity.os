#[path = "../kernel/ui/bitmap.rs"]
mod bitmap;
#[path = "../kernel/ui/installer_template.rs"]
mod installer_template;
#[path = "../kernel/ui/installer_layout.rs"]
mod installer_layout;

// ------------------------=
// FUNC: main
// DESC: Exercises production authentication parsing, semantic actions, visibility and pointer targeting from the packaged layout.
// ------------------=
fn main() {
    use installer_template::{InstallerTemplate, InstallerTemplateRole, login_action};
    assert!(!installer_layout::login_uses_legacy_animation(installer_layout::LOGIN_TEMPLATE_BYTES));
    assert!(installer_layout::login_uses_legacy_animation(&[]));
    assert!(installer_layout::login_uses_legacy_animation(installer_layout::INSTALLER_TEMPLATE_BYTES));
    let mut corrupt_login = installer_layout::LOGIN_TEMPLATE_BYTES.to_vec();
    corrupt_login[0] ^= 0xff;
    assert!(installer_layout::login_uses_legacy_animation(&corrupt_login));
    let template = InstallerTemplate::parse_login(installer_layout::LOGIN_TEMPLATE_BYTES).unwrap();
    let hero = template.element(1, InstallerTemplateRole::ProgressHero).unwrap();
    let bounds = installer_layout::scale_template_rect(hero.frame, 1920, 1080);
    let (regions, count) = installer_layout::login_animation_regions(&template, 1920, 1080);
    assert!(count > 0);
    for region in &regions[..count] {
        assert!(region.left >= bounds.left && region.top >= bounds.top);
        assert!(region.left + region.width <= bounds.left + bounds.width);
        assert!(region.top + region.height <= bounds.top + bounds.height);
        for index in 0..template.element_count(1).unwrap() {
            let element = template.element_at(1, index).unwrap();
            if login_action(&element).is_none() { continue; }
            let control = installer_layout::scale_template_rect(element.frame, 1920, 1080);
            assert!(region.left + region.width <= control.left
                || control.left + control.width <= region.left
                || region.top + region.height <= control.top
                || control.top + control.height <= region.top);
        }
    }
    let mut found = [false; 11];
    for index in 0..template.element_count(1).unwrap() {
        let mut element = template.element_at(1, index).unwrap();
        if let Some(action) = login_action(&element) {
            element.code_identifier = b"renamedControl";
            element.text = b"Custom label";
            assert_eq!(login_action(&element), Some(action));
            found[action] = true;
            let x = i32::from(element.frame.x + element.frame.width / 2);
            let y = i32::from(element.frame.y + element.frame.height / 2);
            assert_eq!(installer_layout::login_target_in(&template, x, y), Some(action));
            element.hidden = true;
            assert_eq!(login_action(&element), None);
            element.hidden = false;
            element.opacity = 0;
            assert_eq!(login_action(&element), None);
        }
    }
    // Optional account-management controls may legitimately be removed in Studio.
    assert!(found[1] && found[2]);
    assert_eq!(template.element(1, InstallerTemplateRole::Input).unwrap().input_variable, 4);
    assert_eq!(installer_layout::login_target_in(&template, 999, 999), None);
    assert!(InstallerTemplate::parse(installer_layout::LOGIN_TEMPLATE_BYTES).is_err());
    assert!(InstallerTemplate::parse_login(installer_layout::INSTALLER_TEMPLATE_BYTES).is_err());
}
