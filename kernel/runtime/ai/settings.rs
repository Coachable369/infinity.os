//! Trusted settings mutation boundary; inference produces proposals, never authority.
use super::control::setting_command::Request;
use crate::runtime::identity::{IdentitySystem, IdentityError, StableId, SessionState, SESSION_SETTINGS_UPDATE_ALLOWED};

// ------------------------=
// FUNC: authorize
// DESC: Validates the live session before changing the authoritative identity record; the caller must durably commit before reporting success.
// ------------------=
pub fn authorize(identity: &IdentitySystem, session: StableId, user: StableId) -> Result<(), IdentityError> {
    let allowed = (0..crate::runtime::identity::MAX_SESSIONS)
        .filter_map(|i| identity.session_nth(i))
        .any(|s| s.id == session && s.user == user && s.state == SessionState::Active
            && s.capabilities & SESSION_SETTINGS_UPDATE_ALLOWED != 0);
    if !allowed { return Err(IdentityError::AccessDenied); }
    Ok(())
}

// ------------------------=
// FUNC: matches
// DESC: Reads back authoritative values before an execution receipt can claim success.
// ------------------=
pub fn matches(identity:&IdentitySystem,user:StableId,request:Request)->bool {
    match request {
        Request::Name{bytes,length}=>bytes.get(..length).is_some_and(|name|
            identity.machine().is_some_and(|m|m.display_name.as_bytes()==name)),
        Request::Primary(rgb)=>identity.primary_rgb()==rgb,
        Request::Accent(rgb)=>identity.user_profile(user).is_some_and(|p|p.accent_rgb==rgb),
        Request::Opacity(value)=>identity.background_effects().0==value,
        Request::Blur(value)=>identity.background_effects().1==value,
        Request::LockMinutes(value)=>identity.user_profile(user).is_some_and(|p|p.no_activity_timeout_minutes==value),
        Request::Speech(value)=>identity.ai_profile(user).is_some_and(|p|p.speech_output_enabled==value),
        Request::Theme(light)=>identity.user_profile(user).is_some_and(|p|p.theme.as_bytes()==
            if light{b"Diagnostic Light".as_slice()}else{b"Cosmic Dark"}),
        Request::World{..}|Request::Invalid=>false,
    }
}

// ------------------------=
// FUNC: apply
// DESC: Mutates only authorized identity settings; world changes use the native workspace controller.
// ------------------=
pub fn apply(identity: &mut IdentitySystem, session: StableId, user: StableId, request: Request) -> Result<(), IdentityError> {
    authorize(identity,session,user)?;
    match request {
        Request::World{..} => return Err(IdentityError::InvalidInput),
        Request::Name { bytes, length } => { identity.update_machine_name(bytes.get(..length).ok_or(IdentityError::InvalidInput)?)?; }
        Request::Primary(rgb) => { identity.update_primary(user, rgb)?; }
        Request::Accent(rgb) => { identity.update_user_accent(user, user, rgb)?; }
        Request::Opacity(opacity) => {
            let (_, blur) = identity.background_effects();
            identity.update_background_effects(user, opacity, blur)?;
        }
        Request::Blur(blur) => {
            let (opacity, _) = identity.background_effects();
            identity.update_background_effects(user, opacity, blur)?;
        }
        Request::LockMinutes(minutes) => { identity.update_user_no_activity_timeout(user, user, minutes)?; }
        Request::Speech(enabled) => { identity.update_speech_output(user, user, enabled)?; }
        Request::Theme(light) => { identity.update_user_theme(user, user,
            if light { b"Diagnostic Light" } else { b"Cosmic Dark" })?; }
        Request::Invalid => return Err(IdentityError::InvalidInput),
    }
    Ok(())
}
