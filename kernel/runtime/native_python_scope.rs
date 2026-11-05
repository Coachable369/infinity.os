//! Read authority for the native Python snapshot launcher.

// ------------------------=
// FUNC: readable
// DESC: Allows owned personal data, legacy shared Documents and the four immutable system scripts only.
// ------------------=
pub fn readable(owner: [u8; 16], user: [u8; 16], personal: bool, path: &[u8]) -> bool {
    user != [0; 16] && ((personal && (owner == user ||
        owner == [0; 16] && path.starts_with(b"/home/default/documents/"))) ||
        (!personal && owner==[0;16] && system_script(path)))
}

// ------------------------=
// FUNC: system_script
// DESC: Allows only the immutable bundled utility entry points, not arbitrary system objects.
// ------------------=
pub fn system_script(path:&[u8])->bool {
    matches!(path,b"/System/awk.py"|b"/System/sed.py"|b"/System/grep.py"|b"/System/figlet.py")
}
