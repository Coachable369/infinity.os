#[path = "../kernel/ui/installer_template.rs"]
mod installer_template;

use installer_template::{InstallerTemplate, InstallerTemplateRole};

const FACTORY_TEMPLATE: &[u8] = include_bytes!("../assets/boot/installer-screens.iuit");

// ------------------------=
// FUNC: little_u16
// DESC: Decodes a test cursor's next little-endian 16-bit field.
// ------------------=
fn little_u16(data: &[u8], offset: &mut usize) -> u16 {
    let value = u16::from_le_bytes([data[*offset], data[*offset + 1]]);
    *offset += 2;
    value
}

// ------------------------=
// FUNC: skip_element
// DESC: Advances across one complete runtime element and returns its role and x-field offset.
// ------------------=
fn skip_element(data: &[u8], offset: &mut usize) -> (u8, usize) {
    *offset += 16;
    *offset += 1;
    let role = data[*offset];
    *offset += 1;
    *offset += 1;
    *offset += 2;
    let x_offset = *offset;
    *offset += 8;
    *offset += 8;
    *offset += 2;
    *offset += 2;
    let name = data[*offset] as usize;
    *offset += 1 + name;
    let text = little_u16(data, offset) as usize;
    *offset += text;
    let image = data[*offset] as usize;
    *offset += 1 + image;
    (role, x_offset)
}

// ------------------------=
// FUNC: first_primary_x_offset
// DESC: Locates screen one's protected primary-button x coordinate in test data.
// ------------------=
fn first_primary_x_offset(data: &[u8]) -> usize {
    let mut offset = 8;
    assert_eq!(data[offset], 1);
    offset += 1;
    let title = data[offset] as usize;
    offset += 1 + title;
    let count = little_u16(data, &mut offset);
    for _ in 0..count {
        let (role, x_offset) = skip_element(data, &mut offset);
        if role == InstallerTemplateRole::PrimaryButton as u8 {
            return x_offset;
        }
    }
    panic!("factory primary button missing")
}

// ------------------------=
// FUNC: main
// DESC: Exercises saved-template parsing, screen coverage, authored copy, geometry, and runtime lock rejection.
// ------------------=
fn main() {
    let template = InstallerTemplate::parse(FACTORY_TEMPLATE).expect("factory template must load");
    assert_eq!(template.screen_count(), 11);
    for screen in 1..=template.screen_count() as u8 {
        assert!(template.element(screen, InstallerTemplateRole::Console).is_some());
        assert!(template.element(screen, InstallerTemplateRole::Content).is_some());
        assert!(!template.element(screen, InstallerTemplateRole::Title).unwrap().text.is_empty());
        assert!(!template.element(screen, InstallerTemplateRole::Body).unwrap().text.is_empty());
    }
    let content = template.element(2, InstallerTemplateRole::Content).unwrap();
    assert_eq!((content.frame.x, content.frame.y, content.frame.width, content.frame.height), (38, 396, 924, 404));

    let mut altered = FACTORY_TEMPLATE.to_vec();
    let primary_x = first_primary_x_offset(&altered);
    altered[primary_x] ^= 1;
    assert!(matches!(
        InstallerTemplate::parse(&altered),
        Err(installer_template::InstallerTemplateError::InvalidNavigation)
    ));
}
