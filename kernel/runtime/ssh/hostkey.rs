//! Host key lifecycle. Storage errors never cause silent rotation or ephemeral fallback.
use super::{ed25519_dalek, Failure, HostKey};
use zeroize::Zeroizing;

pub const HOST_KEY_BYTES: usize = 74;
pub trait HostKeyStorage {
    // ------------------------=
    // FUNC: read_protected
    // DESC: Reads only a service-private persistent object; false means absent, never permission or IO failure.
    // ------------------=
    fn read_protected(&mut self, out: &mut [u8; HOST_KEY_BYTES]) -> Result<bool, Failure>;
    // ------------------------=
    // FUNC: create_protected
    // DESC: Atomically creates a nonreplaceable service-private object, failing if protection or durability is unavailable.
    // ------------------=
    fn create_protected(&mut self, content: &[u8; HOST_KEY_BYTES]) -> Result<(), Failure>;
}
// ------------------------=
// FUNC: load_or_create
// DESC: Uses a durable service-owned key; only a genuinely absent key is generated and read back before listening.
// ------------------=
pub fn load_or_create(store: &mut impl HostKeyStorage, entropy: impl FnOnce(&mut [u8]) -> bool) -> Result<HostKey, Failure> {
    let mut bytes = Zeroizing::new([0; HOST_KEY_BYTES]);
    if !store.read_protected(&mut bytes)? {
        let mut seed = Zeroizing::new([0; 32]);
        if !entropy(&mut *seed) { return Err(Failure::EntropyUnavailable); }
        let key = ed25519_dalek::SigningKey::from_bytes(&seed);
        bytes[..8].copy_from_slice(b"INFSSHK1");
        bytes[8..10].copy_from_slice(&1u16.to_le_bytes());
        bytes[10..42].copy_from_slice(&*seed);
        bytes[42..].copy_from_slice(&key.verifying_key().to_bytes());
        store.create_protected(&bytes)?;
        let mut persisted = Zeroizing::new([0; HOST_KEY_BYTES]);
        if !store.read_protected(&mut persisted)? || *persisted != *bytes {
            return Err(Failure::HostKeyUnavailable);
        }
    }
    if &bytes[..8] != b"INFSSHK1" || bytes[8..10] != [1, 0] { return Err(Failure::HostKeyUnavailable); }
    let seed = Zeroizing::new(<[u8; 32]>::try_from(&bytes[10..42]).map_err(|_| Failure::HostKeyUnavailable)?);
    let key = ed25519_dalek::SigningKey::from_bytes(&seed);
    if key.verifying_key().to_bytes() != bytes[42..] { return Err(Failure::HostKeyUnavailable); }
    Ok(HostKey::Ed25519(key))
}
