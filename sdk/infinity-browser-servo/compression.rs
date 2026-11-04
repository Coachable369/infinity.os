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


struct BoundedOutput { bytes: Vec<u8>, total: usize, limit: usize }
impl std::io::Write for BoundedOutput {
    // ------------------------=
    // FUNC: write
    // DESC: Bounds cumulative decompressed bytes even after earlier chunks have been delivered.
    // ------------------=
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > self.limit.saturating_sub(self.total) {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "gzip expansion limit"));
        }
        self.bytes.extend_from_slice(bytes); self.total += bytes.len(); Ok(bytes.len())
    }
    // ------------------------=
    // FUNC: flush
    // DESC: Keeps decoded output in its bounded memory sink without external IO.
    // ------------------=
    fn flush(&mut self) -> std::io::Result<()> { Ok(()) }
}

pub struct StreamingGzip { decoder: flate2::write::MultiGzDecoder<BoundedOutput> }
impl StreamingGzip {
    // ------------------------=
    // FUNC: new
    // DESC: Creates a bounded incremental gzip decoder for early HTML and dependency discovery.
    // ------------------=
    pub fn new(limit: usize) -> Self {
        Self { decoder: flate2::write::MultiGzDecoder::new(BoundedOutput { bytes: Vec::new(), total: 0, limit }) }
    }
    // ------------------------=
    // FUNC: push
    // DESC: Decodes an authenticated transport chunk without waiting for the entire response body.
    // ------------------=
    pub fn push(&mut self, bytes: &[u8]) -> Result<Vec<u8>, ()> {
        use std::io::Write;
        self.decoder.write_all(bytes).map_err(|_| ())?;
        self.decoder.flush().map_err(|_| ())?;
        Ok(core::mem::take(&mut self.decoder.get_mut().bytes))
    }
    // ------------------------=
    // FUNC: finish
    // DESC: Requires a complete gzip trailer and checksum before publishing resource completion.
    // ------------------=
    pub fn finish(mut self) -> Result<Vec<u8>, ()> {
        self.decoder.try_finish().map_err(|_| ())?;
        Ok(core::mem::take(&mut self.decoder.get_mut().bytes))
    }
}
