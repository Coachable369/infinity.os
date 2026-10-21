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
    let identifier = data[*offset] as usize;
    *offset += 1 + identifier;
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
// FUNC: first_identifier_byte_offset
// DESC: Locates the first authored code identifier byte in a runtime template.
// ------------------=
fn first_identifier_byte_offset(data: &[u8]) -> usize {
    let mut offset = 8;
    offset += 1;
    let title = data[offset] as usize;
    offset += 1 + title;
    offset += 2 + 16;
    assert!(data[offset] > 0);
    offset + 1
}

// ------------------------=
// FUNC: main
// DESC: Exercises authored runtime structure, geometry consumption, data bindings, and action-integrity rejection.
// ------------------=
fn main() {
    use installer_layout::InstallerDateTimeTarget as DateTarget;
    let template=InstallerTemplate::parse(FACTORY_TEMPLATE).unwrap();
    for (role,focus) in [(InstallerTemplateRole::DateField,2),(InstallerTemplateRole::TimeField,3),
        (InstallerTemplateRole::TimeZoneSelector,4)] {
        let r=template.element(5,role).unwrap().frame;
        let y=i32::from(r.y+r.height/2);
        assert_eq!(installer_layout::installer_date_time_target_in(&template,i32::from(r.x)+1,y),
            Some(DateTarget::Field{focus,part:None,adjustment:-1}));
        assert_eq!(installer_layout::installer_date_time_target_in(&template,i32::from(r.x+r.width)-1,y),
            Some(DateTarget::Field{focus,part:None,adjustment:1}));
    }
    let map=template.element(5,InstallerTemplateRole::TimeZoneMap).unwrap().frame;
    assert!(matches!(installer_layout::installer_date_time_target_in(&template,i32::from(map.x+map.width/2),i32::from(map.y+map.height/2)),
        Some(DateTarget::Map{longitude:-1..=1})));
    assert_eq!(installer_layout::installer_date_time_target_in(&template,-1,-1),None);
    let mut moved=FACTORY_TEMPLATE.to_vec();
    let mut cursor=8usize;
    for _ in 0..u16::from_le_bytes([moved[6],moved[7]]) {
        let screen=moved[cursor];cursor+=1;
        cursor+=1+usize::from(moved[cursor]);
        let count=little_u16(&moved,&mut cursor);
        for _ in 0..count {
            let (role,kind)=skip_element(&moved,&mut cursor);
            if screen==5 && role==InstallerTemplateRole::DateField as u8 {
                moved[kind+6..kind+8].copy_from_slice(&100u16.to_le_bytes());
                moved[kind+8..kind+10].copy_from_slice(&350u16.to_le_bytes());
            }
            if screen==5 && role==InstallerTemplateRole::PrimaryButton as u8 {
                moved[kind+6..kind+8].copy_from_slice(&500u16.to_le_bytes());
                moved[kind+8..kind+10].copy_from_slice(&300u16.to_le_bytes());
            }
        }
    }
    let moved=InstallerTemplate::parse(&moved).unwrap();
    let moved_field=moved.element(5,InstallerTemplateRole::DateField).unwrap().frame;
    assert_eq!((moved_field.x,moved_field.y),(100,350));
    assert_eq!(installer_layout::installer_date_time_target_in(&moved,101,351),
        Some(DateTarget::Field{focus:2,part:None,adjustment:-1}));
    assert_eq!(installer_layout::installer_navigation_target_in(&moved,5,501,301),Some(1));
    let old_primary=template.element(5,InstallerTemplateRole::PrimaryButton).unwrap().frame;
    assert_eq!(installer_layout::installer_navigation_target_in(&moved,5,i32::from(old_primary.x+1),i32::from(old_primary.y+1)),None);
    let mut unordered=FACTORY_TEMPLATE.to_vec();
    let mut order_cursor=9usize;
    order_cursor+=1+unordered[order_cursor]as usize;
    let order_count=little_u16(&unordered,&mut order_cursor);
    for index in 0..order_count {
        let (_,kind)=skip_element(&unordered,&mut order_cursor);
        let z=((index as i16*17)%23)-11;
        unordered[kind+4..kind+6].copy_from_slice(&z.to_le_bytes());
    }
    let unsorted=InstallerTemplate::parse(&unordered).unwrap();
    let mut expected=(0..order_count).map(|index|unsorted.element_at(1,index).unwrap()).collect::<Vec<_>>();
    expected.sort_by_key(|element|(element.z_index,element.id));
    for (index,element) in expected.iter().enumerate() {
        assert_eq!(unsorted.layer_at(1,index as u16).unwrap().id,element.id);
    }
    let primary = first_primary_kind_offset(FACTORY_TEMPLATE);
    let mut styled = FACTORY_TEMPLATE.to_vec();
    styled[4..6].copy_from_slice(&7u16.to_le_bytes());
    for material in 0..3 {
        for corners in 0..3 {
            styled[primary+3] = (styled[primary+3]&3) | material<<2 | corners<<4;
            let parsed = InstallerTemplate::parse(&styled).unwrap();
            let button = parsed.element(1,InstallerTemplateRole::PrimaryButton).unwrap();
            assert_eq!(button.surface_material,material);
            assert_eq!(button.corner_style,corners);
        }
    }
    styled[primary+3] = 3<<2;
    assert!(InstallerTemplate::parse(&styled).is_err());
    styled[primary+3] = 3<<4;
    assert!(InstallerTemplate::parse(&styled).is_err());
    let mut legacy = FACTORY_TEMPLATE.to_vec();
    legacy[4..6].copy_from_slice(&6u16.to_le_bytes());
    let mut cursor = 8;
    let screens = u16::from_le_bytes([legacy[6],legacy[7]]);
    for _ in 0..screens {
        cursor += 1;
        cursor += 1 + legacy[cursor] as usize;
        let count = little_u16(&legacy, &mut cursor);
        for _ in 0..count {
            let (_,kind) = skip_element(&legacy,&mut cursor);
            legacy[kind+3] &= 3;
        }
    }
    let parsed = InstallerTemplate::parse(&legacy).unwrap();
    assert_eq!(parsed.element(1,InstallerTemplateRole::PrimaryButton).unwrap().surface_material,0);
    legacy[primary+3] |= 1<<2;
    assert!(InstallerTemplate::parse(&legacy).is_err());
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
        let first = template
            .element_at(screen, 0)
            .expect("screen must expose its first authored element");
        let resolved = template
            .element_by_identifier(screen, first.code_identifier)
            .expect("authored element identifier must resolve");
        assert_eq!(resolved.id, first.id);
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
        let count = settings.element_count(section).unwrap();
        let mut navigation_items = 0;
        let mut navigation_icons = 0;
        let mut navigation_labels = 0;
        let mut artwork = 0;
        let mut tabs = 0;
        let mut summary_cards = 0;
        let mut main_cards = 0;
        let mut sidebar_cards = 0;
        for layer in 0..count {
            let element = settings
                .layer_at(section, layer)
                .expect("Settings layer must resolve");
            match element.role {
                role if role == InstallerTemplateRole::SettingsNavigationItem as u8 => {
                    navigation_items += 1;
                    assert_eq!(element.kind, 1);
                }
                role if role == InstallerTemplateRole::SettingsNavigationIcon as u8 => {
                    navigation_icons += 1;
                    assert_eq!(element.kind, 2);
                    assert!(settings.asset(element.image_asset).is_some());
                }
                role if role == InstallerTemplateRole::SettingsNavigationLabel as u8 => {
                    navigation_labels += 1;
                    assert_eq!(element.kind, 3);
                }
                role if role == InstallerTemplateRole::SettingsArtwork as u8 => {
                    artwork += 1;
                    assert_eq!(element.kind, 2);
                    assert!(settings.asset(element.image_asset).is_some());
                }
                role if role == InstallerTemplateRole::SettingsTab as u8 => tabs += 1,
                role if role == InstallerTemplateRole::SettingsSummaryCard as u8 => {
                    summary_cards += 1
                }
                role if role == InstallerTemplateRole::SettingsMainCard as u8 => main_cards += 1,
                role if role == InstallerTemplateRole::SettingsSidebarCard as u8 => {
                    sidebar_cards += 1
                }
                _ => {}
            }
        }
        assert_eq!(navigation_items, 11);
        assert_eq!(navigation_icons, 11);
        assert_eq!(navigation_labels, 11);
        assert!(artwork <= 1);
        if section == 7 || section == 8 {
            assert_eq!(tabs, if section == 7 { 7 } else { 5 });
            assert_eq!(summary_cards, 1);
            assert_eq!(main_cards, 1);
            assert_eq!(sidebar_cards, 1);
        } else {
            assert_eq!(tabs + summary_cards + main_cards + sidebar_cards, 0);
        }
    }

    let mut altered = FACTORY_TEMPLATE.to_vec();
    let primary_kind = first_primary_kind_offset(&altered);
    altered[primary_kind] = 1;
    assert!(matches!(
        InstallerTemplate::parse(&altered),
        Err(installer_template::InstallerTemplateError::InvalidNavigation)
    ));
    let mut invalid_identifier = FACTORY_TEMPLATE.to_vec();
    let identifier = first_identifier_byte_offset(&invalid_identifier);
    invalid_identifier[identifier] = b'A';
    assert!(matches!(
        InstallerTemplate::parse(&invalid_identifier),
        Err(installer_template::InstallerTemplateError::InvalidElement)
    ));
}
