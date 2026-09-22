//! Private, session-bound spatial checkpoints in the installed native object store.
use super::*;
#[path = "spatial_path.rs"]
mod checkpoint_path;

// ------------------------=
// FUNC: path
// DESC: Validates the exact active session and derives its owner's private checkpoint namespace.
// ------------------=
fn path(
    owner: [u8; 16],
    session: [u8; 16],
) -> Result<[u8; checkpoint_path::PATH_BYTES], object::ObjectError> {
    let allowed = crate::runtime::with_runtime(|runtime| {
        (0..crate::runtime::identity::MAX_SESSIONS)
            .filter_map(|i| runtime.identity.session_nth(i))
            .any(|s| {
                s.id.0 == session
                    && s.user.0 == owner
                    && s.state == crate::runtime::identity::SessionState::Active
            })
    })
    .unwrap_or(false);
    if !allowed || owner == [0; 16] {
        return Err(object::ObjectError::Unauthorized);
    }
    Ok(checkpoint_path::owner_path(owner))
}

// ------------------------=
// FUNC: load
// DESC: Reads and validates only the currently authenticated owner's spatial metadata.
// ------------------=
pub(crate) fn load(
    owner: [u8; 16],
    session: [u8; 16],
) -> Result<crate::ui::spatial::SpatialState, object::ObjectError> {
    let path = path(owner, session)?;
    let mut bytes = [0u8; crate::ui::spatial::STATE_BYTES];
    #[cfg(any(target_arch = "aarch64", target_arch = "x86_64"))]
    let length = with_store(|store| {
        let id = store.resolve(&path)?;
        store.read_spatial_state(id, &mut bytes)
    })?;
    #[cfg(target_arch = "x86")]
    let length = 0;
    crate::ui::spatial::SpatialState::decode(owner, &bytes[..length])
        .map_err(|_| object::ObjectError::CorruptContent)
}

// ------------------------=
// FUNC: commit
// DESC: Atomically replaces a private checkpoint without accumulating unbounded version history.
// ------------------=
pub(crate) fn commit(
    owner: [u8; 16],
    session: [u8; 16],
    state: &crate::ui::spatial::SpatialState,
) -> Result<(), object::ObjectError> {
    let path = path(owner, session)?;
    let bytes = state
        .encode(owner)
        .map_err(|_| object::ObjectError::Unauthorized)?;
    #[cfg(any(target_arch = "aarch64", target_arch = "x86_64"))]
    {
        with_store(|store| match store.resolve(&path) {
            Ok(id) => {
                let mut previous = [0u8; crate::ui::spatial::STATE_BYTES];
                let length = store.read_spatial_state(id, &mut previous)?;
                crate::ui::spatial::SpatialState::decode(owner, &previous[..length])
                    .map_err(|_| object::ObjectError::CorruptContent)?;
                store.replace_state(id, &bytes).map(|_| ())
            }
            Err(object::ObjectError::NotFound | object::ObjectError::NamespaceNotFound) => store
                .create_attached(
                    b"@spatial-state",
                    object::ObjectType::Metadata,
                    object::Space::System,
                    &bytes,
                    &path,
                )
                .map(|_| ()),
            Err(e) => Err(e),
        })
    }
    #[cfg(target_arch = "x86")]
    {
        let _ = (path, bytes);
        Err(object::ObjectError::SpaceUnavailable)
    }
}
