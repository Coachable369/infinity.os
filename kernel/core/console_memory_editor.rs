use super::ConsoleRuntime;

impl ConsoleRuntime {
    // ------------------------=
    // FUNC: reload_editor_memory_profile
    // DESC: Applies a successfully saved owner profile to inference and wake recognition without restoring unrelated session workflows.
    // ------------------=
    pub(super) fn reload_editor_memory_profile(&self) {
        let Ok(document) = crate::storage::hermes_memory(self.current_user.0, self.current_session.0,
            b"", b"Infinity", None) else { return; };
        let mut wake = [0; 16];
        if let Ok(length) = document.get(b"wake_word", &mut wake) {
            let value = if &wake[..length] == b"Computer" { crate::runtime::identity::WakeWord::Computer }
                else { crate::runtime::identity::WakeWord::Infinity };
            crate::runtime::with_runtime(|runtime| {
                let _ = runtime.identity.update_wake_word(self.current_user, self.current_user, value);
            });
        }
        crate::runtime::ai::with_ai_runtime(|ai| ai.load_hermes_memory(self.current_session.0, Some(document)));
    }
}
