//! The packaged Settings document is immutable for the lifetime of a System Generation.
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicU8, Ordering};
use super::installer_template::{InstallerTemplate, InstallerTemplateElement, InstallerTemplateRole};

struct ValidatedTemplate {
    state: AtomicU8,
    value: UnsafeCell<Option<InstallerTemplate<'static>>>,
    elements: UnsafeCell<[[Option<InstallerTemplateElement<'static>>; 128]; 11]>,
    counts: UnsafeCell<[u16; 11]>,
    layers: UnsafeCell<[[u16; 128]; 11]>,
}

// The winning initializer is the only writer. Acquire readers see an immutable value.
unsafe impl Sync for ValidatedTemplate {}

static TEMPLATE: ValidatedTemplate = ValidatedTemplate {
    state: AtomicU8::new(0),
    value: UnsafeCell::new(None),
    elements: UnsafeCell::new([[None; 128]; 11]),
    counts: UnsafeCell::new([0; 11]),
    layers: UnsafeCell::new([[0; 128]; 11]),
};

// ------------------------=
// FUNC: element_identity
// DESC: Resolves a stable layer ID and its source-order role ordinal from validated cached elements without repeatedly parsing the document.
// ------------------=
pub fn element_identity(
    section: usize,
    id: [u8; 16],
) -> Option<(InstallerTemplateElement<'static>, usize)> {
    let template = template()?;
    if section >= 11 { return None; }
    let count = unsafe { (*TEMPLATE.counts.get())[section] };
    let at = |index: u16| {
        if index < 128 {
            unsafe { (*TEMPLATE.elements.get())[section][index as usize] }
        } else {
            template.element_at(section as u8 + 1, index)
        }
    };
    for index in 0..count {
        let element = at(index)?;
        if element.id == id {
            let ordinal = (0..index).filter(|previous| at(*previous)
                .is_some_and(|candidate| candidate.role == element.role)).count();
            return Some((element, ordinal));
        }
    }
    None
}

// ------------------------=
// FUNC: row_action
// DESC: Finds the validated action child by its parent row identity, never by editable text or sparse action order.
// ------------------=
pub fn row_action(section: usize, row: usize) -> Option<InstallerTemplateElement<'static>> {
    row_child(section, row, InstallerTemplateRole::SettingsRowAction)
}

// ------------------------=
// FUNC: row_child
// DESC: Resolves a semantic child by its trusted Metadata parent identity regardless of source or paint ordering.
// ------------------=
pub fn row_child(section: usize, row: usize, role: InstallerTemplateRole) -> Option<InstallerTemplateElement<'static>> {
    let parent = role_at(section, InstallerTemplateRole::Metadata, row)?;
    for ordinal in 0..8 {
        let child = role_at(section, role, ordinal)?;
        if child.parent_id == parent.id { return Some(child); }
    }
    None
}

// ------------------------=
// FUNC: settings_detail_description
// DESC: Supplies trusted native explanatory captions while literal mode can author different presentation without changing the associated action.
// ------------------=
pub fn settings_detail_description(section: usize, row: usize) -> &'static [u8] {
    match (section, row) {
        (0, 0) => b"Rename this machine through the durable identity service.",
        (0, 1) => b"The installed interface language is English (US).",
        (0, 2) => b"Regional formatting follows the installed US locale.",
        (0, 3) => b"This is the active installed System Generation.",
        (0, 4) => b"Updates are delivered as a complete System Generation.",
        (1, 0) => b"Switch between installed, verified InfinityUI skins.",
        (1, 6) => b"Automatic scale follows the active display density.",
        (1, 7) => b"Cosmic Horizon is the active packaged desktop wallpaper.",
        (2, 0) => b"The authenticated user owns this desktop session.",
        (2, 1) => b"Password credentials are managed by the trusted identity flow.",
        (2, 2) => b"This session is authenticated; locking preserves your workspace.",
        (2, 3) => b"Personal Space isolates this user's objects and preferences.",
        (2, 4) => b"The user profile persists with the installed system.",
        (3, 0) => b"Choose whether the local provider is strictly required or preferred.",
        (3, 1) => b"Show or hide your local desktop AI chat widget.",
        (3, 2) => b"Choose the next installed local model without restarting.",
        (3, 3) => b"Remote processing is disabled; local inference stays on this device.",
        (3, 4) => b"Microphone access follows the current user permission.",
        (3, 5) => b"Choose the phrase that opens an eight-second command window.",
        (3, 6) => b"Enable or mute spoken AI replies.",
        (3, 7) => b"Heart / Bella / Nicole (female); Adam / Michael (male).",
        (4, 0) => b"Applications receive no implicit authority over your data.",
        (4, 1) => b"Microphone access is explicitly granted or restricted here.",
        (4, 2) => b"Remote AI access requires explicit capability authorization.",
        (4, 3) => b"Lock this user's session after the selected period without input.",
        (4, 4) => b"Select this row to toggle facial sign-in. Requires enrollment. RGB matching has no anti-spoof protection; password sign-in remains available.",
        (5, 0) => b"The active display is used by the native desktop compositor.",
        (5, 1) => b"Open Input to adjust keyboard repeat delay and rate.",
        (5, 3 | 4) => b"This audio path is unavailable; no device control is exposed.",
        (9, 0) => b"InfinityOS development System Generation.",
        (9, 1) => b"The desktop and services execute in the native OS runtime.",
        (9, 2) => b"Startup uses the installed boot contract and generation checks.",
        (9, 3) => b"Identity records use the version 1 durable format.",
        (9, 4) => b"Select an installed icon family in Themes & Skins.",
        (10, _) => super::settings_cards::description(section, row),
        _ => b"This value is read from the active System Generation.",
    }
}

// ------------------------=
// FUNC: template
// DESC: Validates the compiled Settings document once; invalid documents remain rejected without repeated work in input and paint paths.
// ------------------=
pub fn template() -> Option<InstallerTemplate<'static>> {
    if TEMPLATE.state.load(Ordering::Acquire) != 2 {
        if TEMPLATE.state.compare_exchange(0, 1, Ordering::Acquire, Ordering::Acquire).is_ok() {
            #[cfg(studio_native_preview)]
            let bytes=crate::studio_native_render::settings_template_bytes()
                .unwrap_or(super::installer_layout::SETTINGS_TEMPLATE_BYTES);
            #[cfg(not(studio_native_preview))]
            let bytes=super::installer_layout::SETTINGS_TEMPLATE_BYTES;
            let value = InstallerTemplate::parse_settings(bytes).ok();
            unsafe { *TEMPLATE.value.get() = value; }
            if let Some(template) = value {
                for section in 0..11 {
                    let count = template.element_count(section as u8 + 1).unwrap_or(0);
                    unsafe { (*TEMPLATE.counts.get())[section] = count; }
                    for index in 0..count.min(128) {
                        unsafe { (*TEMPLATE.elements.get())[section][index as usize] =
                            template.element_at(section as u8 + 1, index); }
                    }
                    if count <= 128 {
                        // Insertion-sort indices once, never reparse the document to sort a frame.
                        for index in 0..count as usize {
                            unsafe {
                                let elements = &(*TEMPLATE.elements.get())[section];
                                let layers = &mut (*TEMPLATE.layers.get())[section];
                                let candidate = elements[index].unwrap();
                                let mut position = index;
                                while position > 0 {
                                    let previous = elements[layers[position - 1] as usize].unwrap();
                                    if (previous.z_index, previous.id) <= (candidate.z_index, candidate.id) { break; }
                                    layers[position] = layers[position - 1];
                                    position -= 1;
                                }
                                layers[position] = index as u16;
                            }
                        }
                    }
                }
            }
            TEMPLATE.state.store(2, Ordering::Release);
        } else {
            // A reentrant paint can use fallback geometry until initialization completes.
            return None;
        }
    }
    unsafe { *TEMPLATE.value.get() }
}

// ------------------------=
// FUNC: layer_at
// DESC: Returns the cached Studio paint order in constant time, avoiding repeated nested document scans during eased scrolling.
// ------------------=
pub fn layer_at(section: usize, layer: u16) -> Option<InstallerTemplateElement<'static>> {
    let template = template()?;
    if section >= 11 { return None; }
    let count = unsafe { (*TEMPLATE.counts.get())[section] };
    if layer >= count { return None; }
    if count > 128 { return template.layer_at(section as u8 + 1, layer); }
    unsafe { (*TEMPLATE.elements.get())[section][(*TEMPLATE.layers.get())[section][layer as usize] as usize] }
}

// ------------------------=
// FUNC: element
// DESC: Looks up one singular Settings role in the validated packaged document.
// ------------------=
pub fn element(section: usize, role: InstallerTemplateRole) -> Option<InstallerTemplateElement<'static>> {
    role_at(section, role, 0)
}

// ------------------------=
// FUNC: role_at
// DESC: Looks up a repeated Settings role without revalidating every screen on each pointer movement.
// ------------------=
pub fn role_at(section: usize, role: InstallerTemplateRole, ordinal: usize) -> Option<InstallerTemplateElement<'static>> {
    let template = template()?;
    if section >= 11 { return None; }
    let screen = section.saturating_add(1) as u8;
    let mut found = 0;
    for index in 0..unsafe { (*TEMPLATE.counts.get())[section] } {
        let element = if index < 128 {
            unsafe { (*TEMPLATE.elements.get())[section][index as usize]? }
        } else { template.element_at(screen, index)? };
        if element.role == role as u8 {
            if found == ordinal { return Some(element); }
            found += 1;
        }
    }
    None
}
