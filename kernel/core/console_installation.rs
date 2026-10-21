use super::{ConsoleMode, InstallerStep, StorageError, StorageManager, RUNTIME, INPUT_PRESENTATION};

// ------------------------=
// FUNC: poll
// DESC: Executes an explicitly confirmed installation once, after its new scene is visible and all device/runtime input borrows ended.
// ------------------=
pub fn poll(mut service_input: impl FnMut()) {
    let plan = unsafe {
        let Some(runtime) = (&mut *(&raw mut RUNTIME)).as_mut() else { return; };
        if !runtime.installer_work.begin((&*(&raw const INPUT_PRESENTATION)).active()) { return; }
        runtime.storage_plan
    };
    crate::bootstrap::installer_progress_update(0, b"PREPARING INSTALLATION");
    let mut last_service = None;
    let mut heartbeat = || {
        let now = crate::ui::performance::monotonic_ns();
        if now.zip(last_service).is_some_and(|(now, last)| now.saturating_sub(last) < 16_000_000) { return; }
        last_service = now;
        service_input();
    };
    let authorized = crate::runtime::with_runtime(|runtime| runtime.installer_authorized(0)).unwrap_or(false);
    let result = if !authorized { Err(StorageError::InvalidPlan) } else {
        plan.as_ref().map(|plan| StorageManager::provision_interactive(plan,
            &mut |percent, label| crate::bootstrap::installer_progress_update(percent, label),
            &mut heartbeat)).unwrap_or(Err(StorageError::NoDevice))
    };
    // Drain queued button/key edges under the busy gate, not after displaying a completion action.
    service_input();
    unsafe { if let Some(runtime) = (&mut *(&raw mut RUNTIME)).as_mut() {
        runtime.installer_work.finish();
        runtime.mode = ConsoleMode::Installer;
        match result {
            Ok(()) => {
                runtime.installer_step = InstallerStep::Complete;
                crate::runtime::with_runtime(|runtime| {
                    runtime.installer_record(crate::runtime::EVENT_INSTALLER_COMPLETED, 0);
                    runtime.revoke_installer_authority();
                });
            }
            Err(error) => {
                runtime.installer_error = Some(error);
                super::report_storage_error(error);
                runtime.installer_step = InstallerStep::Failed;
            }
        }
        runtime.installer_focus = runtime.default_installer_focus();
        runtime.render_installer();
        runtime.redraw();
    } }
}
