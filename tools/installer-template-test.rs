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
const ACTION_EXTENSION_BYTES: usize = 21;

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
    if u16::from_le_bytes([data[4], data[5]]) >= 8 {
        *offset += ACTION_EXTENSION_BYTES;
    }
    (role, kind_offset)
}

#[derive(Clone)]
struct TestScreen {
    id: u8,
    title: Vec<u8>,
    elements: Vec<Vec<u8>>,
}

#[derive(Clone)]
struct TestTemplate {
    version: u16,
    screens: Vec<TestScreen>,
    assets: Vec<u8>,
}

impl TestTemplate {
    // ------------------------=
    // FUNC: read
    // DESC: Splits a valid native artifact into independently mutable binary screen and element records for negative tests.
    // ------------------=
    fn read(data: &[u8]) -> Self {
        let version = u16::from_le_bytes([data[4], data[5]]);
        let mut cursor = 8;
        let mut screens = Vec::new();
        for _ in 0..u16::from_le_bytes([data[6], data[7]]) {
            let id = data[cursor];
            cursor += 1;
            let title_length = data[cursor] as usize;
            cursor += 1;
            let title = data[cursor..cursor + title_length].to_vec();
            cursor += title_length;
            let count = little_u16(data, &mut cursor);
            let mut elements = Vec::new();
            for _ in 0..count {
                let start = cursor;
                skip_element(data, &mut cursor);
                elements.push(data[start..cursor].to_vec());
            }
            screens.push(TestScreen { id, title, elements });
        }
        Self { version, screens, assets: data[cursor..].to_vec() }
    }

    // ------------------------=
    // FUNC: encode
    // DESC: Reassembles binary records without repairing any intentionally malformed element contract.
    // ------------------=
    fn encode(&self) -> Vec<u8> {
        let mut bytes = b"IUIT".to_vec();
        bytes.extend_from_slice(&self.version.to_le_bytes());
        bytes.extend_from_slice(&(self.screens.len() as u16).to_le_bytes());
        for screen in &self.screens {
            bytes.push(screen.id);
            bytes.push(screen.title.len() as u8);
            bytes.extend_from_slice(&screen.title);
            bytes.extend_from_slice(&(screen.elements.len() as u16).to_le_bytes());
            for element in &screen.elements {
                bytes.extend_from_slice(element);
            }
        }
        bytes.extend_from_slice(&self.assets);
        bytes
    }

    // ------------------------=
    // FUNC: change_version
    // DESC: Converts record widths and removes newer controls when constructing a legacy compatibility fixture.
    // ------------------=
    fn change_version(&mut self, version: u16) {
        for screen in &mut self.screens {
            if version < 9 {
                screen.elements.retain(|element| {
                    element[element_kind_offset(element) + 1] < 34
                });
            }
            if version < 8 {
                screen.elements.retain(|element| {
                    element[element_kind_offset(element) + 1] != InstallerTemplateRole::SettingsRowAction as u8
                });
            }
            for element in &mut screen.elements {
                if self.version >= 8 && version < 8 {
                    element.truncate(element.len() - ACTION_EXTENSION_BYTES);
                } else if self.version < 8 && version >= 8 {
                    element.extend_from_slice(&[0; 17]);
                    element.extend_from_slice(&[255; 4]);
                }
                if version == 6 {
                    let flags = element_kind_offset(element) + 3;
                    element[flags] &= 3;
                }
            }
        }
        self.version = version;
    }
}

// ------------------------=
// FUNC: verify_settings_children_contract
// DESC: Exercises v9 parent-bound row children and malformed variants through the production binary parser, including real v8 compatibility.
// ------------------=
fn verify_settings_children_contract() {
    let valid = TestTemplate::read(SETTINGS_TEMPLATE);
    assert_eq!(valid.version, 10);
    let parsed = InstallerTemplate::parse_settings(SETTINGS_TEMPLATE).unwrap();
    let mut counts = [0usize; 5];
    for screen in &valid.screens {
        for index in 0..parsed.element_count(screen.id).unwrap() {
            let child = parsed.element_at(screen.id, index).unwrap();
            if !(34..=38).contains(&child.role) { continue; }
            counts[(child.role - 34) as usize] += 1;
            let parent = (0..parsed.element_count(screen.id).unwrap())
                .filter_map(|index| parsed.element_at(screen.id, index))
                .find(|candidate| candidate.id == child.parent_id).unwrap();
            assert_eq!(parent.role, InstallerTemplateRole::Metadata as u8);
        }
    }
    assert_eq!(counts, [40, 40, 40, 1, 1]);
    let legacy = template_at_version(SETTINGS_TEMPLATE, 8);
    let old = InstallerTemplate::parse_settings(&legacy).unwrap();
    assert_eq!(old.screen_count(), parsed.screen_count());
    assert!(old.element(1, InstallerTemplateRole::SettingsRowAction).is_some());
    for role in 34..=38 {
        let section = if role >= 37 { 4 } else { 0 };
        let index = valid.screens[section].elements.iter().position(|record| {
            record[element_kind_offset(record) + 1] == role
        }).unwrap();
        for mutation in 0..7 {
            let mut invalid = valid.clone();
            let record = &mut invalid.screens[section].elements[index];
            let parent = record.len() - ACTION_EXTENSION_BYTES;
            let kind = element_kind_offset(record);
            match mutation {
                0 => record[parent..parent + 16].fill(0),
                1 => record[parent..parent + 16].fill(0xcd),
                2 => record[kind] = 5,
                3 => { invalid.screens[section].elements.remove(index); }
                4 => {
                    let mut duplicate = record.clone();
                    duplicate[0] ^= 0xf0;
                    invalid.screens[section].elements.push(duplicate);
                }
                5 => invalid.version = 8,
                6 => {
                    let wrong = valid.screens[(section + 1) % valid.screens.len()].elements.iter()
                        .find(|candidate| candidate[element_kind_offset(candidate) + 1]
                            == InstallerTemplateRole::Metadata as u8).unwrap();
                    record[parent..parent + 16].copy_from_slice(&wrong[..16]);
                }
                _ => unreachable!(),
            }
            assert!(InstallerTemplate::parse_settings(&invalid.encode()).is_err(),
                "Unexpected accepted child role {role}, mutation {mutation}");
        }
    }
}

// ------------------------=
// FUNC: element_kind_offset
// DESC: Locates the fixed fields following an element UUID and variable-length binding identifier.
// ------------------=
fn element_kind_offset(element: &[u8]) -> usize {
    17 + element[16] as usize
}

// ------------------------=
// FUNC: template_at_version
// DESC: Produces a structurally valid requested-version fixture from whichever version the repository currently packages.
// ------------------=
fn template_at_version(data: &[u8], version: u16) -> Vec<u8> {
    let mut template = TestTemplate::read(data);
    template.change_version(version);
    template.encode()
}

// ------------------------=
// FUNC: action_record
// DESC: Constructs one v8 local-detail button with an explicit parent UUID and distinguishable authored visual properties.
// ------------------=
fn action_record(section: usize, row: usize, parent: &[u8]) -> Vec<u8> {
    let mut bytes = vec![0; 16];
    bytes[0] = 0xfe;
    bytes[1] = section as u8;
    bytes[2] = row as u8;
    let identifier = format!("settings.section.{}.row.{}.action", section, row);
    bytes.push(identifier.len() as u8);
    bytes.extend_from_slice(identifier.as_bytes());
    bytes.extend_from_slice(&[5, InstallerTemplateRole::SettingsRowAction as u8, 0, 1]);
    bytes.extend_from_slice(&30i16.to_le_bytes());
    for coordinate in [40u16, 700, 320, 180] {
        bytes.extend_from_slice(&coordinate.to_le_bytes());
    }
    bytes.extend_from_slice(&[12, 24, 36, 200, 144, 190, 255, 220]);
    bytes.extend_from_slice(&[82, 9]);
    bytes.extend_from_slice(&18u16.to_le_bytes());
    bytes.push(6);
    bytes.extend_from_slice(b"Action");
    bytes.extend_from_slice(&17u16.to_le_bytes());
    bytes.extend_from_slice(b"Configured action");
    bytes.push(0);
    bytes.extend_from_slice(&[0; 4]);
    bytes.extend_from_slice(parent);
    bytes.push(0);
    bytes.extend_from_slice(&[201, 222, 243, 188]);
    bytes
}

// ------------------------=
// FUNC: settings_with_actions
// DESC: Adds exactly the eight supported typed action bindings to real Settings records independently of checked-in format version.
// ------------------=
fn settings_with_actions() -> TestTemplate {
    let mut template = TestTemplate::read(SETTINGS_TEMPLATE);
    template.change_version(7);
    template.change_version(8);
    for screen in &mut template.screens {
        let section = screen.id as usize - 1;
        let parents: Vec<_> = screen.elements.iter().filter(|element| {
            element[element_kind_offset(element) + 1] == InstallerTemplateRole::Metadata as u8
        }).map(|element| element[..16].to_vec()).collect();
        for (row, parent) in parents.iter().enumerate() {
            if installer_template::settings_action_for_row(section, row).is_some() {
                screen.elements.push(action_record(section, row, parent));
            }
        }
    }
    template
}

// ------------------------=
// FUNC: first_action_index
// DESC: Locates a generated action record in a selected binary screen fixture.
// ------------------=
fn first_action_index(screen: &TestScreen) -> usize {
    screen.elements.iter().position(|element| {
        element[element_kind_offset(element) + 1] == InstallerTemplateRole::SettingsRowAction as u8
    }).unwrap()
}

// ------------------------=
// FUNC: verify_settings_action_contract
// DESC: Exercises native action identities, v6/v7 compatibility, v8 parent resolution and bounded malformed-payload rejection.
// ------------------=
fn verify_settings_action_contract() {
    use installer_template::{settings_action_for_row, InstallerTemplateError, SettingsAction};
    let expected = [
        (0, 0, SettingsAction::RenameMachine),
        (3, 0, SettingsAction::CyclePolicy),
        (3, 1, SettingsAction::ToggleChat),
        (3, 2, SettingsAction::CycleModel),
        (3, 4, SettingsAction::ToggleMicrophone),
        (3, 5, SettingsAction::CycleWakeWord),
        (3, 6, SettingsAction::ToggleSpeech),
        (4, 1, SettingsAction::ToggleMicrophone),
    ];
    for section in 0..12 {
        for row in 0..10 {
            let action = expected.iter().find(|entry| entry.0 == section && entry.1 == row).map(|entry| entry.2);
            assert_eq!(settings_action_for_row(section, row), action);
        }
    }
    assert_eq!(settings_action_for_row(usize::MAX, usize::MAX), None);
    for (source, is_settings) in [(FACTORY_TEMPLATE, false), (CONFIGURATION_TEMPLATE, false), (SETTINGS_TEMPLATE, true)] {
        for version in [6, 7] {
            let bytes = template_at_version(source, version);
            let parsed = if is_settings {
                InstallerTemplate::parse_settings(&bytes).unwrap()
            } else {
                InstallerTemplate::parse(&bytes).unwrap()
            };
            for screen in 1..=parsed.screen_count() as u8 {
                for index in 0..parsed.element_count(screen).unwrap() {
                    let element = parsed.element_at(screen, index).unwrap();
                    assert_eq!(element.parent_id, [0; 16]);
                    assert_eq!(element.caption_mode, 0);
                    assert_eq!(element.caption_color, [255; 4]);
                    assert_ne!(element.role, InstallerTemplateRole::SettingsRowAction as u8);
                }
            }
        }
    }
    let valid = settings_with_actions();
    let bytes = valid.encode();
    let parsed = InstallerTemplate::parse_settings(&bytes).unwrap();
    let mut actions = 0;
    for (section, row, expected_action) in expected {
        let screen = section as u8 + 1;
        let parent = (0..parsed.element_count(screen).unwrap()).filter_map(|index| parsed.element_at(screen, index))
            .filter(|element| element.role == InstallerTemplateRole::Metadata as u8).nth(row).unwrap();
        let action = (0..parsed.element_count(screen).unwrap()).filter_map(|index| parsed.element_at(screen, index))
            .find(|element| element.role == InstallerTemplateRole::SettingsRowAction as u8 && element.parent_id == parent.id).unwrap();
        assert_eq!(settings_action_for_row(section, row), Some(expected_action));
        assert_eq!((action.kind, action.caption_mode, action.opacity, action.font_size), (5, 0, 82, 18));
        assert_eq!((action.frame.x, action.frame.y, action.frame.width, action.frame.height), (40, 700, 320, 180));
        assert_eq!(action.fill, [12, 24, 36, 200]);
        assert_eq!(action.border, [144, 190, 255, 220]);
        assert_eq!(action.caption_color, [201, 222, 243, 188]);
        actions += 1;
    }
    assert_eq!(actions, 8);
    let first = first_action_index(&valid.screens[0]);
    let mut literal = valid.clone();
    let action = &mut literal.screens[0].elements[first];
    let mode = action.len() - 5;
    action[mode] = 1;
    let literal_bytes = literal.encode();
    let literal_action = InstallerTemplate::parse_settings(&literal_bytes).unwrap()
        .element(1, InstallerTemplateRole::SettingsRowAction).unwrap();
    assert_eq!(literal_action.caption_mode, 1);
    assert_eq!(literal_action.caption_color, [201, 222, 243, 188]);
    assert_eq!(literal_action.text, parsed.element(1, InstallerTemplateRole::SettingsRowAction).unwrap().text);

    for mutation in 0..14 {
        let mut invalid = valid.clone();
        let action_index = first_action_index(&invalid.screens[0]);
        let action_length = invalid.screens[0].elements[action_index].len();
        let parent_start = action_length - ACTION_EXTENSION_BYTES;
        match mutation {
            0 => invalid.screens[0].elements[action_index][parent_start..parent_start + 16].fill(0),
            1 => invalid.screens[0].elements[action_index][parent_start..parent_start + 16].fill(0xa5),
            2 => {
                let wrong = invalid.screens[0].elements.iter().find(|element| {
                    element[element_kind_offset(element) + 1] == InstallerTemplateRole::Content as u8
                }).unwrap()[..16].to_vec();
                invalid.screens[0].elements[action_index][parent_start..parent_start + 16].copy_from_slice(&wrong);
            }
            3 => {
                let other = invalid.screens[1].elements.iter().find(|element| {
                    element[element_kind_offset(element) + 1] == InstallerTemplateRole::Metadata as u8
                }).unwrap()[..16].to_vec();
                invalid.screens[0].elements[action_index][parent_start..parent_start + 16].copy_from_slice(&other);
            }
            4 => {
                let duplicate = invalid.screens[0].elements[action_index].clone();
                invalid.screens[0].elements.push(duplicate);
            }
            5 => { invalid.screens[0].elements.remove(action_index); }
            6 => {
                let unsupported = invalid.screens[0].elements.iter().filter(|element| {
                    element[element_kind_offset(element) + 1] == InstallerTemplateRole::Metadata as u8
                }).nth(1).unwrap()[..16].to_vec();
                invalid.screens[0].elements[action_index][parent_start..parent_start + 16].copy_from_slice(&unsupported);
            }
            7 => invalid.screens[0].elements[action_index][action_length - 5] = 2,
            8 => {
                let kind = element_kind_offset(&invalid.screens[0].elements[action_index]);
                invalid.screens[0].elements[action_index][kind] = 1;
            }
            9 => {
                let parent = invalid.screens[0].elements[action_index][parent_start..parent_start + 16].to_vec();
                let other = invalid.screens[0].elements.iter_mut().find(|element| {
                    element[element_kind_offset(element) + 1] == InstallerTemplateRole::Content as u8
                }).unwrap();
                other[..16].copy_from_slice(&parent);
            }
            10 => {
                let ordinary = &mut invalid.screens[0].elements[0];
                let parent = ordinary.len() - ACTION_EXTENSION_BYTES;
                ordinary[parent] = 1;
            }
            11 => {
                let ordinary = &mut invalid.screens[0].elements[0];
                let caption = ordinary.len() - 5;
                ordinary[caption] = 1;
            }
            12 => {
                let kind = element_kind_offset(&invalid.screens[0].elements[action_index]);
                invalid.screens[0].elements[action_index][kind + 1] = 34;
            }
            13 => {
                let ordinary = invalid.screens[0].elements.iter_mut().find(|element| {
                    element[element_kind_offset(element)] != 5
                }).unwrap();
                let color = ordinary.len() - 4;
                ordinary[color] = 1;
            }
            _ => unreachable!(),
        }
        assert!(matches!(InstallerTemplate::parse_settings(&invalid.encode()), Err(InstallerTemplateError::InvalidElement)), "mutation {}", mutation);
    }
    for source in [FACTORY_TEMPLATE, CONFIGURATION_TEMPLATE] {
        let mut wizard = TestTemplate::read(&template_at_version(source, 8));
        let primary = wizard.screens[0].elements.iter_mut().find(|element| {
            element[element_kind_offset(element) + 1] == InstallerTemplateRole::PrimaryButton as u8
        }).unwrap();
        let color = primary.len() - 4;
        primary[color..].copy_from_slice(&[90, 140, 210, 180]);
        let colored = wizard.encode();
        assert_eq!(InstallerTemplate::parse(&colored).unwrap()
            .element(1, InstallerTemplateRole::PrimaryButton).unwrap().caption_color, [90, 140, 210, 180]);
        wizard.screens[0].elements.push(valid.screens[0].elements[first].clone());
        assert!(matches!(InstallerTemplate::parse(&wizard.encode()), Err(InstallerTemplateError::InvalidElement)));
    }
    let mut missing = valid.clone();
    for screen in &mut missing.screens {
        screen.elements.retain(|element| element[element_kind_offset(element) + 1] != InstallerTemplateRole::SettingsRowAction as u8);
    }
    assert!(matches!(InstallerTemplate::parse_settings(&missing.encode()), Err(InstallerTemplateError::InvalidElement)));
    for version in [6, 7] {
        let mut legacy = TestTemplate::read(&template_at_version(SETTINGS_TEMPLATE, version));
        let mut action = valid.screens[0].elements[first].clone();
        action.truncate(action.len() - ACTION_EXTENSION_BYTES);
        legacy.screens[0].elements.push(action);
        assert!(matches!(InstallerTemplate::parse_settings(&legacy.encode()), Err(InstallerTemplateError::InvalidElement)));
    }
    let mut truncated = valid.clone();
    truncated.assets.clear();
    let mut truncated_bytes = truncated.encode();
    truncated_bytes.truncate(truncated_bytes.len() - 1);
    assert!(matches!(InstallerTemplate::parse_settings(&truncated_bytes), Err(InstallerTemplateError::Truncated)));
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
// FUNC: template_with_screen_count
// DESC: Builds a bounded wizard fixture from a real authored screen while preserving its native elements and image table.
// ------------------=
fn template_with_screen_count(data: &[u8], count: u16) -> Vec<u8> {
    let mut cursor = 8;
    let mut first_end = cursor;
    for index in 0..u16::from_le_bytes([data[6], data[7]]) {
        cursor += 1;
        cursor += 1 + data[cursor] as usize;
        let elements = little_u16(data, &mut cursor);
        for _ in 0..elements {
            skip_element(data, &mut cursor);
        }
        if index == 0 { first_end = cursor; }
    }
    let mut result = data[..8].to_vec();
    result[6..8].copy_from_slice(&count.to_le_bytes());
    for screen in 1..=count {
        result.push(screen as u8);
        result.extend_from_slice(&data[9..first_end]);
    }
    result.extend_from_slice(&data[cursor..]);
    result
}

// ------------------------=
// FUNC: main
// DESC: Exercises authored runtime structure, geometry consumption, data bindings, and action-integrity rejection.
// ------------------=
fn main() {
    use installer_layout::InstallerDateTimeTarget as DateTarget;
    verify_settings_action_contract();
    verify_settings_children_contract();
    for source in [FACTORY_TEMPLATE, CONFIGURATION_TEMPLATE] {
        for count in [1, 11, 12, 32] {
            let bytes = template_with_screen_count(source, count);
            let parsed = InstallerTemplate::parse(&bytes).unwrap();
            assert_eq!(parsed.screen_count(), count);
            let last = parsed.element(count as u8, InstallerTemplateRole::PrimaryButton).unwrap();
            let first = parsed.element(1, InstallerTemplateRole::PrimaryButton).unwrap();
            assert_eq!(last.frame, first.frame);
            assert_eq!(last.fill, first.fill);
            assert!(InstallerTemplate::parse_settings(&bytes).is_err());
        }
        for count in [0, 33] {
            assert!(InstallerTemplate::parse(&template_with_screen_count(source, count)).is_err());
        }
    }
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
    let mut styled = template_at_version(FACTORY_TEMPLATE, 7);
    let primary = first_primary_kind_offset(&styled);
    for material in 0..4 {
        for corners in 0..3 {
            styled[primary+3] = (styled[primary+3]&3) | material<<2 | corners<<4;
            let parsed = InstallerTemplate::parse(&styled).unwrap();
            let button = parsed.element(1,InstallerTemplateRole::PrimaryButton).unwrap();
            assert_eq!(button.surface_material,material);
            assert_eq!(button.corner_style,corners);
        }
    }
    styled[primary+3] = 1<<6;
    assert!(InstallerTemplate::parse(&styled).is_err());
    styled[primary+3] = 3<<4;
    assert!(InstallerTemplate::parse(&styled).is_err());
    let mut legacy = template_at_version(FACTORY_TEMPLATE, 6);
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
