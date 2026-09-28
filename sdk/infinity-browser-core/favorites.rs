//! Bounded, versioned favorites stored as one native object, never host files.
pub const CAPACITY:usize=32;
pub const BYTES:usize=16*1024;
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Error {Invalid,Full,Missing}
#[derive(Clone,Copy)]
pub struct Favorites {bytes:[u8;BYTES],length:usize,count:usize}
// ------------------------=
// FUNC: owner_path
// DESC: Derives a stable private native namespace from every byte of a profile identifier.
// ------------------=
pub fn owner_path(user:[u8;16])->[u8;57] {
    let mut path=[0u8;57];path[..15].copy_from_slice(b"/system/web/uid");
    for (i,byte) in user.into_iter().enumerate() {path[15+i*2]=b"0123456789abcdef"[(byte>>4) as usize];path[16+i*2]=b"0123456789abcdef"[(byte&15) as usize];}
    path[47..].copy_from_slice(b"/favorites");path
}
impl Favorites {
    // ------------------------=
    // FUNC: new
    // DESC: Creates an empty version-one favorites object.
    // ------------------=
    pub const fn new()->Self {let mut bytes=[0;BYTES];bytes[0]=b'I';bytes[1]=b'F';bytes[2]=b'A';bytes[3]=1;Self{bytes,length:4,count:0}}
    // ------------------------=
    // FUNC: decode
    // DESC: Validates every record and rejects duplicates, truncation and unknown versions.
    // ------------------=
    pub fn decode(bytes:&[u8])->Result<Self,Error> {
        if bytes.len()<4 || bytes[..4]!=[b'I',b'F',b'A',1] || bytes.len()>BYTES {return Err(Error::Invalid);}
        let mut result=Self::new();let mut at=4;
        while at<bytes.len() {
            if at+3>bytes.len() {return Err(Error::Invalid);}
            let url=u16::from_le_bytes([bytes[at],bytes[at+1]]) as usize;let title=bytes[at+2] as usize;at+=3;
            if at+url+title>bytes.len() {return Err(Error::Invalid);}
            result.add(&bytes[at..at+url],&bytes[at+url..at+url+title])?;at+=url+title;
        }Ok(result)
    }
    // ------------------------=
    // FUNC: bytes
    // DESC: Exposes only the populated versioned durable payload.
    // ------------------=
    pub fn bytes(&self)->&[u8] {&self.bytes[..self.length]}
    // ------------------------=
    // FUNC: count
    // DESC: Returns the number of validated saved pages.
    // ------------------=
    pub fn count(&self)->usize {self.count}
    // ------------------------=
    // FUNC: get
    // DESC: Borrows the URL and title of a saved page in insertion order.
    // ------------------=
    pub fn get(&self,index:usize)->Option<(&[u8],&[u8])> {
        let mut at=4;for i in 0..self.count {
            let url=u16::from_le_bytes([self.bytes[at],self.bytes[at+1]]) as usize;let title=self.bytes[at+2] as usize;at+=3;
            if i==index {return Some((&self.bytes[at..at+url],&self.bytes[at+url..at+url+title]));}at+=url+title;
        }None
    }
    // ------------------------=
    // FUNC: find
    // DESC: Matches the complete saved URL without conflating distinct queries or paths.
    // ------------------=
    pub fn find(&self,url:&[u8])->Option<usize> {(0..self.count).find(|&i|self.get(i).unwrap().0==url)}
    // ------------------------=
    // FUNC: add
    // DESC: Adds a unique HTTP(S) destination, checking all limits before changing state.
    // ------------------=
    pub fn add(&mut self,url:&[u8],title:&[u8])->Result<(),Error> {
        if url.len()>2048 || title.is_empty() || title.len()>96 || core::str::from_utf8(title).is_err()
            || core::str::from_utf8(url).is_err() || url.iter().any(|b|*b<=32)
            || !(url.starts_with(b"https://")&&url.len()>8 || url.starts_with(b"http://")&&url.len()>7)
            || self.find(url).is_some() {return Err(Error::Invalid);}
        let end=self.length+3+url.len()+title.len();if self.count==CAPACITY || end>BYTES {return Err(Error::Full);}
        let at=self.length;self.bytes[at..at+2].copy_from_slice(&(url.len() as u16).to_le_bytes());self.bytes[at+2]=title.len() as u8;
        self.bytes[at+3..at+3+url.len()].copy_from_slice(url);self.bytes[at+3+url.len()..end].copy_from_slice(title);
        self.length=end;self.count+=1;Ok(())
    }
    // ------------------------=
    // FUNC: remove
    // DESC: Removes exactly one record while preserving all other titles and destinations.
    // ------------------=
    pub fn remove(&mut self,index:usize)->Result<(),Error> {
        if index>=self.count {return Err(Error::Missing);}
        let mut at=4;for i in 0..self.count {
            let size=3+u16::from_le_bytes([self.bytes[at],self.bytes[at+1]]) as usize+self.bytes[at+2] as usize;
            if i==index {self.bytes.copy_within(at+size..self.length,at);self.length-=size;self.count-=1;return Ok(());}at+=size;
        }Err(Error::Missing)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    // ------------------------=
    // FUNC: profile_keys_preserve_identity
    // DESC: Checks every identifier byte survives namespace encoding and prevents cross-profile key collisions.
    // ------------------=
    #[test] fn profile_keys_preserve_identity() {
        for n in 0..=255u8 {let owner=core::array::from_fn(|i|n.wrapping_add(i as u8));let path=owner_path(owner);
            for (i,pair) in path[15..47].chunks_exact(2).enumerate() {
                assert_eq!(u8::from_str_radix(core::str::from_utf8(pair).unwrap(),16).unwrap(),owner[i]);
            }
            let mut other=owner;other[15]^=1;assert_ne!(path,owner_path(other));
        }
    }
    // ------------------------=
    // FUNC: roundtrip_remove_and_reject_corruption
    // DESC: Exercises durable replay, stable deletion, duplicates and malformed records.
    // ------------------=
    #[test]
    fn roundtrip_remove_and_reject_corruption() {
        let mut f=Favorites::new();f.add(b"https://example.com/a",b"Alpha").unwrap();f.add(b"https://example.com/b",b"Beta").unwrap();
        let mut restored=Favorites::decode(f.bytes()).unwrap();assert_eq!(restored.count(),2);assert_eq!(restored.find(b"https://example.com/b"),Some(1));
        assert_eq!(restored.add(b"https://example.com/a",b"Duplicate"),Err(Error::Invalid));
        restored.remove(0).unwrap();assert_eq!(restored.get(0),Some((b"https://example.com/b".as_slice(),b"Beta".as_slice())));
        for end in 1..f.bytes().len() {if end!=4 && end!=4+3+21+5 {assert!(Favorites::decode(&f.bytes()[..end]).is_err());}}
        assert_eq!(restored.add(b"file:///etc/passwd",b"No"),Err(Error::Invalid));
        restored.remove(0).unwrap();assert_eq!(Favorites::decode(restored.bytes()).unwrap().count(),0);
    }
    // ------------------------=
    // FUNC: capacity_failure_preserves_existing_records
    // DESC: Checks both record and byte budgets without partial updates.
    // ------------------=
    #[test]
    fn capacity_failure_preserves_existing_records() {
        let mut f=Favorites::new();for n in 0..CAPACITY {let mut url=*b"https://example.com/00";url[19]=b'A'+n as u8;f.add(&url,b"Page").unwrap();}
        let before=f;assert_eq!(f.add(b"https://overflow.test",b"Overflow"),Err(Error::Full));assert_eq!(f.bytes(),before.bytes());
        let mut f=Favorites::new();let mut url=[b'a';2048];url[..8].copy_from_slice(b"https://");
        for n in 0..7 {url[8]=b'A'+n;f.add(&url,b"Long URL").unwrap();}
        url[8]=b'Z';let before=f;assert_eq!(f.add(&url,b"Too long"),Err(Error::Full));assert_eq!(before.bytes(),f.bytes());
    }
}
