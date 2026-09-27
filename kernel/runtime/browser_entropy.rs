//! Domain-separated browser random seeds from validated boot entropy.
//! The sequence survives engine teardown; exhaustion fails closed.
use core::sync::atomic::{AtomicU64, Ordering};
use hkdf::Hkdf;
use sha2::Sha256;

pub struct Seeds { next: AtomicU64 }
impl Seeds {
    // ------------------------=
    // FUNC: new
    // DESC: Creates a boot-lifetime sequence that must not be reset on browser close.
    // ------------------=
    pub const fn new() -> Self { Self { next: AtomicU64::new(1) } }

    // ------------------------=
    // FUNC: derive
    // DESC: Derives a distinct session seed without treating counters or identities as entropy.
    // ------------------=
    pub fn derive(&self, entropy: &[u8; 32], owner: &[u8; 16]) -> Option<[u8; 32]> {
        let sequence = self.next.fetch_update(Ordering::Relaxed, Ordering::Relaxed,
            |value| value.checked_add(1)).ok()?;
        let mut context = [0u8; 24];
        context[..16].copy_from_slice(owner);
        context[16..].copy_from_slice(&sequence.to_le_bytes());
        let mut seed = [0u8; 32];
        Hkdf::<Sha256>::new(Some(b"InfinityOS native browser RNG v2"), entropy)
            .expand(&context, &mut seed).ok()?;
        Some(seed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // ------------------------=
    // FUNC: restarts_and_owners_have_distinct_stream_seeds
    // DESC: Checks repeatable boot derivation and separation across launches, owners and entropy.
    // ------------------=
    #[test]
    fn restarts_and_owners_have_distinct_stream_seeds() {
        let seeds = Seeds::new();
        let first = seeds.derive(&[7; 32], &[1; 16]).unwrap();
        assert_ne!(first, seeds.derive(&[7; 32], &[1; 16]).unwrap());
        assert_eq!(first, Seeds::new().derive(&[7; 32], &[1; 16]).unwrap());
        assert_ne!(first, Seeds::new().derive(&[7; 32], &[2; 16]).unwrap());
        assert_ne!(first, Seeds::new().derive(&[8; 32], &[1; 16]).unwrap());
    }
    // ------------------------=
    // FUNC: exhaustion_never_wraps
    // DESC: Ensures a launch sequence cannot silently repeat after integer exhaustion.
    // ------------------=
    #[test]
    fn exhaustion_never_wraps() {
        let seeds = Seeds { next: AtomicU64::new(u64::MAX - 1) };
        assert!(seeds.derive(&[7; 32], &[1; 16]).is_some());
        assert_eq!(seeds.derive(&[7; 32], &[1; 16]), None);
        assert_eq!(seeds.derive(&[7; 32], &[1; 16]), None);
    }
}
