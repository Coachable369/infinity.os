use super::ConsoleRuntime;

impl ConsoleRuntime {
    // ------------------------=
    // FUNC: reload_editor_memory_profile
    // DESC: Applies a successfully saved owner profile to inference and wake recognition without restoring unrelated session workflows.
    // ------------------=
    pub(super) fn reload_editor_memory_profile(&self) {
        let Ok(document) = crate::storage::hermes_memory(self.current_user.0, self.current_session.0,
            b"", b"Infinity", None) else { return; };
        crate::runtime::ai::with_ai_runtime(|ai| ai.load_hermes_memory(self.current_session.0, Some(document)));
    }
}
