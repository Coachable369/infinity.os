//! C-only decoder adapter; linked only into the isolated native worker.
unsafe extern "C" {
    fn infinity_face_decode(jpeg:*const u8,length:usize,rgb:*mut u8,rgb_length:usize,scratch:*mut u8,scratch_length:usize)->i32;
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Error { Bounds, Decode }
// ------------------------=
// FUNC: decode
// DESC: Safely bounds real JPEG and private scratch slices before entering the isolated C decoder.
// ------------------=
pub fn decode(encoded:&[u8],rgb:&mut [u8],scratch:&mut [u8])->Result<(),Error> {
    if encoded.len()<4 || encoded.len()>1_228_800 || rgb.len()!=crate::vision::RGB_BYTES || scratch.len()!=8*1024*1024 {
        return Err(Error::Bounds);
    }
    let result=unsafe {infinity_face_decode(encoded.as_ptr(),encoded.len(),rgb.as_mut_ptr(),rgb.len(),scratch.as_mut_ptr(),scratch.len())};
    if result==0 {Ok(())}else{Err(Error::Decode)}
}
