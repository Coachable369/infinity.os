#[cfg(not(target_os = "none"))]
use rand_core::{CryptoRng, UnwrapErr};
#[allow(unused_imports)]
use {
    crate::error::{Error, Result, TrapBug},
    log::{debug, error, info, log, trace, warn},
};

#[cfg(not(target_os = "none"))]
pub(crate) fn rng() -> impl CryptoRng {
    UnwrapErr(getrandom::SysRng)
}

// ------------------------=
// FUNC: fill_random
// DESC: Uses native service entropy on InfinityOS and propagates failure without a panic or weak fallback.
// ------------------=
#[cfg_attr(target_os = "none", allow(unsafe_code))]
pub fn fill_random(buf: &mut [u8]) -> Result<(), Error> {
    #[cfg(not(target_os = "none"))]
    { getrandom::fill(buf).map_err(|_| Error::EntropyUnavailable) }
    #[cfg(target_os = "none")]
    {
        unsafe extern "C" {
            fn infinity_ssh_random(bytes: *mut u8, length: usize) -> u8;
        }
        // Service supplies a validated, domain-separated CSPRNG; no fallback.
        if unsafe { infinity_ssh_random(buf.as_mut_ptr(), buf.len()) } == 1 {
            Ok(())
        } else {
            buf.fill(0);
            Err(Error::EntropyUnavailable)
        }
    }
}
