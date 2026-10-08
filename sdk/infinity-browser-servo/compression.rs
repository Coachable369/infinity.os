//! Bounded content decoding shared by native document and subresource loads.
use std::{io::Read, string::String, vec::Vec};
// ------------------------=
// FUNC: gzip
// DESC: Accepts only negotiated gzip or identity, rejecting ambiguous encoding chains.
// ------------------=
pub fn gzip(headers:&[(String,String)])->Result<bool,()> {
    let mut values=headers.iter().filter(|(name,_)|name.eq_ignore_ascii_case("content-encoding"));
    let Some((_,value))=values.next() else{return Ok(false);};
    if values.next().is_some() {return Err(());}
    if value.trim().eq_ignore_ascii_case("gzip") {Ok(true)}
    else if value.trim().eq_ignore_ascii_case("identity") {Ok(false)} else {Err(())}
}
// ------------------------=
// FUNC: decode
// DESC: Validates gzip checksums and bounds expansion before exposing decoded bytes to Servo.
// ------------------=
pub fn decode(bytes:&[u8],limit:usize)->Result<Vec<u8>,()> {
    let mut out=Vec::with_capacity(bytes.len().min(limit));
    flate2::read::MultiGzDecoder::new(bytes).take(limit as u64+1).read_to_end(&mut out).map_err(|_|())?;
    if out.len()>limit {return Err(());}Ok(out)
}
// ------------------------=
// FUNC: remove_wire_encoding
// DESC: Prevents decoded bytes from being decompressed twice or retaining compressed length metadata.
// ------------------=
pub fn remove_wire_encoding(headers:&mut Vec<(String,String)>) {
    headers.retain(|(name,_)|!name.eq_ignore_ascii_case("content-encoding") && !name.eq_ignore_ascii_case("content-length"));
}
