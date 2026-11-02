//! Typed account command preparation, independent of Console rendering and storage.

use crate::runtime::console_language::OperationNode;
use crate::runtime::identity::{AccountReader, AccountRequest, AccountResult, GroupIdentity, IdentityError, ShortText, StableId, UserIdentity, MAX_GROUPS, MAX_USERS};
use crate::runtime::iop::OperationId;
use super::account_password::PasswordAction;

pub enum AccountCommand {
    Users { users: [Option<UserIdentity>; MAX_USERS], count: usize },
    Groups { groups: [Option<GroupIdentity>; MAX_GROUPS], count: usize },
    User { user: UserIdentity, groups: [Option<GroupIdentity>; MAX_GROUPS], count: usize },
    Group { group: GroupIdentity, users: [Option<UserIdentity>; MAX_USERS], count: usize },
    Password(PasswordAction),
    Mutation(AccountMutation),
}

pub enum AccountMutation {
    RenameUser { user: StableId, handle: ShortText, display_name: ShortText },
    SetUserEnabled { user: StableId, enabled: bool },
    DeleteUser { user: StableId },
    CreateGroup { name: ShortText, display_name: ShortText },
    RenameGroup { group: StableId, name: ShortText, display_name: ShortText },
    DeleteGroup { group: StableId },
    AddMember { group: StableId, user: StableId },
    RemoveMember { group: StableId, user: StableId },
}

impl AccountMutation {
    // ------------------------=
    // FUNC: execute
    // DESC: Borrows validated owned fields into the shared backend or durable runtime wrapper.
    // ------------------=
    pub fn execute(self, now: u64, execute: impl FnOnce(AccountRequest<'_>) -> Result<AccountResult, IdentityError>) -> Result<AccountResult, IdentityError> {
        match self {
            Self::RenameUser { user, handle, display_name } => execute(AccountRequest::RenameUser { user, handle: handle.as_bytes(), display_name: display_name.as_bytes() }),
            Self::SetUserEnabled { user, enabled } => execute(AccountRequest::SetUserEnabled { user, enabled }),
            Self::DeleteUser { user } => execute(AccountRequest::DeleteUser { user }),
            Self::CreateGroup { name, display_name } => execute(AccountRequest::CreateGroup { name: name.as_bytes(), display_name: display_name.as_bytes(), now }),
            Self::RenameGroup { group, name, display_name } => execute(AccountRequest::RenameGroup { group, name: name.as_bytes(), display_name: display_name.as_bytes() }),
            Self::DeleteGroup { group } => execute(AccountRequest::DeleteGroup { group }),
            Self::AddMember { group, user } => execute(AccountRequest::AddMember { group, user }),
            Self::RemoveMember { group, user } => execute(AccountRequest::RemoveMember { group, user }),
        }
    }
}

// ------------------------=
// FUNC: prepare
// DESC: Resolves explicit targets under authenticated read authority and prepares typed data without mutating identity state.
// ------------------=
pub fn prepare(reader: &AccountReader<'_>, node: &OperationNode<'_>) -> Result<AccountCommand, IdentityError> {
    let target = node.target.map(|reference| reference.value);
    match node.schema.operation {
        OperationId::IdentityList => {
            let mut users = [None; MAX_USERS];
            let count = reader.user_count();
            for (index, slot) in users.iter_mut().enumerate().take(count) { *slot = reader.user_nth(index); }
            Ok(AccountCommand::Users { users, count })
        }
        OperationId::IdentityCreate => Ok(AccountCommand::Password(PasswordAction::Create {
            handle: ShortText::new(required(node, b"handle")?)?,
            display_name: ShortText::new(required(node, b"display-name")?)?,
        })),
        OperationId::IdentityRead | OperationId::IdentityUpdate | OperationId::IdentityDelete | OperationId::IdentitySetEnabled | OperationId::IdentityResetPassword => {
            let user = resolve_user(reader, target.ok_or(IdentityError::InvalidInput)?)?;
            match node.schema.operation {
                OperationId::IdentityRead => {
                    let mut groups = [None; MAX_GROUPS];
                    let count = reader.user_group_count(user.id);
                    for (index, slot) in groups.iter_mut().enumerate().take(count) { *slot = reader.user_group_nth(user.id, index); }
                    if node.schema.action == b"groups" { Ok(AccountCommand::Groups { groups, count }) }
                    else { Ok(AccountCommand::User { user, groups, count }) }
                }
                OperationId::IdentityUpdate => {
                    if argument(node, b"handle").is_none() && argument(node, b"display-name").is_none() { return Err(IdentityError::InvalidInput); }
                    Ok(AccountCommand::Mutation(AccountMutation::RenameUser {
                        user: user.id,
                        handle: text_or(node, b"handle", user.handle)?,
                        display_name: text_or(node, b"display-name", user.display_name)?,
                    }))
                }
                OperationId::IdentityDelete => Ok(AccountCommand::Mutation(AccountMutation::DeleteUser { user: user.id })),
                OperationId::IdentitySetEnabled => Ok(AccountCommand::Mutation(AccountMutation::SetUserEnabled { user: user.id, enabled: node.schema.action == b"enable" })),
                OperationId::IdentityResetPassword => Ok(AccountCommand::Password(PasswordAction::Reset { user: user.id })),
                _ => Err(IdentityError::InvalidInput),
            }
        }
        OperationId::GroupList => {
            let mut groups = [None; MAX_GROUPS];
            let count = reader.group_count();
            for (index, slot) in groups.iter_mut().enumerate().take(count) { *slot = reader.group_nth(index); }
            Ok(AccountCommand::Groups { groups, count })
        }
        OperationId::GroupCreate => {
            let name = ShortText::new(required(node, b"name")?)?;
            Ok(AccountCommand::Mutation(AccountMutation::CreateGroup { name, display_name: text_or(node, b"display-name", name)? }))
        }
        OperationId::GroupRead | OperationId::GroupUpdate | OperationId::GroupDelete | OperationId::GroupAddMember | OperationId::GroupRemoveMember => {
            let group = resolve_group(reader, target.ok_or(IdentityError::InvalidInput)?)?;
            match node.schema.operation {
                OperationId::GroupRead => {
                    let mut users = [None; MAX_USERS];
                    let count = reader.group_member_count(group.id);
                    for (index, slot) in users.iter_mut().enumerate().take(count) { *slot = reader.group_member_nth(group.id, index); }
                    Ok(AccountCommand::Group { group, users, count })
                }
                OperationId::GroupUpdate => {
                    if argument(node, b"name").is_none() && argument(node, b"display-name").is_none() { return Err(IdentityError::InvalidInput); }
                    Ok(AccountCommand::Mutation(AccountMutation::RenameGroup { group: group.id, name: text_or(node, b"name", group.name)?, display_name: text_or(node, b"display-name", group.display_name)? }))
                }
                OperationId::GroupDelete => Ok(AccountCommand::Mutation(AccountMutation::DeleteGroup { group: group.id })),
                OperationId::GroupAddMember | OperationId::GroupRemoveMember => {
                    let reference = required(node, b"user")?;
                    let user = resolve_user(reader, reference.strip_prefix(b"user:").unwrap_or(reference))?;
                    Ok(AccountCommand::Mutation(if node.schema.operation == OperationId::GroupAddMember {
                        AccountMutation::AddMember { group: group.id, user: user.id }
                    } else { AccountMutation::RemoveMember { group: group.id, user: user.id } }))
                }
                _ => Err(IdentityError::InvalidInput),
            }
        }
        _ => Err(IdentityError::InvalidInput),
    }
}

// ------------------------=
// FUNC: resolve_user
// DESC: Resolves a stable numeric reference or exact handle; missing explicit targets never select the current user.
// ------------------=
pub fn resolve_user(reader: &AccountReader<'_>, reference: &[u8]) -> Result<UserIdentity, IdentityError> {
    let reference = reference.strip_prefix(b"user:").unwrap_or(reference);
    if reference.iter().all(u8::is_ascii_digit) && !reference.is_empty() {
        let id = number(reference).ok_or(IdentityError::InvalidInput)?;
        (0..reader.user_count()).filter_map(|index| reader.user_nth(index)).find(|user| user.id.short() == id).ok_or(IdentityError::NotFound)
    } else { reader.user_by_handle(reference).ok_or(IdentityError::NotFound) }
}

// ------------------------=
// FUNC: resolve_group
// DESC: Resolves a stable group number or exact group name rather than a mutable list index.
// ------------------=
pub fn resolve_group(reader: &AccountReader<'_>, reference: &[u8]) -> Result<GroupIdentity, IdentityError> {
    let reference = reference.strip_prefix(b"group:").unwrap_or(reference);
    if reference.iter().all(u8::is_ascii_digit) && !reference.is_empty() {
        let id = number(reference).ok_or(IdentityError::InvalidInput)?;
        (0..reader.group_count()).filter_map(|index| reader.group_nth(index)).find(|group| group.id.short() == id).ok_or(IdentityError::NotFound)
    } else { reader.group_by_name(reference).ok_or(IdentityError::NotFound) }
}

// ------------------------=
// FUNC: number
// DESC: Parses a nonempty decimal identity reference with checked arithmetic.
// ------------------=
fn number(bytes: &[u8]) -> Option<u64> {
    if bytes.is_empty() { return None; }
    bytes.iter().try_fold(0u64, |value, byte| {
        if !byte.is_ascii_digit() { return None; }
        value.checked_mul(10)?.checked_add((byte - b'0') as u64)
    })
}

// ------------------------=
// FUNC: argument
// DESC: Retrieves a typed named argument without retaining the command line.
// ------------------=
fn argument<'a>(node: &OperationNode<'a>, name: &[u8]) -> Option<&'a [u8]> {
    node.arguments.iter().flatten().find(|argument| argument.name == name).map(|argument| argument.value)
}

// ------------------------=
// FUNC: required
// DESC: Requires an argument already admitted by the command schema.
// ------------------=
fn required<'a>(node: &OperationNode<'a>, name: &[u8]) -> Result<&'a [u8], IdentityError> { argument(node, name).ok_or(IdentityError::InvalidInput) }

// ------------------------=
// FUNC: text_or
// DESC: Preserves an existing typed field when a rename request omits it.
// ------------------=
fn text_or(node: &OperationNode<'_>, name: &[u8], fallback: ShortText) -> Result<ShortText, IdentityError> {
    argument(node, name).map(ShortText::new).unwrap_or(Ok(fallback))
}

// ------------------------=
// FUNC: sensitive_command
// DESC: Suppresses echo and history for account commands even when malformed input includes an unexpected secret argument.
// ------------------=
pub fn sensitive_command(input: &[u8]) -> bool {
    input.split(|byte| byte.is_ascii_whitespace() || *byte == b'=' || *byte == b'|')
        .any(|word| [b"user".as_slice(), b"group", b"identity", b"credential", b"password", b"passwd"].iter().any(|name| word.eq_ignore_ascii_case(name)))
}
