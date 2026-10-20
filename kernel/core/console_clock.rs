//! Authenticated clock dispatch uses the ordinary AIKit execution broker and native storage.
use super::*;
use crate::runtime::ai::{clock, kit::RequestOrigin};

// ------------------------=
// FUNC: load_settings
// DESC: Restores the clock checkpoint on live or installed boot without inventing a firmware time.
// ------------------=
pub(super) fn load_settings() {
    if let Some(settings) = crate::storage::installed_clock_settings() { clock::SYSTEM.configure(settings); }
}

// ------------------------=
// FUNC: hardware_utc
// DESC: Reads validated UTC from the BSP firmware clock and publishes a worker-safe epoch sample.
// ------------------=
pub(super) fn hardware_utc(services: u64) -> Option<u64> {
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    {
        if services == 0 { return None; }
        let mut time = EfiTime::zero();
        if unsafe { ((*(services as *const EfiRuntimeServices)).get_time)(&mut time, core::ptr::null_mut()) } != 0 { return None; }
        let seconds = crate::http_transport::clock::unix_seconds(time.year, time.month, time.day,
            time.hour, time.minute, time.second, time.time_zone)?;
        clock::SYSTEM.observe(seconds, crate::runtime::ai::qwen::workers::clock_ns());
        Some(seconds)
    }
    #[cfg(target_arch = "x86")]
    { let _ = services; None }
}

// ------------------------=
// FUNC: configuration
// DESC: Converts exact native local fields to the existing desktop/calendar presentation type.
// ------------------=
fn configuration(local: clock::LocalTime, settings: clock::Settings) -> DateTimeConfiguration {
    DateTimeConfiguration { year: local.year, month: local.month, day: local.day,
        hour: local.hour, minute: local.minute, second: local.second,
        time_zone_id: TIME_ZONES[time_zone_index(settings.offset_minutes)].id,
        utc_offset_minutes: settings.offset_minutes }
}

// ------------------------=
// FUNC: local_configuration
// DESC: Projects the actual system clock using its persisted fixed UTC offset.
// ------------------=
pub(super) fn local_configuration(services: u64) -> Option<DateTimeConfiguration> {
    let hardware = hardware_utc(services)?;
    let settings = clock::SYSTEM.settings();
    clock::local_time(settings.utc(hardware)?, settings.offset_minutes).map(|local| configuration(local, settings))
}

impl ConsoleRuntime {
    // ------------------------=
    // FUNC: dispatch_clock
    // DESC: Checks current session Settings rights and stores real clock evidence for presentation and speech.
    // ------------------=
    pub(super) fn dispatch_clock(&mut self, request: clock::Request) -> bool {
        use crate::runtime::identity::{SessionState, SESSION_SETTINGS_READ, SESSION_SETTINGS_UPDATE_ALLOWED};
        let permissions = crate::runtime::with_runtime(|runtime| {
            (0..crate::runtime::identity::MAX_SESSIONS).filter_map(|i| runtime.identity.session_nth(i))
                .find(|session| session.id == self.current_session && session.user == self.current_user
                    && session.state == SessionState::Active).map(|session| session.capabilities).unwrap_or(0)
        }).unwrap_or(0);
        let raw = hardware_utc(self.system.firmware_runtime_services);
        let receipt = clock::execute(request, raw, clock::SYSTEM.settings(),
            permissions & SESSION_SETTINGS_READ != 0,
            permissions & SESSION_SETTINGS_UPDATE_ALLOWED != 0,
            |settings, local| crate::storage::update_system_clock(self.current_user.0, self.current_session.0,
                configuration(local, settings), settings.correction_seconds));
        if receipt.outcome == clock::Outcome::Updated { clock::SYSTEM.configure(receipt.settings); }
        if let Some(local) = receipt.local { self.desktop_clock = configuration(local, receipt.settings); }
        self.clock_receipt = Some(receipt);
        matches!(receipt.outcome, clock::Outcome::Read | clock::Outcome::Updated)
    }

    // ------------------------=
    // FUNC: publish_clock_receipt
    // DESC: Replaces generic action copy with the verified clock result before chat/TTS or an app panel observes completion.
    // ------------------=
    pub(super) fn publish_clock_receipt(&mut self, origin: Option<RequestOrigin>) {
        let Some(receipt) = self.clock_receipt.take() else { return; };
        let mut reply = [0; 384];
        let length = clock::format(receipt, &mut reply);
        if let Some(RequestOrigin::Application(id)) = origin {
            let mut panel = crate::ui::app_assistant::read(id); panel.reply(&reply[..length]);
            crate::ui::app_assistant::write(id, panel);
        } else {
            crate::runtime::ai::with_ai_runtime(|ai| { ai.chat.update_native_response(&reply[..length]); });
        }
    }

    // ------------------------=
    // FUNC: clock_command
    // DESC: Exposes the same authorized service to the native console without invoking inference.
    // ------------------=
    pub(super) fn clock_command(&mut self, input: &[u8]) -> bool {
        let Some(request) = clock::Request::parse(input) else { return false; };
        self.dispatch_clock(request);
        if let Some(receipt) = self.clock_receipt.take() {
            let mut reply = [0; 384]; let length = clock::format(receipt, &mut reply);
            self.output.write_line(&reply[..length]);
        }
        true
    }
}
