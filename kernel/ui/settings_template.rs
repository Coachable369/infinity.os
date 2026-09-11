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
// FUNC: template
// DESC: Validates the compiled Settings document once; invalid documents remain rejected without repeated work in input and paint paths.
// ------------------=
pub fn template() -> Option<InstallerTemplate<'static>> {
    if TEMPLATE.state.load(Ordering::Acquire) != 2 {
        if TEMPLATE.state.compare_exchange(0, 1, Ordering::Acquire, Ordering::Acquire).is_ok() {
            let value = InstallerTemplate::parse_settings(super::installer_layout::SETTINGS_TEMPLATE_BYTES).ok();
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
