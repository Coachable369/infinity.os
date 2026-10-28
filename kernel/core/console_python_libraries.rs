//! Read-only, profile-scoped library context for Python generation and edits.
use super::ConsoleRuntime;
use crate::storage::object::{PythonLibraryCatalog, PythonPackageRevision, format_python_library_context};
pub(super) const CONTEXT_BYTES: usize = crate::storage::object::PYTHON_LIBRARY_CONTEXT_BYTES;
static mut CACHED: Option<([u8; 16], PythonPackageRevision, PythonLibraryCatalog)> = None;

impl ConsoleRuntime {
    // ------------------------=
    // FUNC: append_python_libraries
    // DESC: Appends bounded sanitized installed import roots after an authenticated committed-revision lookup, never running package code or trusting metadata instructions.
    // ------------------=
    pub(super) fn append_python_libraries(&self, output: &mut [u8], start: usize, objective: &[u8]) -> Option<usize> {
        let mut directory = [0; CONTEXT_BYTES];
        let cached = unsafe { CACHED }.filter(|entry| entry.0 == self.current_user.0).map(|entry| (entry.1, entry.2));
        let catalog = crate::storage::python_library_catalog(self.current_user.0, self.current_session.0, cached);
        let result = match catalog {
            Ok((revision, catalog)) => {
                unsafe { CACHED = revision.map(|revision| (self.current_user.0, revision, catalog)); }
                format_python_library_context(Some(&catalog), &mut directory)?
            }
            Err(_) => {
                unsafe { CACHED = None; }
                format_python_library_context(None, &mut directory)?
            }
        };
        crate::ui::app_assistant::python_generation_context::append_directory(
            output, start, objective, &directory[..result.length])
    }
}
