//! Installation-scoped physical identities, independent of capacity, namespace,
//! object authority and the NodeId subsequently created by installed boot.
use sha2::{Digest, Sha256};
use core::sync::atomic::{AtomicBool, AtomicU8, Ordering};

static READY: AtomicBool = AtomicBool::new(false);
static BOOT_IDENTITY: [AtomicU8; 32] = [const { AtomicU8::new(0) }; 32];

// ------------------------=
// FUNC: initialize
// DESC: Captures domain-separated installation uniqueness before live-only node-service persistence can clear its state; this is called during single-threaded boot, never from a guest request.
// ------------------=
pub(crate) fn initialize(entropy: &[u8; 32], valid: bool) {
    READY.store(false, Ordering::Release);
    if !valid || *entropy == [0; 32] { return; }
    let mut digest = Sha256::new();
    digest.update(b"InfinityOS/installer-boot-identity/v1"); digest.update(entropy);
    for (slot, value) in BOOT_IDENTITY.iter().zip(digest.finalize()) { slot.store(value, Ordering::Relaxed); }
    READY.store(true, Ordering::Release);
}

// ------------------------=
// FUNC: for_target
// DESC: Derives stable reviewed-plan IDs from boot-owned installation state without depending on the live node service, network state or persistent identity availability.
// ------------------=
pub(crate) fn for_target(target: &[u8]) -> Option<Identities> {
    if !READY.load(Ordering::Acquire) { return None; }
    derive(core::array::from_fn(|index| BOOT_IDENTITY[index].load(Ordering::Relaxed)), target)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Identities {
    pub disk: [u8; 16], pub esp: [u8; 16], pub container: [u8; 16],
    pub pool: [u8; 16], pub spaces: [[u8; 16]; 4],
}

// ------------------------=
// FUNC: derive
// DESC: Domain-separates physical IDs using the entropy-backed live-boot public identity and discovered target identity; absent uniqueness material fails closed.
// ------------------=
pub fn derive(boot_identity: [u8; 32], target: &[u8]) -> Option<Identities> {
    if boot_identity == [0; 32] || target.is_empty() || target.len() > 128 { return None; }
    let mut ids = [[0; 16]; 8];
    for (role, id) in ids.iter_mut().enumerate() {
        let mut hash = Sha256::new();
        hash.update(b"InfinityOS/physical-installation/v1");
        hash.update(boot_identity); hash.update((target.len() as u32).to_le_bytes());
        hash.update(target); hash.update([role as u8]);
        id.copy_from_slice(&hash.finalize()[..16]);
        // UUIDv8: domain-separated hash identifiers, not claims of random v4.
        id[6] = (id[6] & 15) | 0x80; id[8] = (id[8] & 63) | 0x80;
    }
    Some(Identities { disk: ids[0], esp: ids[1], container: ids[2], pool: ids[3],
        spaces: ids[4..8].try_into().ok()? })
}

#[cfg(test)]
mod tests {
    use super::*;
    // ------------------------=
    // FUNC: independent_installations_and_targets_have_independent_physical_ids
    // DESC: Verifies plan reproducibility, domain separation and unique IDs for otherwise identical disks across independent entropy-backed boot identities.
    // ------------------=
    #[test]
    fn independent_installations_and_targets_have_independent_physical_ids() {
        let reference = derive([1; 32], b"disk0").unwrap();
        assert_eq!(derive([1; 32], b"disk0"), Some(reference));
        for identity in [derive([2; 32], b"disk0").unwrap(), derive([1; 32], b"disk1").unwrap()] {
            let a = [reference.disk, reference.esp, reference.container, reference.pool,
                reference.spaces[0], reference.spaces[1], reference.spaces[2], reference.spaces[3]];
            let b = [identity.disk, identity.esp, identity.container, identity.pool,
                identity.spaces[0], identity.spaces[1], identity.spaces[2], identity.spaces[3]];
            for (index, id) in b.iter().enumerate() {
                assert!(!a.contains(id)); assert!(!b[..index].contains(id));
                assert_eq!(id[6] >> 4, 8); assert_eq!(id[8] >> 6, 2);
            }
        }
        assert_eq!(derive([0; 32], b"disk0"), None);
        assert_eq!(derive([1; 32], b""), None);
        assert_eq!(derive([1; 32], &[1; 129]), None);
    }

    // ------------------------=
    // FUNC: boot_owned_installation_identity_survives_without_node_service
    // DESC: Exercises boot initialization, repeat plan validation, independent boots and missing-entropy failure with no NodeRuntime or object store present.
    // ------------------=
    #[test]
    fn boot_owned_installation_identity_survives_without_node_service() {
        initialize(&[11; 32], true);
        let first = for_target(b"disk0").unwrap();
        assert_eq!(for_target(b"disk0"), Some(first));
        initialize(&[12; 32], true);
        let second = for_target(b"disk0").unwrap();
        assert_ne!(first.disk, second.disk); assert_ne!(first.container, second.container);
        initialize(&[12; 32], false); assert_eq!(for_target(b"disk0"), None);
        initialize(&[0; 32], true); assert_eq!(for_target(b"disk0"), None);
    }
}
