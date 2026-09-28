//! Explicit fail-closed startup prerequisites shared by both native targets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum Error { Session=0x100, Boot, Entropy, Clock, Network, Seed, Worker, Viewport }

// ------------------------=
// FUNC: valid_viewport
// DESC: Admits wide desktop windows within the existing four-megapixel frame allocation, including 2560-wide displays.
// ------------------=
pub fn valid_viewport(width:u32,height:u32)->bool {
    width>0 && height>0 && width<=4096 && height<=4096
        && u64::from(width)*u64::from(height)<=2560*1600
}

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
    // ------------------------=
    // FUNC: desktop_viewports_respect_frame_capacity
    // DESC: Accepts wide and portrait desktop layouts without allowing oversized frame storage.
    // ------------------=
    #[test]
    fn desktop_viewports_respect_frame_capacity() {
        for (w,h) in [(1024,768),(2560,1440),(2560,1600),(1600,2560)] {
            assert!(valid_viewport(w,h));assert!(w as usize*h as usize*4<=16384000);
        }
        for (w,h) in [(0,768),(2560,1601),(3840,2160),(u32::MAX,1)] {assert!(!valid_viewport(w,h));}
    }
}
