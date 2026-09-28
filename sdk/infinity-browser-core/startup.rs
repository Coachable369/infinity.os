//! Explicit fail-closed startup prerequisites shared by both native targets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum Error { Session=0x100, Boot, Entropy, Clock, Network, Seed, Worker, Viewport }

// ------------------------=
// FUNC: prerequisites
// DESC: Requires trusted randomness and certificate time before admitting a browser worker.
// ------------------=
pub fn prerequisites(entropy:bool, seconds:Option<u64>)->Result<u64,Error> {
    if !entropy {return Err(Error::Entropy);}
    seconds.ok_or(Error::Clock)
}

#[cfg(test)]
mod tests {
    use super::*;
    // ------------------------=
    // FUNC: missing_prerequisites_fail_closed_and_recover
    // DESC: Rejects missing entropy/time and admits a retry only after both real prerequisites exist.
    // ------------------=
    #[test]
    fn missing_prerequisites_fail_closed_and_recover() {
        assert_eq!(prerequisites(false,Some(123)),Err(Error::Entropy));
        assert_eq!(prerequisites(false,None),Err(Error::Entropy));
        assert_eq!(prerequisites(true,None),Err(Error::Clock));
        assert_eq!(prerequisites(true,Some(123)),Ok(123));
    }
}
