//! Native account command execution and isolated masked credential input.

#[path = "console_accounts_password.rs"]
pub(super) mod account_password;
#[path = "console_accounts_command.rs"]
mod account_command;
pub(super) use account_command::sensitive_command;
pub(super) use account_password::{PasswordAuthority, PasswordEntry};

use super::{ConsoleKey, ConsoleRuntime};
use account_command::AccountCommand;
use account_password::{PasswordProgress, PasswordShortcut};
use crate::drivers::input::desktop_shortcuts;
use crate::runtime::identity::{AccountResult, GroupIdentity, IdentityError, UserIdentity, UserState};

impl ConsoleRuntime {
    // ------------------------=
    // FUNC: execute_accounts_node
    // DESC: Prepares safe typed account output or dispatches mutations through the durable shared backend.
    // ------------------=
    pub(super) fn execute_accounts_node(&mut self, node: &crate::runtime::console_language::OperationNode<'_>) -> bool {
        let prepared = crate::runtime::with_runtime(|runtime| {
            account_command::prepare(&runtime.identity.accounts(self.current_session)?, node)
        }).unwrap_or(Err(IdentityError::InvalidState));
        match prepared {
            Ok(AccountCommand::Users { users, count }) => {
                self.output.write_number(b"Users: ", count as u64);
                for user in users.iter().flatten() { self.render_account_user(*user); }
            }
            Ok(AccountCommand::Groups { groups, count }) => {
                self.output.write_number(b"Groups: ", count as u64);
                for group in groups.iter().flatten() { self.render_account_group(*group); }
            }
            Ok(AccountCommand::User { user, groups, count }) => {
                self.render_account_user(user);
                self.output.write_number(b"Memberships: ", count as u64);
                for group in groups.iter().flatten() { self.render_account_group(*group); }
            }
            Ok(AccountCommand::Group { group, users, count }) => {
                self.render_account_group(group);
                self.output.write_number(b"Members: ", count as u64);
                for user in users.iter().flatten() { self.render_account_user(*user); }
            }
            Ok(AccountCommand::Password(action)) => {
                self.account_password.begin(self.current_session, action);
                self.reset_input();
                self.output.write_line(b"New password (8-72 characters). Enter continues; Escape cancels.");
            }
            Ok(AccountCommand::Mutation(mutation)) => {
                let session = self.current_session;
                let now = crate::runtime::ai::qwen::workers::clock_ns() / 1_000_000_000;
                match mutation.execute(now, |request| crate::runtime::manage_accounts(session, request)) {
                    Ok(result) => self.render_account_result(result),
                    Err(error) => { self.identity_error(error); }
                }
            }
            Err(error) => { self.identity_error(error); }
        }
        true
    }

    // ------------------------=
    // FUNC: account_password_input
    // DESC: Consumes private input before line editing, history, clipboard, completion, or command echo can observe it.
    // ------------------=
    pub(super) fn account_password_input(&mut self, key: ConsoleKey) -> bool {
        if !self.account_password.active() { return false; }
        let session = self.current_session;
        let authorized = self.account_password.session() == session && crate::runtime::with_runtime(|runtime| runtime.identity.accounts(session).is_ok()).unwrap_or(false);
        if !authorized {
            self.cancel_account_password();
            self.identity_error(IdentityError::AccessDenied);
            return true;
        }
        if let ConsoleKey::Shortcut(code) = key {
            let desktop = matches!(self.mode, super::ConsoleMode::Desktop);
            match self.account_password.shortcut(code, desktop) {
                PasswordShortcut::Lock => {
                    self.reset_input();
                    self.lock_session_preserving_desktop(false);
                    return true;
                }
                PasswordShortcut::Dispatch => { self.reset_input(); return false; }
                PasswordShortcut::Consume => return true,
            }
        }
        if matches!(key, ConsoleKey::Tab(true)) && self.mode == super::ConsoleMode::Desktop {
            self.cancel_account_password();
            return false;
        }
        match key {
            ConsoleKey::Escape => { self.cancel_account_password(); self.output.write_line(b"Password entry cancelled."); }
            ConsoleKey::Character(byte) => {
                if !self.account_password.push(byte) { self.output.write_line(b"Password input limit: 72 printable characters."); }
            }
            ConsoleKey::Backspace | ConsoleKey::Delete => self.account_password.backspace(),
            ConsoleKey::Enter => match self.account_password.submit() {
                PasswordProgress::TooShort => self.output.write_line(b"Use at least 8 characters."),
                PasswordProgress::Confirming => self.output.write_line(b"Confirm password:"),
                PasswordProgress::Mismatch => self.output.write_line(b"Passwords did not match. Enter a new password:"),
                PasswordProgress::Ready => {
                    let now = crate::runtime::ai::qwen::workers::clock_ns() / 1_000_000_000;
                    let result = self.account_password.finish(session, now, |request| crate::runtime::manage_accounts(session, request));
                    self.reset_input();
                    match result {
                        Ok(result) => self.render_account_result(result),
                        Err(error) => { self.identity_error(error); }
                    }
                }
                _ => {}
            },
            _ => {}
        }
        let length = self.account_password.masked_length();
        self.command.fill(0);
        self.command[..length].fill(b'*');
        self.command_length = length;
        self.command_cursor = length;
        true
    }

    // ------------------------=
    // FUNC: cancel_account_password
    // DESC: Erases pending secrets and their mask at every cancellation or session boundary.
    // ------------------=
    pub(super) fn cancel_account_password(&mut self) {
        if self.account_password.active() { self.account_password.cancel(); self.reset_input(); }
    }

    // ------------------------=
    // FUNC: render_account_user
    // DESC: Projects a user handle, display name, stable reference, and enabled state without credentials.
    // ------------------=
    fn render_account_user(&mut self, user: UserIdentity) {
        self.output.write_number(b"user:", user.id.short());
        self.output.write_segments(&[b"  ", user.handle.as_bytes(), b" / ", user.display_name.as_bytes()]);
        self.output.write_line(if user.state == UserState::Active { b"  enabled" } else { b"  disabled" });
    }

    // ------------------------=
    // FUNC: render_account_group
    // DESC: Projects group identity and built-in status through the existing console output renderer.
    // ------------------=
    fn render_account_group(&mut self, group: GroupIdentity) {
        self.output.write_number(b"group:", group.id.short());
        self.output.write_segments(&[b"  ", group.name.as_bytes(), b" / ", group.display_name.as_bytes()]);
        if group.builtin { self.output.write_line(b"  built-in administrator authority"); }
    }

    // ------------------------=
    // FUNC: render_account_result
    // DESC: Reports only results returned after durable success, including preserved storage on deletion.
    // ------------------=
    fn render_account_result(&mut self, result: AccountResult) {
        match result {
            AccountResult::User(user) => self.render_account_user(user),
            AccountResult::Group(group) => self.render_account_group(group),
            AccountResult::Membership { group, user, present } => {
                self.output.write_number(b"group:", group.short());
                self.output.write_number(b"user:", user.short());
                self.output.write_line(if present { b"Membership added." } else { b"Membership removed." });
            }
            AccountResult::DeletedUser { user, preserved_personal_space } => {
                self.output.write_number(b"Deleted user:", user.short());
                self.output.write_number(b"Retained personal space:", preserved_personal_space.short());
            }
            AccountResult::DeletedGroup { group } => self.output.write_number(b"Deleted group:", group.short()),
            AccountResult::PasswordReset { user } => {
                self.output.write_number(b"Password updated for user:", user.short());
                self.output.write_line(b"Previous user sessions have been revoked.");
                if user == self.current_user && !crate::runtime::with_runtime(|runtime| runtime.identity.accounts(self.current_session).is_ok()).unwrap_or(false) {
                    self.accounts_return_to_login();
                }
            }
        }
    }
}
