//! Versioned shared live/installed application contract and non-secret behavioral inspection.
pub const MAGIC: u64 = 0x494e464150505331;
pub const VERSION: u64 = 1;
pub const FEATURES: u64 = 0x1ff; // syntax, selection, history, find, replace, indent, navigation, assistant, reviewed actions
#[no_mangle]
pub static mut INFINITY_APP_FEATURE_SNAPSHOT: [u64; 32] = [
    MAGIC,
    VERSION,
    0,
    FEATURES,
    super::text_editor::DOCUMENT_CAPACITY as u64,
    8,
    9,
    0,
    0,
    0,
    0,
    1,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
];
// ------------------------=
// FUNC: publish
// DESC: Exposes only structured non-secret editor and assistant state for fresh-install behavioral parity checks.
// ------------------=
pub fn publish(
    document: &super::text_editor::TextDocument,
    owner: Option<(usize, super::geometry::Rect)>,
    scale: usize,
) {
    unsafe {
        let p = &mut *(&raw mut INFINITY_APP_FEATURE_SNAPSHOT);
        p[2] = p[2].wrapping_add(1) | 1;
        p[7] = document.bytes().len() as u64;
        p[8] = document.cursor() as u64;
        let selection = document
            .selection()
            .unwrap_or((document.cursor(), document.cursor()));
        p[9] = selection.0 as u64;
        p[10] = selection.1 as u64;
        p[11] = document.is_saved() as u64;
        p[12] = owner.map_or(u64::MAX, |(i, _)| i as u64);
        let panel = super::app_assistant::read(owner.map_or(usize::MAX, |v| v.0));
        p[13] = panel.expanded as u64;
        p[14] = panel.pending as u64;
        p[15] = panel.focused as u64;
        p[16] = super::app_assistant::fingerprint(document.bytes(), 0, None);
        p[17] = super::editor_tools::current().field as u64;
        p[18] = panel.length as u64;
        p[19] = panel.response_len as u64;
        p[20] = super::editor_tools::current().language as u64;
        let r = owner.map_or(super::geometry::Rect::default(), |v| v.1);
        p[21] = r.x as u64;
        p[22] = r.y as u64;
        p[23] = r.width as u64;
        p[24] = r.height as u64;
        let view = super::editor_tools::current();
        p[25] = view.menu as u64;
        p[26] = view.menu_index as u64;
        let g = super::editor_chrome::Layout::new(r, scale, panel.expanded, view.field);
        p[27] = g.body.width as u64;
        p[28] = g.body.height as u64;
        p[29] = g.body.y as u64;
        p[30] = g.body.x as u64;
        p[2] = p[2].wrapping_add(1) & !1;
        p[31] = p[2];
    }
}
