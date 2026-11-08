//! Authenticated command and Task Manager adapter for the same durable job catalog.
use super::ConsoleRuntime;
use crate::runtime::automation::{self, Access, Action, Definition, Entry, Error, Kind, NextRun, Receipt, Restart, State, Trigger};

impl ConsoleRuntime {
    // ------------------------=
    // FUNC: automation_command
    // DESC: Claims daemon and scheduled-task commands with the same whitespace grammar as the shared parser.
    // ------------------=
    pub(super) fn automation_command(&mut self, input: &[u8]) -> bool {
        let verb = input.trim_ascii().split(u8::is_ascii_whitespace).next().unwrap_or(b"");
        if !matches!(verb, b"service" | b"schedule") { return false; }
        if let Ok(command) = automation::parse(input) {
            if command.kind == Kind::Daemon && command.action == Action::Inspect && super::service_id(command.name.bytes()).is_some() {
                let access=self.automation_access();
                if !access.can_read { self.automation_error(access.reason.unwrap_or(Error::Denied),Some(Action::Inspect));return true; }
                let mut canonical=[0;80];let prefix=b"service inspect ";canonical[..prefix.len()].copy_from_slice(prefix);
                canonical[prefix.len()..prefix.len()+command.name.bytes().len()].copy_from_slice(command.name.bytes());
                return self.execute_runtime_command(&canonical[..prefix.len()+command.name.bytes().len()]);
            }
        }
        self.task_automation_execute(input);
        true
    }

    // ------------------------=
    // FUNC: automation_access
    // DESC: Exposes the shared current-session authorization decision to command and graphical clients.
    // ------------------=
    pub(super) fn automation_access(&self) -> Access {
        crate::runtime::with_runtime(|runtime| {
            use crate::runtime::identity::{SessionOrigin, SessionState, UserState, SESSION_SETTINGS_UPDATE_ALLOWED};
            let active = runtime.identity.user(self.current_user).is_some_and(|u| u.state == UserState::Active);
            let session = (0..crate::runtime::identity::MAX_SESSIONS).filter_map(|i| runtime.identity.session_nth(i))
                .find(|s| s.id == self.current_session && s.user == self.current_user && s.state == SessionState::Active);
            let administrator = session.is_some_and(|s| s.origin == SessionOrigin::Local
                && s.capabilities & SESSION_SETTINGS_UPDATE_ALLOWED != 0 && runtime.identity.is_administrator(self.current_user));
            Access::evaluate(runtime.automation_ready, active && session.is_some(), administrator)
        }).unwrap_or(Access { available: false, can_read: false, can_manage: false, reason: Some(Error::Unavailable) })
    }

    // ------------------------=
    // FUNC: task_automation_execute
    // DESC: Presents action-specific results while preserving the typed transaction API used by Task Manager.
    // ------------------=
    pub(super) fn task_automation_execute(&mut self, input: &[u8]) -> bool {
        match self.task_automation_execute_receipt(input, None) {
            Ok(receipt) => {
                let message: Option<&[u8]> = match receipt.action {
                    Action::Create => Some(b"created."), Action::Edit => Some(b"configuration saved."),
                    Action::Delete => Some(b"deleted."), Action::Start => Some(b"run queued."),
                    Action::Restart => Some(b"restart requested; waiting for any active run to exit."),
                    Action::Stop => Some(b"stop requested; automatic execution paused. Start/run explicitly to resume."),
                    Action::Kill => Some(b"forced termination requested; automatic execution paused. Start/run explicitly to resume."),
                    Action::Enable => Some(b"automatic execution enabled."),
                    Action::Disable => Some(b"automatic execution disabled; an active run continues."), _ => None,
                };
                if let Some(message) = message {
                    if let Ok(command)=automation::parse(input) { self.output.write_segments(&[if command.kind==Kind::Daemon{b"Service "}else{b"Scheduled task "},command.name.bytes(),b": ",message]); }
                    self.output.write_number(b"Job ID: ", u64::from(receipt.id.unwrap_or(0)));
                }
                true
            }
            Err(error) => { self.automation_error(error, automation::parse(input).ok().map(|c| c.action)); false }
        }
    }

    // ------------------------=
    // FUNC: task_automation_execute_receipt
    // DESC: Returns a stable job receipt only after authorization, optional definition comparison and durable commit succeed.
    // ------------------=
    pub(super) fn task_automation_execute_receipt(&mut self, input: &[u8], expected: Option<Definition>) -> Result<Receipt, Error> {
        let command = automation::parse(input)?;
        let access = self.automation_access();
        if !access.can_read { return Err(access.reason.unwrap_or(Error::Denied)); }
        if command.kind == Kind::Daemon && super::service_id(command.name.bytes()).is_some() { return Err(Error::Protected); }
        if command.action == Action::List {
            let now_ns = crate::runtime::ai::qwen::workers::clock_ns();
            let utc = crate::runtime::ai::clock::SYSTEM.utc_nanoseconds(now_ns).map(|n| n / 1_000_000_000);
            crate::runtime::with_runtime(|runtime| {
                if runtime.automation.count(command.kind) == 0 { self.output.write_line(b"No configured jobs. Use help service or help schedule to create one."); }
                for index in 0..runtime.automation.count(command.kind) {
                    if let Some(entry) = runtime.automation.nth(command.kind, index) {
                        self.output.write_segments(&[entry.name.bytes(), b"  ", if entry.enabled { b"enabled" } else { b"disabled" }, b"  ", state_text(entry.state), b"  ", entry.path.bytes()]);
                        self.automation_trigger(entry.trigger);
                        self.automation_next(entry.next_status(utc, now_ns / 1_000_000_000));
                    }
                }
                if command.kind == Kind::Daemon {
                    self.output.write_line(b"Protected kernel infrastructure (read-only):");
                    for index in 0..runtime.services.count() {
                        if let Some(service) = runtime.services.nth(index) {
                            self.output.write_segments(&[super::service_name(service.manifest.service_id), b"  ", super::service_state_text(service.state)]);
                        }
                    }
                }
            });
            return Ok(Receipt { id: None, action: command.action });
        }
        if matches!(command.action, Action::Inspect | Action::Logs) {
            let entry = crate::runtime::with_runtime(|r| r.automation.find(command.kind, command.name.bytes())).flatten().ok_or(Error::Unknown)?;
            if entry.owner != self.current_user.0 && !access.can_manage { return Err(Error::Denied); }
            if expected.is_some_and(|definition| definition != entry.definition()) { return Err(Error::Conflict); }
            if command.action == Action::Inspect { self.automation_inspect(entry); }
            else {
                self.output.write_segments(&[if entry.kind==Kind::Daemon{b"Service "}else{b"Scheduled task "},entry.name.bytes(),b" - retained output"]);
                self.output.write_number(b"Job ID: ",u64::from(entry.id));
                #[cfg(all(feature="native-python",target_os="none",any(target_arch="aarch64",target_arch="x86_64")))]
                {
                    let mut bytes = [0u8; 8192];
                    let length = crate::runtime::automation_worker::copy_logs(entry.id, &mut bytes);
                    if length == 0 { self.output.write_line(b"No retained output for this job."); }
                    for line in bytes[..length].split(|b| *b == b'\n') { self.output.write_line(line); }
                }
                #[cfg(not(all(feature="native-python",target_os="none",any(target_arch="aarch64",target_arch="x86_64"))))]
                return Err(Error::Unavailable);
            }
            return Ok(Receipt { id: Some(entry.id), action: command.action });
        }
        if !access.can_manage { return Err(access.reason.unwrap_or(Error::Denied)); }
        if let Some(path) = command.path {
            let script_owner = if command.action == Action::Edit {
                crate::runtime::with_runtime(|r| r.automation.find(command.kind, command.name.bytes())).flatten().ok_or(Error::Unknown)?.owner
            } else { self.current_user.0 };
            let readable = crate::storage::object_inspect_path(path.bytes()).ok().is_some_and(|(metadata, _)|
                crate::runtime::native_python_scope::readable(metadata.owner.0, script_owner,
                    metadata.space == crate::storage::object::Space::Personal, path.bytes()));
            if !readable { return Err(Error::ScriptUnavailable); }
        }
        let now_ns = crate::runtime::ai::qwen::workers::clock_ns();
        let clock = crate::runtime::ai::clock::SYSTEM.utc_nanoseconds(now_ns).map(|n| n / 1_000_000_000);
        if clock.is_none() && !matches!(command.action, Action::Stop | Action::Kill | Action::Disable | Action::Delete) { return Err(Error::ClockUnavailable); }
        crate::runtime::with_runtime(|runtime| {
            let mut candidate = runtime.automation;
            let id = candidate.apply_expected(&command, self.current_user.0, access.can_manage, clock.unwrap_or(0), expected)?;
            let mut bytes = [0u8; automation::STATE_BYTES];
            let length = candidate.encode(&mut bytes)?;
            crate::storage::automation_state_commit(&bytes[..length]).map_err(|_| Error::Storage)?;
            runtime.automation = candidate;
            Ok(Receipt { id, action: command.action })
        }).unwrap_or(Err(Error::Unavailable))
    }

    // ------------------------=
    // FUNC: automation_inspect
    // DESC: Shows configured policy and actual execution receipts with readable UTC eligibility.
    // ------------------=
    fn automation_inspect(&mut self, entry: Entry) {
        self.output.write_segments(&[b"Name: ", entry.name.bytes()]);
        self.output.write_number(b"Job ID: ", u64::from(entry.id));
        self.output.write_segments(&[b"Python script: ", entry.path.bytes()]);
        self.output.write_segments(&[b"State: ", state_text(entry.state)]);
        self.output.write_line(if entry.enabled { b"Enabled: yes" } else { b"Enabled: no" });
        self.automation_trigger(entry.trigger);
        self.output.write_segments(&[b"Restart: ", match entry.restart { Restart::Never => b"never", Restart::Always => b"always", Restart::OnFailure => b"on-failure" }]);
        let now = crate::runtime::ai::qwen::workers::clock_ns();
        self.automation_next(entry.next_status(crate::runtime::ai::clock::SYSTEM.utc_nanoseconds(now).map(|n| n / 1_000_000_000), now / 1_000_000_000));
        self.output.write_number(b"Process ID: ", u64::from(entry.process_id));
        self.output.write_number(b"Runs: ", entry.runs);
        if let Some(exit) = entry.last_exit { self.output.write_number(if exit < 0 { b"Last exit: -" } else { b"Last exit: " }, exit.unsigned_abs()); }
        else { self.output.write_line(b"Last exit: none"); }
    }

    // ------------------------=
    // FUNC: automation_trigger
    // DESC: Describes the configured automatic trigger independently of current eligibility.
    // ------------------=
    fn automation_trigger(&mut self, trigger: Trigger) {
        match trigger {
            Trigger::Manual => self.output.write_line(b"Trigger: manual"),
            Trigger::Boot { delay_seconds } => self.output.write_number(b"Trigger: seconds after boot: ", u64::from(delay_seconds)),
            Trigger::Interval { seconds } => self.output.write_number(b"Trigger: repeat every seconds: ", u64::from(seconds)),
            Trigger::Once { unix_seconds } => self.automation_utc(b"Trigger: once at ", unix_seconds),
        }
    }

    // ------------------------=
    // FUNC: automation_utc
    // DESC: Displays a UTC calendar value with an explicit epoch fallback for legacy out-of-range checkpoints.
    // ------------------=
    fn automation_utc(&mut self, label: &[u8], seconds: u64) {
        if let Some(date) = automation::format_utc(seconds) { self.output.write_segments(&[label, date.bytes()]); }
        else { self.output.write_number(b"UTC epoch seconds: ", seconds); }
    }

    // ------------------------=
    // FUNC: automation_next
    // DESC: Explains why a job will run or remain idle using the same typed eligibility as Task Manager.
    // ------------------=
    fn automation_next(&mut self, next: NextRun) {
        match next {
            NextRun::Scheduled { unix_seconds } => self.automation_utc(b"Next: ", unix_seconds),
            NextRun::BootDelay { seconds } => self.output.write_number(b"Next: boot delay seconds remaining: ", seconds),
            other => self.output.write_line(match other {
                NextRun::Disabled => b"Next: disabled; run explicitly to execute once.", NextRun::Manual => b"Next: manual run only.",
                NextRun::Paused => b"Next: paused by stop; start or enable to resume.", NextRun::NextBoot => b"Next: next boot.",
                NextRun::Ready => b"Next: ready; waiting for a worker.", NextRun::Running => b"Next: current run must finish; runs never overlap.",
                NextRun::Stopping => b"Next: waiting for process termination.", NextRun::Finished => b"Next: one-time trigger complete; edit the date or run manually.",
                NextRun::ClockUnavailable => b"Next: waiting for a valid system UTC clock.", _ => b"Next: unavailable.",
            }),
        }
    }

    // ------------------------=
    // FUNC: automation_error
    // DESC: Reports a specific failure without acknowledging an uncommitted mutation.
    // ------------------=
    pub(super) fn automation_error(&mut self, error: Error, action: Option<Action>) {
        self.output.write_line(match error {
            Error::Invalid => b"Invalid command. Use help service or help schedule for syntax and examples.",
            Error::Unknown => b"Job not found. Use service list or schedule list.", Error::Duplicate => b"This job name is already in use. Choose another name or edit the existing job.",
            Error::Full => b"All 16 job slots are in use. Delete an unused job first.", Error::SignInRequired => b"Sign in to inspect or manage jobs.",
            Error::Denied if matches!(action,Some(Action::Inspect|Action::Logs)) => b"Only this job's approving account or a local administrator can inspect its details or output.",
            Error::Denied => b"An active local administrator session is required to change jobs.",
            Error::Protected => b"Kernel infrastructure is protected. Choose a distinct name for an executable daemon.",
            Error::ScriptUnavailable => b"Script unavailable. Use an existing absolute .py path readable by the job's approving account.",
            Error::ClockUnavailable => b"A valid system UTC clock is required. Check Date & Time settings before scheduling or starting jobs.",
            Error::Conflict => b"This job changed or was replaced. Refresh it and review the current definition before saving.",
            Error::Busy if action == Some(Action::Start) => b"This job is already queued or running. Use inspect for its state or restart to request a new run.",
            Error::Busy => b"Stop the job and wait for termination before editing or deleting it.",
            Error::Storage | Error::Corrupt => b"Job checkpoint unavailable or corrupt; no change committed.", Error::Unavailable => b"Native job authority is unavailable.",
        });
    }
}

// ------------------------=
// FUNC: state_text
// DESC: Formats the authoritative lifecycle state for human display only.
// ------------------=
fn state_text(state: State) -> &'static [u8] {
    match state { State::Stopped => b"stopped", State::Queued => b"queued", State::Running => b"running",
        State::Stopping => b"stopping", State::Killing => b"killing", State::Completed => b"completed", State::Failed => b"failed" }
}
