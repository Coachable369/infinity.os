//! SSH's independent CSPRNG, seeded only from validated firmware entropy.
use core::sync::atomic::{AtomicBool, Ordering};
use crate::http_transport::{rand_chacha::ChaCha20Rng, rand_core::{RngCore, SeedableRng}};
use hkdf::Hkdf;
use sha2::Sha256;
use zeroize::Zeroizing;

static BUSY: AtomicBool = AtomicBool::new(false);
static mut RNG: Option<ChaCha20Rng> = None;
static mut GENERATED: u64 = 0;

// ------------------------=
// FUNC: initialize
// DESC: Seeds a domain-separated generator once; missing entropy leaves SSH unusable.
// ------------------=
pub(super) fn initialize(entropy: &[u8; 32], valid: bool) -> bool {
    if !valid || BUSY.swap(true, Ordering::Acquire) { return false; }
    let mut seed = Zeroizing::new([0; 32]);
    let derived = Hkdf::<Sha256>::new(Some(b"InfinityOS SSH entropy v1"), entropy)
        .expand(b"transport and host key", &mut *seed).is_ok();
    if derived {
        unsafe {
            if (&*(&raw const RNG)).is_none() { RNG = Some(ChaCha20Rng::from_seed(*seed)); }
        }
    }
    BUSY.store(false, Ordering::Release);
    derived
}

// ------------------------=
// FUNC: fill
// DESC: Fills bytes without blocking or fallback; a conservative lifetime bound prevents counter exhaustion.
// ------------------=
pub(super) fn fill(bytes: &mut [u8]) -> bool {
    if BUSY.swap(true, Ordering::Acquire) { bytes.fill(0); return false; }
    let ok = unsafe {
        match (&mut *(&raw mut RNG)).as_mut() {
            Some(rng) if bytes.len() <= 35_000 && GENERATED <= (1u64 << 40) - bytes.len() as u64 => {
                rng.fill_bytes(bytes);
                GENERATED += bytes.len() as u64;
                true
            }
            _ => false,
        }
    };
    if !ok { bytes.fill(0); }
    BUSY.store(false, Ordering::Release);
    ok
}

// ------------------------=
// FUNC: infinity_ssh_random
// DESC: Supplies Sunset's native entropy ABI; called only with its valid writable packet or seed buffers.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn infinity_ssh_random(bytes: *mut u8, length: usize) -> u8 {
    if bytes.is_null() || length > 35_000 { return 0; }
    fill(core::slice::from_raw_parts_mut(bytes, length)) as u8
}
