//! Read authority for the native Python snapshot launcher.

// ------------------------=
// FUNC: readable
// DESC: Allows owned objects and the active shared Documents compatibility namespace, never foreign-owned or system objects.
// ------------------=
pub fn readable(owner: [u8; 16], user: [u8; 16], personal: bool, path: &[u8]) -> bool {
    personal && user != [0; 16] && (owner == user ||
        owner == [0; 16] && path.starts_with(b"/home/default/documents/"))
}
