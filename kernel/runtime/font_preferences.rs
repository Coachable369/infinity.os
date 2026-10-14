//! Bounded per-profile availability for the immutable, openly licensed System font library.
pub const FAMILY_COUNT: usize = 46;
pub const BYTES: usize = 16;
pub const FALLBACK: u8 = 2;
const ALL: u64 = (1u64 << FAMILY_COUNT) - 1;
use core::sync::atomic::{AtomicU64, Ordering};
static CURRENT: AtomicU64 = AtomicU64::new(ALL | ((FALLBACK as u64) << 48));

// ------------------------=
// FUNC: current
// DESC: Takes a coherent snapshot of the active profile's installed faces and selection.
// ------------------=
pub fn current() -> Preferences {
    let value=CURRENT.load(Ordering::Acquire);
    Preferences { installed:value & ALL, selected:(value >> 48) as u8 }
}
// ------------------------=
// FUNC: publish
// DESC: Publishes only a typed valid preference after the owning controller has committed storage.
// ------------------=
pub fn publish(value: Preferences) {
    CURRENT.store(value.installed | ((value.selected as u64) << 48), Ordering::Release);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Preferences {
    installed: u64,
    pub selected: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error { InvalidId, Protected, NotInstalled, InvalidState }

impl Preferences {
    // ------------------------=
    // FUNC: new
    // DESC: Makes every bundled OFL family available with the original Roboto fallback selected.
    // ------------------=
    pub const fn new() -> Self { Self { installed: ALL, selected: FALLBACK } }
    // ------------------------=
    // FUNC: contains
    // DESC: Checks availability without shifting by an untrusted identifier.
    // ------------------=
    pub fn contains(self, id: u8) -> bool {
        (1..=FAMILY_COUNT as u8).contains(&id) && self.installed & (1u64 << (id-1)) != 0
    }
    // ------------------------=
    // FUNC: count
    // DESC: Returns the exact number of installed families in this profile.
    // ------------------=
    pub fn count(self) -> usize { self.installed.count_ones() as usize }
    // ------------------------=
    // FUNC: select
    // DESC: Selects an installed family without permitting a missing rendering resource.
    // ------------------=
    pub fn select(&mut self, id: u8) -> Result<(), Error> {
        if !self.contains(id) { return Err(Error::NotInstalled); }
        self.selected=id; Ok(())
    }
    // ------------------------=
    // FUNC: install
    // DESC: Restores a bundled family from the immutable System Generation without duplicating its bytes.
    // ------------------=
    pub fn install(&mut self, id: u8) -> Result<(), Error> {
        if !(1..=FAMILY_COUNT as u8).contains(&id) { return Err(Error::InvalidId); }
        self.installed |= 1u64 << (id-1); Ok(())
    }
    // ------------------------=
    // FUNC: remove
    // DESC: Removes profile availability, preserving the fallback and atomically switching an active removed face.
    // ------------------=
    pub fn remove(&mut self, id: u8) -> Result<(), Error> {
        if id==FALLBACK { return Err(Error::Protected); }
        if !self.contains(id) { return Err(Error::NotInstalled); }
        self.installed &= !(1u64 << (id-1));
        if self.selected==id { self.selected=FALLBACK; }
        Ok(())
    }
    // ------------------------=
    // FUNC: encode
    // DESC: Encodes a versioned fixed-size native object checkpoint with reserved bytes zeroed.
    // ------------------=
    pub fn encode(self) -> [u8; BYTES] {
        let mut out=[0;BYTES]; out[..4].copy_from_slice(b"IFNT"); out[4]=1; out[5]=self.selected;
        out[8..].copy_from_slice(&self.installed.to_le_bytes()); out
    }
    // ------------------------=
    // FUNC: decode
    // DESC: Rejects unknown versions, dangling selections, invalid bits and missing fallback resources.
    // ------------------=
    pub fn decode(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len()!=BYTES || &bytes[..4]!=b"IFNT" || bytes[4]!=1 || bytes[6..8]!=[0,0] {
            return Err(Error::InvalidState);
        }
        let p=Self { installed:u64::from_le_bytes(bytes[8..16].try_into().map_err(|_|Error::InvalidState)?),selected:bytes[5] };
        if p.installed & !ALL !=0 || !p.contains(FALLBACK) || !p.contains(p.selected) { return Err(Error::InvalidState); }
        Ok(p)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    // ------------------------=
    // FUNC: lifecycle
    // DESC: Exercises every family's selection, removal, fallback and reinstall through serialization.
    // ------------------=
    fn lifecycle() {
        let mut p=Preferences::new(); assert!(p.count()>=20);
        for id in 1..=FAMILY_COUNT as u8 {
            p.select(id).unwrap();
            if id==FALLBACK {assert_eq!(p.remove(id),Err(Error::Protected));continue;}
            p.remove(id).unwrap(); assert_eq!(p.selected,FALLBACK); assert_eq!(p.count(),45);
            assert_eq!(p.select(id),Err(Error::NotInstalled));
            p=Preferences::decode(&p.encode()).unwrap(); assert!(!p.contains(id));
            p.install(id).unwrap(); p.select(id).unwrap(); assert_eq!(p.count(),46);
            assert_eq!(Preferences::decode(&p.encode()),Ok(p));
        }
    }
    #[test]
    // ------------------------=
    // FUNC: rejects_corruption
    // DESC: Rejects invalid registry records without changing an existing valid preference.
    // ------------------=
    fn rejects_corruption() {
        let p=Preferences::new();
        for len in 0..BYTES {assert!(Preferences::decode(&p.encode()[..len]).is_err());}
        for index in [0,4,6,7,15] {let mut b=p.encode();b[index]=255;assert!(Preferences::decode(&b).is_err());}
        let mut b=p.encode();b[5]=0;assert!(Preferences::decode(&b).is_err());
        b=p.encode();b[8]&=!2;assert!(Preferences::decode(&b).is_err());
        let mut q=p;for id in [0,47,255] {assert!(q.install(id).is_err());assert_eq!(q,p);}
    }
}
