//! Canonical private checkpoint keys, shared by load and commit.
const PREFIX: &[u8] = b"/system/spatial/";
pub(super) const PATH_BYTES: usize = PREFIX.len() + 32;

// ------------------------=
// FUNC: owner_path
// DESC: Encodes every owner byte without hard-coded namespace offsets.
// ------------------=
pub(super) fn owner_path(owner: [u8; 16]) -> [u8; PATH_BYTES] {
    let mut path = [0; PATH_BYTES];
    path[..PREFIX.len()].copy_from_slice(PREFIX);
    for (i, b) in owner.iter().enumerate() {
        path[PREFIX.len() + 2 * i] = b"0123456789abcdef"[(b >> 4) as usize];
        path[PREFIX.len() + 2 * i + 1] = b"0123456789abcdef"[(b & 15) as usize];
    }
    path
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    // ------------------------=
    // FUNC: owner_key_preserves_every_identifier_byte
    // DESC: Exercises the native key encoder for all byte values and decodes the result to verify identity.
    // ------------------=
    fn owner_key_preserves_every_identifier_byte() {
        for value in 0..=255u8 {
            let owner = core::array::from_fn(|i| value.wrapping_add(i as u8));
            let path = owner_path(owner);
            assert_eq!(path.len(), 48);
            assert_eq!(&path[..PREFIX.len()], PREFIX);
            for (i, pair) in path[PREFIX.len()..].chunks_exact(2).enumerate() {
                let decoded = u8::from_str_radix(core::str::from_utf8(pair).unwrap(), 16).unwrap();
                assert_eq!(decoded, owner[i]);
            }
            let mut other = owner;
            other[15] ^= 1;
            assert_ne!(path, owner_path(other));
        }
    }
}
