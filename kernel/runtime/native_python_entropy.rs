//! Boot-seeded, domain-separated Python launch secrets; never substitutes clocks for entropy.
use hkdf::Hkdf;
use sha2::Sha256;
static mut ENTROPY: Option<[u8; 32]> = None;
static mut SEQUENCE: u64 = 0;

// ------------------------=
// FUNC: initialize
// DESC: Records validated firmware entropy once on the boot processor before Python can launch.
// ------------------=
pub unsafe fn initialize(entropy: &[u8; 32], valid: bool) {
    if (&*(&raw const ENTROPY)).is_none() && valid { ENTROPY = Some(*entropy); }
}

// ------------------------=
// FUNC: next
// DESC: Derives a unique owner-bound launch secret on the BSP, failing closed without entropy or on exhaustion.
// ------------------=
pub unsafe fn next(owner: &[u8; 16]) -> Option<[u8; 32]> {
    let entropy = (&*(&raw const ENTROPY)).as_ref()?;
    SEQUENCE = SEQUENCE.checked_add(1)?;
    let mut context = [0; 24]; context[..16].copy_from_slice(owner);
    context[16..].copy_from_slice(&SEQUENCE.to_le_bytes());
    let mut seed = [0; 32];
    Hkdf::<Sha256>::new(Some(b"InfinityOS Python launch seed v1"), entropy).expand(&context, &mut seed).ok()?;
    Some(seed)
}
