//! Authenticated command and Task Manager adapter for the same durable job catalog.
use super::ConsoleRuntime;
use crate::runtime::automation::{self, Action, Entry, Error, Kind, Restart, State, Trigger};

impl ConsoleRuntime {
    // ------------------------=
    // FUNC: automation_command
    // DESC: Claims explicit daemon and scheduled-task commands before generic operation-language parsing.
    // ------------------=
    pub(super) fn automation_command(&mut self, input: &[u8]) -> bool {
        let input = input.trim_ascii();
        if input != b"service" && input != b"schedule" && !input.starts_with(b"service ") && !input.starts_with(b"schedule ") {
            return false;
        }
        if let Some(name) = input.strip_prefix(b"service inspect ") {
            if super::service_id(name).is_some() { return false; }
        }
        self.task_automation_execute(input);
        true
    }

    // ------------------------=
    // FUNC: task_automation_execute
    // DESC: Validates an active session and commits a staged job mutation before either client can observe success.
    // ------------------=
    pub(super) fn task_automation_execute(&mut self, input: &[u8]) -> bool {
        let command = match automation::parse(input) {
            Ok(command) => command,
            Err(error) => { self.automation_error(error); return false; }
        };
        let authorization = crate::runtime::with_runtime(|runtime| {
            use crate::runtime::identity::{SessionOrigin, SessionState, SESSION_SETTINGS_UPDATE_ALLOWED};
            if !runtime.identity.user(self.current_user).is_some_and(|u| u.state == crate::runtime::identity::UserState::Active) { return None; }
            let session = (0..crate::runtime::identity::MAX_SESSIONS)
                .filter_map(|i| runtime.identity.session_nth(i))
                .find(|s| s.id == self.current_session && s.user == self.current_user && s.state == SessionState::Active)?;
            Some((runtime.automation_ready, session.origin == SessionOrigin::Local
                && session.capabilities & SESSION_SETTINGS_UPDATE_ALLOWED != 0
                && runtime.identity.is_administrator(self.current_user)))
        }).flatten();
        let Some((ready, administrator)) = authorization else { self.automation_error(Error::Denied); return false; };
        if !ready { self.automation_error(Error::Storage); return false; }
        if command.kind == Kind::Daemon && super::service_id(command.name.bytes()).is_some() {
            self.output.write_line(b"Kernel infrastructure is protected; executable daemons use distinct service names.");
            return false;
        }
        if command.action == Action::List {
            crate::runtime::with_runtime(|runtime| {
                self.output.write_line(b"NAME   ENABLED   STATE   PYTHON SCRIPT");
                for index in 0..runtime.automation.count(command.kind) {
                    if let Some(entry) = runtime.automation.nth(command.kind, index) {
                        self.output.write_segments(&[entry.name.bytes(), b"  ", if entry.enabled { b"yes" } else { b"no" },
                            b"  ", state_text(entry.state), b"  ", entry.path.bytes()]);
                    }
                }
                if command.kind == Kind::Daemon {
                    self.output.write_line(b"Protected kernel infrastructure:");
                    for index in 0..runtime.services.count() {
                        if let Some(service) = runtime.services.nth(index) {
                            self.output.write_segments(&[super::service_name(service.manifest.service_id), b"  ",
                                super::service_state_text(service.state)]);
                        }
                    }
                }
            });
            return true;
        }
        if matches!(command.action, Action::Inspect | Action::Logs) {
            let entry = crate::runtime::with_runtime(|r| r.automation.find(command.kind, command.name.bytes())).flatten();
            let Some(entry) = entry else { self.automation_error(Error::Unknown); return false; };
            if entry.owner != self.current_user.0 && !administrator { self.automation_error(Error::Denied); return false; }
            if command.action == Action::Inspect { self.automation_inspect(entry); }
            else {
                #[cfg(all(feature="native-python",target_os="none",any(target_arch="aarch64",target_arch="x86_64")))]
                {
                    let mut bytes = [0u8; 8192];
                    let length = crate::runtime::automation_worker::copy_logs(entry.id, &mut bytes);
                    if length == 0 { self.output.write_line(b"No retained output for this job."); }
                    for line in bytes[..length].split(|b| *b == b'\n') { self.output.write_line(line); }
                }
                #[cfg(not(all(feature="native-python",target_os="none",any(target_arch="aarch64",target_arch="x86_64"))))]
                self.output.write_line(b"Native Python worker is unavailable in this build.");
            }
            return true;
        }
        if !administrator { self.automation_error(Error::Denied); return false; }
        if let Some(path) = command.path {
            let script_owner = if command.action == Action::Edit {
                crate::runtime::with_runtime(|r| r.automation.find(command.kind, command.name.bytes()))
                    .flatten().map_or(self.current_user.0, |entry| entry.owner)
            } else { self.current_user.0 };
            let readable = crate::storage::object_inspect_path(path.bytes()).ok().is_some_and(|(metadata, _)|
                crate::runtime::native_python_scope::readable(metadata.owner.0, script_owner,
                    metadata.space == crate::storage::object::Space::Personal, path.bytes()));
            if !readable { self.output.write_line(b"Script must exist at an absolute .py path readable by the job's approving account."); return false; }
        }
        let now_ns = crate::runtime::ai::qwen::workers::clock_ns();
        let clock = crate::runtime::ai::clock::SYSTEM.utc_nanoseconds(now_ns).map(|n| n / 1_000_000_000);
        if clock.is_none() && !matches!(command.action, Action::Stop | Action::Kill | Action::Disable | Action::Delete) {
            self.output.write_line(b"A valid system clock is required to configure or dispatch jobs."); return false;
        }
        let now = clock.unwrap_or(0);
        let outcome = crate::runtime::with_runtime(|runtime| {
            let mut candidate = runtime.automation;
            let id = candidate.apply(&command, self.current_user.0, administrator, now)?;
            let mut bytes = [0u8; automation::STATE_BYTES];
            let length = candidate.encode(&mut bytes)?;
            crate::storage::automation_state_commit(&bytes[..length]).map_err(|_| Error::Storage)?;
            runtime.automation = candidate;
            Ok::<_, Error>(id)
        }).unwrap_or(Err(Error::Unavailable));
        match outcome {
            Ok(_) => {
                self.output.write_line(if matches!(command.action, Action::Start | Action::Restart | Action::Stop | Action::Kill) {
                    b"Lifecycle request committed; inspect the job for native process completion."
                } else { b"Job configuration saved." });
                true
            }
            Err(error) => { self.automation_error(error); false }
        }
    }

    // ------------------------=
    // FUNC: automation_inspect
    // DESC: Presents the configured trigger, actual run state, and observed exit receipt without inventing process success.
    // ------------------=
    fn automation_inspect(&mut self, entry: Entry) {
        self.output.write_segments(&[b"Name: ", entry.name.bytes()]);
        self.output.write_segments(&[b"Python script: ", entry.path.bytes()]);
        self.output.write_segments(&[b"State: ", state_text(entry.state)]);
        self.output.write_line(if entry.enabled { b"Enabled: yes" } else { b"Enabled: no" });
        match entry.trigger {
            Trigger::Manual => self.output.write_line(b"Start: manual"),
            Trigger::Boot { delay_seconds } => self.output.write_number(b"Boot delay seconds: ", u64::from(delay_seconds)),
            Trigger::Interval { seconds } => self.output.write_number(b"Interval seconds: ", u64::from(seconds)),
            Trigger::Once { unix_seconds } => self.output.write_number(b"Once at UTC epoch: ", unix_seconds),
        }
        self.output.write_segments(&[b"Restart: ", match entry.restart { Restart::Never => b"never", Restart::Always => b"always", Restart::OnFailure => b"on-failure" }]);
        self.output.write_number(b"Process id: ", u64::from(entry.process_id));
        self.output.write_number(b"Runs: ", entry.runs);
        self.output.write_number(b"Next UTC epoch: ", entry.next_run);
        if let Some(exit) = entry.last_exit {
            self.output.write_number(if exit < 0 { b"Exit: -" } else { b"Exit: " }, exit.unsigned_abs());
        }
    }

    // ------------------------=
    // FUNC: automation_error
    // DESC: Reports failed admission without changing the registry or acknowledging an uncommitted mutation.
    // ------------------=
    fn automation_error(&mut self, error: Error) {
        self.output.write_line(match error {
            Error::Invalid => b"Invalid command. Use help service or help schedule for syntax and examples.",
            Error::Unknown => b"Unknown job. Use service list or schedule list.",
            Error::Duplicate => b"A job with this name already exists.",
            Error::Full => b"The bounded job catalog is full.",
            Error::Denied => b"An active local administrator session is required to change jobs.",
            Error::Busy => b"Stop the job and wait for completion before editing or deleting it.",
            Error::Storage | Error::Corrupt => b"Job checkpoint unavailable or corrupt; no change committed.",
            Error::Unavailable => b"Native job authority is unavailable.",
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
