#[path="../../sdk/infinity-browser-servo/compression.rs"] pub mod compression;
#[cfg(test)] mod tests {
    use super::compression::*;
    use std::io::Write;
    // ------------------------=
    // FUNC: encode
    // DESC: Creates actual gzip data for decoder boundary tests.
    // ------------------=
    fn encode(bytes:&[u8])->Vec<u8> {
        let mut out=flate2::write::GzEncoder::new(Vec::new(),flate2::Compression::default());
        out.write_all(bytes).unwrap();out.finish().unwrap()
    }
    #[test]
    // ------------------------=
    // FUNC: bounded_complete_decode
    // DESC: Verifies exact bytes, expansion bounds, truncated streams and CRC rejection.
    // ------------------=
    fn bounded_complete_decode() {
        let original=vec![b'x';100000];let packed=encode(&original);
        assert!(packed.len()<1000);
        assert_eq!(decode(&packed,original.len()).unwrap(),original);
        assert!(decode(&packed,original.len()-1).is_err());
        assert!(decode(&packed[..packed.len()-1],original.len()).is_err());
        let mut corrupt=packed.clone();let n=corrupt.len();corrupt[n-8]^=1;
        assert!(decode(&corrupt,original.len()).is_err());
        let both=[encode(b"first"),encode(b"second")].concat();
        assert_eq!(decode(&both,11).unwrap(),b"firstsecond");
    }
    #[test]
    // ------------------------=
    // FUNC: response_metadata
    // DESC: Preserves security fields while removing wire-only encoding metadata.
    // ------------------=
    fn response_metadata() {
        let mut h=vec![("Content-Encoding".into(),"gzip".into()),("Content-Length".into(),"123".into()),("Content-Security-Policy".into(),"default-src 'self'".into())];
        assert_eq!(gzip(&h),Ok(true));remove_wire_encoding(&mut h);
        assert_eq!(h.len(),1);assert_eq!(h[0].0,"Content-Security-Policy");
        assert_eq!(gzip(&h),Ok(false));
        h.push(("content-encoding".into(),"gzip, br".into()));assert!(gzip(&h).is_err());
        h.pop();h.push(("content-encoding".into(),"gzip".into()));h.push(("content-encoding".into(),"gzip".into()));
        assert!(gzip(&h).is_err());
    }
}
