//! Native Settings controller. Only authenticated transactional dispatch mutates accounts.
use super::console_accounts_cli::PasswordAuthority;
use super::{ConsoleKey, ConsoleMode, ConsoleRuntime, DesktopAppKind, SystemLayout};
use crate::runtime::identity::{AccountRequest, AccountResult, IdentityError, StableId, UserState};
use crate::ui::accounts::{
    self, Directory, Edit, Entry, Feedback, Mode, State, Tab, Target, Text, View,
};

struct SecretField {
    bytes: [u8; 72],
    len: usize,
    cursor: usize,
}
impl SecretField {
    // ------------------------=
    // FUNC: new
    // DESC: Creates a private password field with the backend's exact capacity.
    // ------------------=
    const fn new() -> Self {
        Self {
            bytes: [0; 72],
            len: 0,
            cursor: 0,
        }
    }
    // ------------------------=
    // FUNC: as_bytes
    // DESC: Borrows secret bytes solely for comparison or authenticated dispatch.
    // ------------------=
    fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }
    // ------------------------=
    // FUNC: edit
    // DESC: Applies shared bounded editing without retaining an implicit copy of a password.
    // ------------------=
    fn edit(&mut self, event: Edit) -> bool {
        use crate::ui::text_input as input;
        match event {
            Edit::Insert(byte) => {
                input::insert_ascii(&mut self.bytes, &mut self.len, &mut self.cursor, byte)
            }
            Edit::Backspace => input::backspace(&mut self.bytes, &mut self.len, &mut self.cursor),
            Edit::Delete => input::delete(&mut self.bytes, &mut self.len, &mut self.cursor),
            Edit::Left => input::move_caret(&mut self.cursor, self.len, -1),
            Edit::Right => input::move_caret(&mut self.cursor, self.len, 1),
            Edit::Home => input::move_caret(&mut self.cursor, self.len, -2),
            Edit::End => input::move_caret(&mut self.cursor, self.len, 2),
        }
    }
}
pub(super) struct Secrets {
    fields: [SecretField; 2],
    session: StableId,
}
impl Secrets {
    // ------------------------=
    // FUNC: new
    // DESC: Allocates private password inputs outside the published UI snapshot.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            fields: [SecretField::new(), SecretField::new()],
            session: StableId::zero(),
        }
    }
    // ------------------------=
    // FUNC: clear
    // DESC: Erases password storage with volatile writes before dropping entry metadata.
    // ------------------=
    pub fn clear(&mut self) {
        self.session = StableId::zero();
        for field in &mut self.fields {
            for byte in &mut field.bytes {
                unsafe {
                    core::ptr::write_volatile(byte, 0);
                }
            }
            field.len = 0;
            field.cursor = 0;
        }
        core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
    }
}
// ------------------------=
// FUNC: edit
// DESC: Maps native editing keys into bounded UI editing events without clipboard or history.
// ------------------=
fn edit(key: ConsoleKey) -> Option<Edit> {
    Some(match key {
        ConsoleKey::Character(b) => Edit::Insert(b),
        ConsoleKey::Backspace => Edit::Backspace,
        ConsoleKey::Delete => Edit::Delete,
        ConsoleKey::Left => Edit::Left,
        ConsoleKey::Right => Edit::Right,
        ConsoleKey::Home => Edit::Home,
        ConsoleKey::End => Edit::End,
        _ => return None,
    })
}
// ------------------------=
// FUNC: failure
// DESC: Preserves distinct service errors without reporting an unsuccessful mutation as saved.
// ------------------=
fn failure(error: IdentityError) -> Feedback {
    match error {
        IdentityError::AccessDenied => Feedback::Denied,
        IdentityError::Conflict => Feedback::Conflict,
        IdentityError::Full => Feedback::Full,
        IdentityError::InvalidInput => Feedback::Invalid,
        IdentityError::InvalidCredential => Feedback::Credential,
        IdentityError::RateLimited => Feedback::RateLimited,
        IdentityError::NotFound => Feedback::Unavailable,
        IdentityError::CommitUncertain => Feedback::RestartRequired,
        _ => Feedback::Failed,
    }
}
impl ConsoleRuntime {
    // ------------------------=
    // FUNC: accounts_directory
    // DESC: Projects authenticated read leases into bounded non-secret presentation records.
    // ------------------=
    fn accounts_directory(&self) -> Directory {
        crate::runtime::with_runtime(|runtime| {
            let Ok(reader) = runtime.identity.accounts(self.current_session) else {
                return Directory::empty();
            };
            let mut data = Directory::empty();
            data.available = true;
            data.actor = self.current_user.0;
            data.administrator = reader.is_administrator(self.current_user);
            for i in 0..reader.user_count().min(accounts::CAPACITY) {
                if let Some(user) = reader.user_nth(i) {
                    data.users[data.users_len] = Entry {
                        id: user.id.0,
                        name: Text::from_bytes(user.display_name.as_bytes()),
                        handle: Text::from_bytes(user.handle.as_bytes()),
                        enabled: user.state == UserState::Active,
                        admin: reader.is_administrator(user.id),
                        builtin: false,
                        members: 0,
                    };
                    data.users_len += 1;
                }
            }
            for i in 0..reader.group_count().min(accounts::CAPACITY) {
                if let Some(group) = reader.group_nth(i) {
                    let mut members = 0;
                    for u in 0..data.users_len {
                        if reader.is_group_member(group.id, StableId(data.users[u].id)) {
                            members |= 1 << u;
                        }
                    }
                    data.groups[data.groups_len] = Entry {
                        id: group.id.0,
                        name: Text::from_bytes(group.display_name.as_bytes()),
                        handle: Text::from_bytes(group.name.as_bytes()),
                        enabled: true,
                        admin: false,
                        builtin: group.builtin,
                        members,
                    };
                    data.groups_len += 1;
                }
            }
            data
        })
        .unwrap_or(Directory::empty())
    }
    // ------------------------=
    // FUNC: accounts_publish
    // DESC: Refreshes the committed directory and publishes edits without exposing password storage.
    // ------------------=
    pub(super) fn accounts_publish(&self) {
        if self.mode == ConsoleMode::Settings && self.system_focus == 2 {
            let data = self.accounts_directory();
            let mut state = self.accounts;
            state.password_lengths = [
                self.accounts_secrets.fields[0].len,
                self.accounts_secrets.fields[1].len,
            ];
            state.password_cursors = [
                self.accounts_secrets.fields[0].cursor,
                self.accounts_secrets.fields[1].cursor,
            ];
            accounts::publish(View { state, data });
        }
    }
    // ------------------------=
    // FUNC: accounts_layout
    // DESC: Resolves the same physical viewport and scale used by the production painter.
    // ------------------=
    fn accounts_layout(&self, data: &Directory) -> accounts::Layout {
        let layout = SystemLayout::new(
            self.system.framebuffer_width,
            self.system.framebuffer_height,
        );
        accounts::layout(
            layout
                .settings_window_geometry_for_section(self.settings_window, 2)
                .viewport,
            &self.accounts,
            data,
            layout.scale(),
        )
    }
    // ------------------------=
    // FUNC: accounts_changed
    // DESC: Invalidates retained Settings pixels after real account or editing transitions.
    // ------------------=
    fn accounts_changed(&self) {
        self.accounts_publish();
        crate::bootstrap::system_ui_settings_changed();
    }
    // ------------------------=
    // FUNC: accounts_cancel
    // DESC: Drops local drafts and securely erases secret inputs when leaving their context.
    // ------------------=
    pub(super) fn accounts_cancel(&mut self) {
        let restart = self.accounts.feedback == Feedback::RestartRequired;
        self.accounts_secrets.clear();
        self.accounts.cancel();
        if restart {
            self.accounts.feedback = Feedback::RestartRequired;
        }
        if self.system_focus == 2 {
            self.settings_editing = false;
        }
        accounts::publish(View {
            state: if restart { self.accounts } else { State::new() },
            data: Directory::empty(),
        });
    }
    // ------------------------=
    // FUNC: accounts_return_to_login
    // DESC: Clears revoked session context without attempting to lock an already invalid session.
    // ------------------=
    pub(super) fn accounts_return_to_login(&mut self) {
        self.accounts_cancel();
        self.cancel_account_password();
        crate::runtime::ai::with_ai_runtime(|ai| {
            ai.cancel_app_turn();
            ai.cancel_chat();
        });
        self.current_session = StableId::zero();
        self.current_user = StableId::zero();
        self.end_surface_task(crate::runtime::task_manager::IMAGE_SETTINGS);self.settings_open = false;
        self.settings_editing = false;
        self.desktop_app = DesktopAppKind::None;
        self.mode = ConsoleMode::Authentication;
        self.system_step = 0;
        self.system_focus = 1;
        self.reset_input();
        self.synchronize_workplace();
        crate::bootstrap::system_ui_settings_changed();
    }
    // ------------------------=
    // FUNC: accounts_focus_guard
    // DESC: Wipes revoked or unfocused secrets and consumes the event that discovers an authority change.
    // ------------------=
    pub(super) fn accounts_focus_guard(&mut self) -> bool {
        let was_editing = self.accounts.mode != Mode::Browse || self.account_password.active();
        let ui_editing = self.accounts.mode != Mode::Browse
            && self.accounts.feedback != Feedback::RestartRequired;
        if ui_editing || self.account_password.active() {
            let authority = crate::runtime::with_runtime(|r| {
                r.identity.accounts(self.current_session).map(|_| ())
            })
            .unwrap_or(Err(IdentityError::InvalidState));
            let private_authority = self
                .account_password
                .revalidate(self.current_session, authority);
            let changed = ui_editing && self.accounts_secrets.session != self.current_session;
            if changed
                || authority.is_err()
                || matches!(
                    private_authority,
                    PasswordAuthority::SignIn | PasswordAuthority::RestartRequired
                )
            {
                self.accounts_cancel();
                self.cancel_account_password();
                // Revalidation already erased the private entry; clear its command mask too.
                self.reset_input();
                match authority {
                    Err(IdentityError::CommitUncertain) => {
                        self.mode = ConsoleMode::Settings;
                        self.system_focus = 2;
                        self.settings_open = true;
                        self.settings_editing = false;
                        self.accounts.feedback = Feedback::RestartRequired;
                        self.ai_chat_focus = 0;
                    }
                    _ => self.accounts_return_to_login(),
                }
                self.accounts_changed();
                self.redraw();
                return true;
            }
        }
        let owner = if self.mode == ConsoleMode::Settings {
            4
        } else {
            1
        };
        let assistant = crate::ui::app_assistant::read(owner);
        let obscured = self.ai_chat_focus != 0
            || (assistant.expanded && assistant.focused)
            || self.spatial.open
            || super::clipboard_menu::current().open
            || self.editor_dialog != super::EditorDialog::None;
        if (obscured || !(self.mode == ConsoleMode::Settings && self.system_focus == 2))
            && self.accounts.mode != Mode::Browse
        {
            self.accounts_cancel();
        }
        if obscured
            || !(self.mode == ConsoleMode::Console
                || (self.mode == ConsoleMode::Desktop
                    && self.desktop_app == DesktopAppKind::CommandWindow))
        {
            self.cancel_account_password();
        }
        if was_editing && self.accounts.mode == Mode::Browse && !self.account_password.active() {
            self.accounts_changed();
            self.redraw();
        }
        false
    }
    // ------------------------=
    // FUNC: accounts_key
    // DESC: Routes keyboard navigation, private form entry and activation through shared targets.
    // ------------------=
    pub(super) fn accounts_key(&mut self, key: ConsoleKey) -> bool {
        if self.mode != ConsoleMode::Settings || self.system_focus != 2 {
            return false;
        }
        if self.accounts_focus_guard() {
            return true;
        }
        if self.accounts.feedback == Feedback::RestartRequired {
            if matches!(key, ConsoleKey::Escape) {
                if self.accounts.mode == Mode::Browse {
                    return false;
                }
                self.accounts_cancel();
                self.accounts_changed();
            }
            return true;
        }
        if self.ai_chat_focus != 0
            || self.spatial.open
            || super::clipboard_menu::current().open
            || crate::ui::app_assistant::read(4).focused
        {
            return false;
        }
        let data = self.accounts_directory();
        let geometry = self.accounts_layout(&data);
        match key {
            ConsoleKey::Escape => {
                if self.accounts.mode != Mode::Browse {
                    self.accounts_cancel();
                } else if self.accounts.detail {
                    self.accounts.detail = false;
                    self.accounts.focus = Target::Users;
                } else {
                    return false;
                }
            }
            ConsoleKey::Tab(backwards) => geometry.tab(&mut self.accounts, backwards),
            ConsoleKey::Enter => {
                if matches!(self.accounts.focus, Target::Field(_)) {
                    if self.accounts.focus == Target::Field(3)
                        || self.accounts.focus == Target::Field(1)
                            && !(self.accounts.mode == Mode::Create
                                && self.accounts.tab == Tab::Users)
                    {
                        self.accounts_activate(Target::Save, &data);
                    } else {
                        geometry.tab(&mut self.accounts, false);
                    }
                } else {
                    self.accounts_activate(self.accounts.focus, &data);
                }
            }
            ConsoleKey::Up | ConsoleKey::Down
                if self.accounts.mode == Mode::Browse
                    && matches!(self.accounts.focus, Target::Row(_)) =>
            {
                let count = data.entries(self.accounts.tab).len();
                if count > 0 {
                    let i = self.accounts.selected_index(&data).unwrap_or(0);
                    let next = if matches!(key, ConsoleKey::Up) {
                        i.saturating_sub(1)
                    } else {
                        (i + 1).min(count - 1)
                    };
                    self.accounts.select(&data, next);
                    let layout = self.accounts_layout(&data);
                    if let Some(c) = layout.controls[..layout.count]
                        .iter()
                        .find(|c| c.target == Target::Row(next))
                    {
                        layout.reveal(&mut self.accounts, *c);
                    }
                }
            }
            ConsoleKey::Shortcut(code)
                if matches!(self.accounts.focus, Target::Field(0 | 1))
                    && self.accounts.mode != Mode::Browse =>
            {
                self.accounts_text_shortcut(code);
            }
            _ => {
                if let Target::Field(index) = self.accounts.focus {
                    if let Some(event) = edit(key) {
                        if self.accounts.mode != Mode::Browse {
                            let changed = match index {
                                0 => self.accounts.name.edit(event),
                                1 => self.accounts.handle.edit(event),
                                2 | 3 => self.accounts_secrets.fields[index - 2].edit(event),
                                _ => false,
                            };
                            self.accounts.feedback = if !changed && matches!(event, Edit::Insert(_))
                            {
                                Feedback::TooLong
                            } else {
                                Feedback::None
                            };
                        }
                    }
                }
            }
        }
        self.settings_editing = self.accounts.mode != Mode::Browse;
        self.accounts_changed();
        true
    }
    // ------------------------=
    // FUNC: accounts_text_shortcut
    // DESC: Applies session-owned clipboard and select-all operations only to ordinary identity labels.
    // ------------------=
    fn accounts_text_shortcut(&mut self, code: u8) {
        let index = if self.accounts.focus == Target::Field(0) {
            0
        } else {
            1
        };
        let mut text = if index == 0 {
            self.accounts.name
        } else {
            self.accounts.handle
        };
        match code.to_ascii_lowercase() {
            b'a' => text.selected = true,
            b'c' | b'x' if text.selected => {
                if self.copy_workplace_text(text.as_bytes()) && code.to_ascii_lowercase() == b'x' {
                    text = Text::new();
                }
            }
            b'v' => {
                let mut value = [0; crate::ui::clipboard::MAX_CLIPBOARD_BYTES];
                match self.read_workplace_text(&mut value) {
                    Ok(length) => {
                        let mut next = if text.selected { Text::new() } else { text };
                        if crate::ui::text_input::paste_ascii(
                            &mut next.bytes,
                            &mut next.len,
                            &mut next.cursor,
                            &value[..length],
                        ) {
                            next.selected = false;
                            text = next;
                            self.accounts.feedback = Feedback::None;
                        } else {
                            self.accounts.feedback =
                                if value[..length].iter().any(|b| !(32..=126).contains(b)) {
                                    Feedback::Invalid
                                } else {
                                    Feedback::TooLong
                                };
                        }
                    }
                    Err(_) => self.accounts.feedback = Feedback::Failed,
                }
            }
            _ => {}
        }
        if index == 0 {
            self.accounts.name = text;
        } else {
            self.accounts.handle = text;
        }
    }
    // ------------------------=
    // FUNC: accounts_pointer
    // DESC: Shares clipped physical hit testing with paint and consumes only the section body.
    // ------------------=
    pub(super) fn accounts_pointer(&mut self, layout: SystemLayout, clicked: bool) -> bool {
        if self.system_focus != 2
            || self.settings_window_dragging
            || self.settings_window_resizing.is_some()
        {
            return false;
        }
        let point = layout.point(self.pointer_x, self.pointer_y);
        let viewport = layout
            .settings_window_geometry_for_section(self.settings_window, 2)
            .viewport;
        if !viewport.contains(point) {
            return false;
        }
        if clicked {
            let data = self.accounts_directory();
            let geometry = self.accounts_layout(&data);
            if let Some(target) = geometry.hit(point) {
                self.accounts.focus = target;
                self.accounts_activate(target, &data);
                self.settings_editing = self.accounts.mode != Mode::Browse;
                self.accounts_changed();
            }
        }
        true
    }
    // ------------------------=
    // FUNC: accounts_scroll
    // DESC: Scrolls the pane under the pointer with the user's configured wheel direction.
    // ------------------=
    pub(super) fn accounts_scroll(&mut self, direction: i8) {
        let data = self.accounts_directory();
        let geometry = self.accounts_layout(&data);
        let layout = SystemLayout::new(
            self.system.framebuffer_width,
            self.system.framebuffer_height,
        );
        let point = layout.point(self.pointer_x, self.pointer_y);
        let amount = crate::ui::input_preferences::current().wheel(direction) * 20;
        let (offset, maximum) = if geometry.list.contains(point) {
            (&mut self.accounts.list_scroll, geometry.list_max)
        } else {
            (&mut self.accounts.detail_scroll, geometry.detail_max)
        };
        *offset = (*offset as i32 + amount).max(0) as usize;
        *offset = (*offset).min(maximum);
        self.accounts_changed();
    }
    // ------------------------=
    // FUNC: accounts_activate
    // DESC: Proposes one UI operation without changing committed rows optimistically.
    // ------------------=
    fn accounts_activate(&mut self, target: Target, data: &Directory) {
        let geometry = self.accounts_layout(data);
        if !geometry.controls[..geometry.count]
            .iter()
            .any(|c| c.target == target && c.enabled)
        {
            return;
        }
        let entry = self.accounts.selected_entry(data).copied();
        match target {
            Target::Users | Target::Groups => {
                self.accounts_secrets.clear();
                self.accounts.switch(if target == Target::Users {
                    Tab::Users
                } else {
                    Tab::Groups
                });
            }
            Target::New => {
                self.accounts_secrets.clear();
                self.accounts.begin(Mode::Create, data);
                self.accounts_secrets.session = self.current_session;
            }
            Target::Back => {
                self.accounts_cancel();
                self.accounts.detail = false;
            }
            Target::Row(i) => {
                self.accounts_secrets.clear();
                self.accounts.select(data, i);
                if geometry.narrow {
                    self.accounts.focus = Target::Back;
                }
            }
            Target::Rename | Target::Password | Target::Delete => {
                self.accounts_secrets.clear();
                self.accounts.begin(
                    match target {
                        Target::Rename => Mode::Rename,
                        Target::Password => Mode::Password,
                        _ => Mode::Delete,
                    },
                    data,
                );
                self.accounts_secrets.session = self.current_session;
            }
            Target::Cancel => self.accounts_cancel(),
            Target::Save | Target::Confirm => self.accounts_save(),
            Target::Enabled => {
                if let Some(e) = entry {
                    let result = crate::runtime::manage_accounts(
                        self.current_session,
                        AccountRequest::SetUserEnabled {
                            user: StableId(e.id),
                            enabled: !e.enabled,
                        },
                    );
                    self.accounts_result(result);
                }
            }
            Target::Member(i) => {
                if let Some(e) = entry {
                    let (group, user, present) = if self.accounts.tab == Tab::Users {
                        let g = data.groups[i];
                        (
                            StableId(g.id),
                            StableId(e.id),
                            self.accounts
                                .selected_index(data)
                                .is_some_and(|u| g.members & (1 << u) != 0),
                        )
                    } else {
                        let u = data.users[i];
                        (StableId(e.id), StableId(u.id), e.members & (1 << i) != 0)
                    };
                    let request = if present {
                        AccountRequest::RemoveMember { group, user }
                    } else {
                        AccountRequest::AddMember { group, user }
                    };
                    let result = crate::runtime::manage_accounts(self.current_session, request);
                    self.accounts_result(result);
                }
            }
            Target::ListUp => {
                self.accounts.list_scroll = self.accounts.list_scroll.saturating_sub(132)
            }
            Target::ListDown => {
                self.accounts.list_scroll = (self.accounts.list_scroll + 132).min(geometry.list_max)
            }
            Target::DetailUp => {
                self.accounts.detail_scroll = self.accounts.detail_scroll.saturating_sub(132)
            }
            Target::DetailDown => {
                self.accounts.detail_scroll =
                    (self.accounts.detail_scroll + 132).min(geometry.detail_max)
            }
            _ => {}
        }
    }
    // ------------------------=
    // FUNC: accounts_save
    // DESC: Validates form confirmation and sends borrowed secrets only to authenticated durable dispatch.
    // ------------------=
    fn accounts_save(&mut self) {
        let passwords = self.accounts.mode == Mode::Password
            || (self.accounts.mode == Mode::Create && self.accounts.tab == Tab::Users);
        if passwords && self.accounts_secrets.fields[0].len < 8 {
            self.accounts.feedback = Feedback::PasswordLength;
            return;
        }
        if passwords
            && self.accounts_secrets.fields[0].as_bytes()
                != self.accounts_secrets.fields[1].as_bytes()
        {
            self.accounts.feedback = Feedback::PasswordMismatch;
            return;
        }
        if matches!(self.accounts.mode, Mode::Create | Mode::Rename)
            && (self.accounts.name.len == 0 || self.accounts.handle.len == 0)
        {
            self.accounts.feedback = Feedback::Invalid;
            return;
        }
        let id = StableId(self.accounts.selected.unwrap_or([0; 16]));
        let now = crate::ui::performance::monotonic_ns().unwrap_or(0) / 1_000_000_000;
        let request = match (self.accounts.mode, self.accounts.tab) {
            (Mode::Create, Tab::Users) => AccountRequest::CreateUser {
                handle: self.accounts.handle.as_bytes(),
                display_name: self.accounts.name.as_bytes(),
                password: self.accounts_secrets.fields[0].as_bytes(),
                now,
            },
            (Mode::Create, Tab::Groups) => AccountRequest::CreateGroup {
                name: self.accounts.handle.as_bytes(),
                display_name: self.accounts.name.as_bytes(),
                now,
            },
            (Mode::Rename, Tab::Users) => AccountRequest::RenameUser {
                user: id,
                handle: self.accounts.handle.as_bytes(),
                display_name: self.accounts.name.as_bytes(),
            },
            (Mode::Rename, Tab::Groups) => AccountRequest::RenameGroup {
                group: id,
                name: self.accounts.handle.as_bytes(),
                display_name: self.accounts.name.as_bytes(),
            },
            (Mode::Password, Tab::Users) => AccountRequest::ResetPassword {
                user: id,
                password: self.accounts_secrets.fields[0].as_bytes(),
                now,
            },
            (Mode::Delete, Tab::Users) => AccountRequest::DeleteUser { user: id },
            (Mode::Delete, Tab::Groups) => AccountRequest::DeleteGroup { group: id },
            _ => return,
        };
        let result = crate::runtime::manage_accounts(self.current_session, request);
        self.accounts_secrets.clear();
        self.accounts_secrets.session = self.current_session;
        self.accounts.password_lengths = [0; 2];
        self.accounts_result(result);
    }
    // ------------------------=
    // FUNC: accounts_result
    // DESC: Shows durable success only after commit and refreshes authority after every mutation.
    // ------------------=
    fn accounts_result(&mut self, result: Result<AccountResult, IdentityError>) {
        match result {
            Ok(result) => {
                self.accounts.cancel();
                self.accounts.feedback = Feedback::Saved;
                match result {
                    AccountResult::User(user) => {
                        self.accounts.selected = Some(user.id.0);
                        self.accounts.detail = true;
                    }
                    AccountResult::Group(group) => {
                        self.accounts.selected = Some(group.id.0);
                        self.accounts.detail = true;
                    }
                    AccountResult::DeletedUser { .. } | AccountResult::DeletedGroup { .. } => {
                        self.accounts.selected = None;
                        self.accounts.detail = false;
                        self.accounts.feedback = Feedback::Deleted;
                    }
                    _ => {}
                }
            }
            Err(error) => self.accounts.feedback = failure(error),
        }
        if !self.accounts_directory().available
            && self.accounts.feedback != Feedback::RestartRequired
        {
            self.accounts_return_to_login();
        }
        self.accounts_changed();
    }
}
