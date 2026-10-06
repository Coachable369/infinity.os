//! Complete-field paste admission shared by native editors and workplace tests.
// ------------------------=
// FUNC: paste_ascii
// DESC: Inserts a complete printable single line or leaves the field unchanged on unsafe text or overflow.
// ------------------=
pub fn paste_ascii<const N:usize>(bytes:&mut [u8;N],length:&mut usize,caret:&mut usize,text:&[u8])->bool {
    if *length>N || text.len()>N-*length || text.iter().any(|b|!(32..=126).contains(b)) {return false;}
    let at=(*caret).min(*length);bytes.copy_within(at..*length,at+text.len());
    bytes[at..at+text.len()].copy_from_slice(text);*length+=text.len();*caret=at+text.len();true
}
