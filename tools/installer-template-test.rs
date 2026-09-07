#[path = "../kernel/ui/bitmap.rs"]
mod bitmap;

#[path = "../kernel/ui/installer_template.rs"]
mod installer_template;

use bitmap::RuntimeBitmap;
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
    assert_eq!(date.fill, [2, 10, 20, 255]);
    assert_eq!(time.fill, date.fill);
    assert_eq!(zone.border, [40, 72, 95, 255]);
    assert_eq!(badge.border, [40, 181, 231, 255]);
    assert!(map.image_asset.ends_with(b"infinity-time-zone-map-v1.png"));
    assert!(template.asset(map.image_asset).is_none());

    let configuration =
        InstallerTemplate::parse(CONFIGURATION_TEMPLATE).expect("configuration template must load");
    assert_eq!(configuration.screen_count(), 8);
    for screen in 1..=configuration.screen_count() as u8 {
        let card = configuration
            .element(screen, InstallerTemplateRole::Console)
            .expect("configuration card must exist");
        assert!(card.frame.width >= 300);
        assert!(card.frame.height >= 600);
        assert!(card.frame.x as u32 + card.frame.width as u32 <= 1000);
        assert!(card.frame.y as u32 + card.frame.height as u32 <= 1000);
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
                assert_eq!(element.frame.height, 20);
                assert!(element.text.is_empty());
            }
            if element.role == InstallerTemplateRole::Input as u8 && !element.hidden {
                input_count += 1;
            }
        }
        assert_eq!(progress_count, 8);
        assert!(input_count <= 1);
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
    let node_image = (0..configuration.element_count(1).unwrap())
        .filter_map(|layer| configuration.layer_at(1, layer))
        .find(|element| element.kind == 2 && element.role == InstallerTemplateRole::Image as u8)
        .expect("Studio-authored node image must remain in the configuration scene");
    let node_bitmap = RuntimeBitmap::parse(
        configuration
            .asset(node_image.image_asset)
            .expect("Studio-authored node image must be embedded in the runtime template"),
    )
    .expect("embedded node image must be runtime-decodable");
    let transparent = node_bitmap.rgba(0, 0).unwrap();
    let visible = node_bitmap
        .rgba(node_bitmap.width() / 2, node_bitmap.height() / 2)
        .unwrap();
    assert!(transparent[3] < 8, "transparent PNG pixels must remain transparent");
    assert!(visible[3] > 200, "visible PNG pixels must remain visible");
    assert!(visible[0] > 0 || visible[1] > 0 || visible[2] > 0);
    for screen in 2..=5 {
        let input = configuration
            .element(screen, InstallerTemplateRole::Input)
            .expect("profile and credential steps need an authored input field");
        assert!(!input.hidden);
        assert!(input.frame.width >= 220);
        assert_eq!(input.frame.height, 47);
        assert!(input.frame.x as u32 + input.frame.width as u32 <= 1000);
        assert!(input.frame.y as u32 + input.frame.height as u32 <= 1000);
        assert_eq!(input.fill, [2, 10, 20, 255]);
        assert_eq!(input.border, [40, 72, 95, 255]);
        assert_eq!(input.corner_radius, 10);
        assert_eq!(input.input_variable, screen - 1);
    }

    let mut altered = FACTORY_TEMPLATE.to_vec();
    let primary_kind = first_primary_kind_offset(&altered);
    altered[primary_kind] = 1;
    assert!(matches!(
        InstallerTemplate::parse(&altered),
        Err(installer_template::InstallerTemplateError::InvalidNavigation)
    ));
}
