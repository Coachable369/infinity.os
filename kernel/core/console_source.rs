//! Owner-scoped source discovery for explicit existing-application updates.
use super::ConsoleRuntime;
use crate::ui::app_assistant::editor_compile_intent::SourceLanguage;
#[path = "../ui/editor_source_lookup.rs"]
mod lookup;

// ------------------------=
// FUNC: readable_source
// DESC: Requires a bounded native source document in this authenticated user's Personal space.
// ------------------=
fn readable_source(path: &[u8], user: [u8; 16]) -> Option<([u8; 16], u32)> {
    if !lookup::source_path(path) || user == [0; 16] { return None; }
    let (metadata, _) = crate::storage::object_inspect_path(path).ok()?;
    (metadata.logical_size > 0 && metadata.logical_size as usize <= crate::ui::text_editor::DOCUMENT_CAPACITY
        && metadata.owner.0 == user && metadata.space == crate::storage::object::Space::Personal)
        .then_some((metadata.object.id.0, metadata.current_version))
}

// ------------------------=
// FUNC: read_source_snapshot
// DESC: Reads the selected immutable source version and rechecks ownership, object identity and current version before granting an update receipt.
// ------------------=
fn read_source_snapshot(path: &[u8], user: [u8; 16], expected: ([u8; 16], u32), bytes: &mut [u8]) -> Result<usize, &'static [u8]> {
    let (object, length) = crate::storage::object_read_path(path, Some(expected.1), bytes)
        .map_err(|_| b"Cannot read the selected source version. Review the file and retry the update.".as_slice())?;
    if !lookup::snapshot_matches(expected, object.0, readable_source(path, user)) {
        return Err(b"The source changed while reading. Review the file and retry the update.");
    }
    Ok(length)
}

impl ConsoleRuntime {
    // ------------------------=
    // FUNC: resolve_assistant_update_source
    // DESC: Loads one unambiguous owned source without replacing unrelated unsaved work or guessing from anonymous source contents.
    // ------------------=
    pub(super) fn resolve_assistant_update_source(&mut self, target: &[u8]) -> Result<(SourceLanguage, [u8; 16], u32), &'static [u8]> {
        let authenticated = crate::runtime::with_runtime(|runtime| (0..crate::runtime::identity::MAX_SESSIONS)
            .filter_map(|i| runtime.identity.session_nth(i)).any(|session| session.id == self.current_session
                && session.user == self.current_user && session.state == crate::runtime::identity::SessionState::Active))
            .unwrap_or(false);
        if !authenticated { return Err(b"Sign in before updating an application source."); }
        let current = &self.editor_document_path[..self.editor_document_path_length];
        if let Some((object, version)) = readable_source(current, self.current_user.0) {
            if lookup::current_target(target) || lookup::matches(target, current) {
                if self.editor_document.bytes().is_empty() { return Err(b"The current source is empty; open the intended saved application source first."); }
                let mut snapshot = [0; crate::ui::text_editor::DOCUMENT_CAPACITY];
                let length = read_source_snapshot(current, self.current_user.0, (object, version), &mut snapshot)?;
                if !self.editor_document.saved_source_matches(&snapshot[..length]) {
                    return Err(b"The saved source changed since this document was opened. Reopen it before requesting an update; your editor contents were preserved.");
                }
                // The current buffer, including prior unsaved edits, is supplied
                // to the model by the caller. Its revision remains fenced until
                // the completed patch is applied; unrelated dirty tabs are not replaced.
                return Ok((if current.ends_with(b".py") { SourceLanguage::Python } else { SourceLanguage::C }, object, version));
            }
        }
        if lookup::current_target(target) { return Err(b"Open an owned C or Python source file first."); }
        if !self.editor_document.is_saved() { return Err(b"Save or discard your current edits before updating another application."); }
        let mut selected = [0; crate::ui::text_editor::DOCUMENT_PATH_CAPACITY];
        let mut length = 0;
        if target.starts_with(b"/") {
            if target.len() > selected.len() || readable_source(target, self.current_user.0).is_none() {
                return Err(b"The requested owned C or Python source file is unavailable.");
            }
            selected[..target.len()].copy_from_slice(target);
            length = target.len();
        } else if let Some(entry) = {
            let registry = crate::storage::generated_app_catalog(self.current_user.0, self.current_session.0, None)
                .map_err(|_| b"Cannot read your application catalog. No source was changed.".as_slice())?;
            match registry.find(self.current_user.0, target) {
                Ok(_) => Some(self.registered_application(target, false)?),
                Err(crate::runtime::generated_apps::Error::Missing) => None,
                Err(crate::runtime::generated_apps::Error::Ambiguous) => return Err(b"Several registered apps match. Specify the exact source path."),
                Err(_) => return Err(b"Cannot resolve this application safely. No source was changed."),
            }
        } {
            if entry.source_len > selected.len() { return Err(b"Registered source path exceeds the editor limit."); }
            selected[..entry.source_len].copy_from_slice(&entry.source[..entry.source_len]);
            length = entry.source_len;
        } else {
            for index in 0..256 {
                let entry = crate::storage::namespace_list_nth(crate::storage::object::DOCUMENTS_PATH, index)
                    .map_err(|_| b"Cannot inspect your Documents sources.".as_slice())?;
                let Some(entry) = entry else { break; };
                let path = &entry.path[..entry.path_len as usize];
                if lookup::matches(target, path) && readable_source(path, self.current_user.0).is_some() {
                    if length > 0 && selected[..length] != *path {
                        return Err(b"Several source files match. Specify the exact source path or open the intended file first.");
                    }
                    if path.len() > selected.len() { continue; }
                    selected[..path.len()].copy_from_slice(path);
                    length = path.len();
                }
            }
            if crate::storage::namespace_list_nth(crate::storage::object::DOCUMENTS_PATH, 256)
                .map_err(|_| b"Cannot inspect your Documents sources.".as_slice())?.is_some() {
                return Err(b"Your Documents source directory exceeds the bounded search. Open the intended file or specify its exact path.");
            }
        }
        if length == 0 { return Err(b"No matching owned source was found. Open its C or Python file, or specify its exact path."); }
        let (object, version) = readable_source(&selected[..length], self.current_user.0)
            .ok_or(b"The selected source changed before opening.".as_slice())?;
        let mut snapshot = [0; crate::ui::text_editor::DOCUMENT_CAPACITY];
        let content_length = read_source_snapshot(&selected[..length], self.current_user.0, (object, version), &mut snapshot)?;
        // Load the already verified version, not a second unpinned path read.
        // TextDocument::open validates before mutation; invalid source preserves the old document.
        if !self.editor_document.open(&snapshot[..content_length]) { return Err(b"The source could not be opened; your existing document was preserved."); }
        self.editor_document_path[..length].copy_from_slice(&selected[..length]);
        self.editor_document_path_length = length;
        let name = crate::runtime::object_navigation::namespace_basename(&selected[..length]);
        let name_length = name.len().min(self.editor_document_name.len());
        self.editor_document_name[..name_length].copy_from_slice(&name[..name_length]);
        self.editor_document_name_length = name_length;
        self.detect_editor_language();
        self.editor_scroll_row = 0;
        self.open_text_editor();
        Ok((if selected[..length].ends_with(b".py") { SourceLanguage::Python } else { SourceLanguage::C }, object, version))
    }
}
