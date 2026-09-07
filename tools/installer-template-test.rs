#[path = "../kernel/ui/installer_template.rs"]
mod installer_template;

use installer_template::{InstallerTemplate, InstallerTemplateRole};

const FACTORY_TEMPLATE: &[u8] = include_bytes!("../assets/boot/installer-screens.iuit");
const CONFIGURATION_TEMPLATE: &[u8] = include_bytes!("../assets/boot/configuration-screens.iuit");

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
// DESC: Advances across one complete runtime element and returns its role and kind-field offset.
// ------------------=
fn skip_element(data: &[u8], offset: &mut usize) -> (u8, usize) {
    *offset += 16;
    let kind_offset = *offset;
    *offset += 1;
    let role = data[*offset];
    *offset += 1;
    *offset += 1;
    *offset += 2;
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
    *offset += 4;
    (role, kind_offset)
}

// ------------------------=
// FUNC: first_primary_kind_offset
// DESC: Locates screen one's primary-button kind field in test data.
// ------------------=
fn first_primary_kind_offset(data: &[u8]) -> usize {
    let mut offset = 8;
    assert_eq!(data[offset], 1);
    offset += 1;
    let title = data[offset] as usize;
    offset += 1 + title;
    let count = little_u16(data, &mut offset);
    for _ in 0..count {
        let (role, kind_offset) = skip_element(data, &mut offset);
        if role == InstallerTemplateRole::PrimaryButton as u8 {
            return kind_offset;
        }
    }
    panic!("factory primary button missing")
}

// ------------------------=
// FUNC: main
// DESC: Exercises saved-template parsing, screen coverage, authored copy, geometry, and action-integrity rejection.
// ------------------=
fn main() {
    let template = InstallerTemplate::parse(FACTORY_TEMPLATE).expect("factory template must load");
    assert_eq!(template.screen_count(), 11);
    for screen in 1..=template.screen_count() as u8 {
        let count = template
            .element_count(screen)
            .expect("screen layers must enumerate");
        assert!(count >= 8);
        let mut previous = None;
        for layer in 0..count {
            let element = template
                .layer_at(screen, layer)
                .expect("saved layer must resolve");
            if let Some((z_index, id)) = previous {
                assert!(
                    element.z_index > z_index || (element.z_index == z_index && element.id > id)
                );
            }
            previous = Some((element.z_index, element.id));
        }
        assert!(template
            .element(screen, InstallerTemplateRole::Console)
            .is_some());
        assert!(template
            .element(screen, InstallerTemplateRole::Content)
            .is_some());
        assert!(!template
            .element(screen, InstallerTemplateRole::Title)
            .unwrap()
            .text
            .is_empty());
        assert!(!template
            .element(screen, InstallerTemplateRole::Body)
            .unwrap()
            .text
            .is_empty());
    }
    let content = template.element(2, InstallerTemplateRole::Content).unwrap();
    assert_eq!(
        (
            content.frame.x,
            content.frame.y,
            content.frame.width,
            content.frame.height
        ),
        (38, 396, 924, 404)
    );
    let masthead = template
        .element(1, InstallerTemplateRole::Masthead)
        .unwrap();
    assert!(masthead
        .image_asset
        .ends_with(b"infinity-installer-masthead-v2.png"));
    assert!(template.asset(masthead.image_asset).is_none());

    let configuration =
        InstallerTemplate::parse(CONFIGURATION_TEMPLATE).expect("configuration template must load");
    assert_eq!(configuration.screen_count(), 8);
    for screen in 1..=configuration.screen_count() as u8 {
        let card = configuration
            .element(screen, InstallerTemplateRole::Console)
            .expect("configuration card must exist");
        assert_eq!(
            (
                card.frame.x,
                card.frame.y,
                card.frame.width,
                card.frame.height
            ),
            (40, 128, 340, 736)
        );
        assert_eq!(card.fill, [10, 18, 29, 230]);
        assert_eq!(card.border, [51, 71, 91, 255]);
        assert_eq!(card.corner_radius, 16);
        assert!(configuration
            .element(screen, InstallerTemplateRole::Title)
            .is_some());
        assert!(configuration
            .element(screen, InstallerTemplateRole::Body)
            .is_some());
        let back = configuration
            .element(screen, InstallerTemplateRole::BackButton)
            .unwrap();
        let primary = configuration
            .element(screen, InstallerTemplateRole::PrimaryButton)
            .unwrap();
        assert_eq!(back.frame.height, 47);
        assert_eq!(primary.frame.height, 47);
        assert_eq!(back.fill, [5, 15, 27, 255]);
        assert_eq!(back.border, [42, 69, 91, 255]);
        assert_eq!(primary.fill, [8, 54, 84, 255]);
        assert_eq!(primary.border, [40, 181, 231, 255]);
        if screen == 1 {
            assert_eq!(back.opacity, 0);
            assert_eq!(back.frame, primary.frame);
        } else {
            assert_eq!(primary.frame.x - (back.frame.x + back.frame.width), 8);
        }
    }
    let background = configuration
        .element(1, InstallerTemplateRole::Masthead)
        .unwrap();
    assert!(background
        .image_asset
        .ends_with(b"infinity-onboarding-wallpaper-v1.png"));
    assert!(configuration.asset(background.image_asset).is_none());
    for screen in 2..=5 {
        let input = configuration
            .element(screen, InstallerTemplateRole::Input)
            .expect("profile and credential steps need an authored input field");
        assert!(!input.hidden);
        assert_eq!(
            (
                input.frame.x,
                input.frame.y,
                input.frame.width,
                input.frame.height
            ),
            (72, 442, 276, 47)
        );
        assert_eq!(input.fill, [2, 10, 20, 255]);
        assert_eq!(input.border, [40, 72, 95, 255]);
        assert_eq!(input.corner_radius, 10);
    }

    let mut altered = FACTORY_TEMPLATE.to_vec();
    let primary_kind = first_primary_kind_offset(&altered);
    altered[primary_kind] = 1;
    assert!(matches!(
        InstallerTemplate::parse(&altered),
        Err(installer_template::InstallerTemplateError::InvalidNavigation)
    ));
}
