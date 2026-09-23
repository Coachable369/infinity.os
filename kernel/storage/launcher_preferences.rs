//! Session-bound, bounded-history launcher preferences in the native object store.
use super::*;
// ------------------------=
// FUNC: allowed
// DESC: Requires the exact active owner session before reading or changing a launcher checkpoint.
// ------------------=
fn allowed(owner: [u8; 16], session: [u8; 16]) -> bool {
    owner != [0; 16]
        && crate::runtime::with_runtime(|r| {
            (0..crate::runtime::identity::MAX_SESSIONS)
                .filter_map(|i| r.identity.session_nth(i))
                .any(|s| {
                    s.id.0 == session
                        && s.user.0 == owner
                        && s.state == crate::runtime::identity::SessionState::Active
                })
        })
        .unwrap_or(false)
}
// ------------------------=
// FUNC: load
// DESC: Loads a validated owner's checkpoint through ordinary Personal-space read authorization.
// ------------------=
pub(crate) fn load(
    owner: [u8; 16],
    session: [u8; 16],
    out: &mut [u8],
) -> Result<usize, object::ObjectError> {
    if !allowed(owner, session) {
        return Err(object::ObjectError::Unauthorized);
    }
    object_read_path(
        &crate::ui::app_launcher::shortcuts::user_path(owner),
        None,
        out,
    )
    .map(|(_, n)| n)
}
// ------------------------=
// FUNC: save
// DESC: Atomically replaces app preferences without consuming another history slot on every drag.
// ------------------=
pub(crate) fn save(
    owner: [u8; 16],
    session: [u8; 16],
    bytes: &[u8],
) -> Result<(), object::ObjectError> {
    if !allowed(owner, session) {
        return Err(object::ObjectError::Unauthorized);
    }
    if crate::ui::app_launcher::shortcuts::State::decode(bytes).is_none() {
        return Err(object::ObjectError::CorruptContent);
    }
    let path = crate::ui::app_launcher::shortcuts::user_path(owner);
    with_store(|store| match store.resolve(&path) {
        Ok(id) => {
            let mut old = [0; crate::ui::app_launcher::shortcuts::STATE_BYTES];
            let n = store.read(id, None, &mut old)?;
            if crate::ui::app_launcher::shortcuts::State::decode(&old[..n]).is_none() {
                return Err(object::ObjectError::CorruptContent);
            }
            store.replace_state(id, bytes).map(|_| ())
        }
        Err(object::ObjectError::NotFound | object::ObjectError::NamespaceNotFound) => store
            .create_attached(
                b"Launcher layout",
                object::ObjectType::Metadata,
                object::Space::Personal,
                bytes,
                &path,
            )
            .map(|_| ()),
        Err(e) => Err(e),
    })
}
