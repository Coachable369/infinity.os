#[path = "../kernel/ui/bitmap.rs"]
mod bitmap;

#[path = "../kernel/ui/installer_template.rs"]
mod installer_template;

#[path = "../kernel/ui/installer_layout.rs"]
mod installer_layout;

use installer_layout::installer_wizard_layout;
use installer_template::{InstallerTemplate, InstallerTemplateRole};

const FACTORY_TEMPLATE: &[u8] = include_bytes!("../assets/boot/installer-screens.iuit");
const CONFIGURATION_TEMPLATE: &[u8] = include_bytes!("../assets/boot/configuration-screens.iuit");
const SETTINGS_TEMPLATE: &[u8] = include_bytes!("../assets/boot/settings-screens.iuit");

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
// DESC: Exercises authored runtime structure, geometry consumption, data bindings, and action-integrity rejection.
// ------------------=
fn main() {
    let mut lines = [[0u8; 96]; 6];
    let lengths = [2, 3, 1, 0, 0, 0];
    lines[0][..2].copy_from_slice(&[70, 71]);
    lines[1][..3].copy_from_slice(&[80, 81, 82]);
    lines[2][0] = 90;
    let (details, length) = installer_template::installer_live_details(&lines, &lengths, 3, &[60], &[61]);
    assert_eq!(&details[..length], &[80, 81, 82, 10, 90, 10, 60, 61]);
    let (details, length) = installer_template::installer_live_details(&lines, &lengths, 1, &[], &[]);
    assert_eq!(&details[..length], &[70, 71]);
    let (_, length) = installer_template::installer_live_details(&lines, &[usize::MAX; 6], usize::MAX, &[1; 1024], &[2; 1024]);
    assert_eq!(length, 768);
    let template = InstallerTemplate::parse(FACTORY_TEMPLATE).expect("factory template must load");
    for screen in [3, 4, 6, 10] {
        let details = template.element(screen, InstallerTemplateRole::LiveDetails).unwrap();
        assert_eq!(details.kind, 3);
        assert!(!details.hidden);
        assert!(details.frame.width > 0 && details.frame.height > 0);
    }
    assert!((1..=32).contains(&template.screen_count()));
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
        let authored_content = template
            .element(screen, InstallerTemplateRole::Content)
            .expect("compiled screen must retain its required content structure");
        let runtime_content = installer_wizard_layout(screen, 1000, 1000).content;
        assert_eq!(
            (
                runtime_content.left,
                runtime_content.top,
                runtime_content.width,
                runtime_content.height,
            ),
            (
                usize::from(authored_content.frame.x),
                usize::from(authored_content.frame.y),
                usize::from(authored_content.frame.width),
                usize::from(authored_content.frame.height),
            ),
            "runtime geometry must follow the compiled Studio template"
        );
    }
    if template.screen_count() >= 8 {
        let progress = template
            .element(8, InstallerTemplateRole::ProgressBar)
            .expect("installing screen must expose one semantic progress control");
        assert_eq!(progress.kind, 6);
        let progress_hero = template
            .element(8, InstallerTemplateRole::ProgressHero)
            .expect("installing screen must expose its runtime progress hero");
        assert_eq!(progress_hero.kind, 2);
    }
    if template.screen_count() >= 5 {
        let date = template
            .element(5, InstallerTemplateRole::DateField)
            .expect("date field must be a saved semantic control");
        let time = template
            .element(5, InstallerTemplateRole::TimeField)
            .expect("time field must be a saved semantic control");
        let zone = template
            .element(5, InstallerTemplateRole::TimeZoneSelector)
            .expect("time-zone selector must be a saved semantic control");
        let badge = template
            .element(5, InstallerTemplateRole::OffsetBadge)
            .expect("UTC offset badge must be a saved semantic control");
        let map = template
            .element(5, InstallerTemplateRole::TimeZoneMap)
            .expect("time-zone map must be a saved semantic layer");
        assert!(!date.hidden && !time.hidden && !zone.hidden && !badge.hidden && !map.hidden);
    }

    let configuration =
        InstallerTemplate::parse(CONFIGURATION_TEMPLATE).expect("configuration template must load");
    assert!((1..=32).contains(&configuration.screen_count()));
    for screen in 1..=configuration.screen_count() as u8 {
        let card = configuration
            .element(screen, InstallerTemplateRole::Console)
            .expect("configuration card must exist");
        assert!(card.frame.x as u32 + card.frame.width as u32 <= 1000);
        assert!(card.frame.y as u32 + card.frame.height as u32 <= 1000);
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
        assert_eq!(back.kind, 5);
        assert_eq!(primary.kind, 5);
        let count = configuration
            .element_count(screen)
            .expect("configuration layers must enumerate");
        let mut progress_count = 0;
        let mut input_count = 0;
        for layer in 0..count {
            let element = configuration
                .layer_at(screen, layer)
                .expect("configuration layer must resolve");
            if element.role == InstallerTemplateRole::ProgressSegment as u8 {
                progress_count += 1;
                assert_eq!(element.kind, 1);
            }
            if element.role == InstallerTemplateRole::Input as u8 && !element.hidden {
                input_count += 1;
            }
        }
        assert_eq!(progress_count, 8);
        assert!(input_count <= 1);
    }
    if configuration.screen_count() >= 2 {
        for screen in 2..=configuration.screen_count().min(5) as u8 {
            let input = configuration
                .element(screen, InstallerTemplateRole::Input)
                .expect("profile and credential steps need an authored input field");
            assert!(!input.hidden);
            assert!(input.frame.x as u32 + input.frame.width as u32 <= 1000);
            assert!(input.frame.y as u32 + input.frame.height as u32 <= 1000);
            assert_eq!(input.input_variable, screen - 1);
        }
    }

    let settings = InstallerTemplate::parse_settings(SETTINGS_TEMPLATE)
        .expect("System Settings template must load");
    assert_eq!(settings.screen_count(), 11);
    for section in 1..=11 {
        let viewport = settings
            .element(section, InstallerTemplateRole::Content)
            .expect("Settings section must expose its authored viewport");
        assert!(viewport.frame.width > 0 && viewport.frame.height > 0);
        assert!(settings.element_count(section).unwrap() >= 12);
    }

    let mut altered = FACTORY_TEMPLATE.to_vec();
    let primary_kind = first_primary_kind_offset(&altered);
    altered[primary_kind] = 1;
    assert!(matches!(
        InstallerTemplate::parse(&altered),
        Err(installer_template::InstallerTemplateError::InvalidNavigation)
    ));
}
