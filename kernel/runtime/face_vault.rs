//! Local encrypted enrollment records. Records are not login credentials and
//! do not claim liveness. Access remains behind the password-authorized worker.
use super::{crypto::NodeCrypto,face_setup::wire};
use zeroize::Zeroizing;
pub const HEADER:usize=160;
pub const BYTES:usize=HEADER+wire::ENROLLMENT_BYTES+16;

// ------------------------=
// FUNC: seal
// DESC: Encrypts account-bound verified portraits and vectors with authenticated metadata and a fresh nonce; rejects malformed vectors.
// ------------------=
pub fn seal(key:&[u8;32],owner:[u8;16],control:&wire::Control,nonce:[u8;12],now:u64,
    payload:&[u8])->Option<Zeroizing<[u8;BYTES]>> {
    if payload.len()!=wire::ENROLLMENT_BYTES || owner==[0;16] || control.model==[0;32]
        || control.name_len==0 || control.name_len>64 || nonce==[0;12] {return None;}
    for sample in payload[..3*128*4].chunks_exact(128*4) {
        let mut norm=0f64;
        for bytes in sample.chunks_exact(4) {
            let value=f32::from_le_bytes(bytes.try_into().ok()?);
            if !value.is_finite() || value.abs()>1e6 {return None;}
            norm+=f64::from(value)*f64::from(value);
        }
        if norm<=1e-12 {return None;}
    }
    let mut record=Zeroizing::new([0;BYTES]);
    record[..8].copy_from_slice(b"INFFACE1");record[8..24].copy_from_slice(&owner);
    record[24..56].copy_from_slice(&control.model);record[56..120].copy_from_slice(&control.name);
    record[120]=control.name_len as u8;record[128..140].copy_from_slice(&nonce);
    record[140..148].copy_from_slice(&now.to_le_bytes());
    record[HEADER..BYTES-16].copy_from_slice(payload);
    let (aad,body)=record.split_at_mut(HEADER);
    let tag=NodeCrypto::seal(key,&nonce,aad,&mut body[..wire::ENROLLMENT_BYTES]).ok()?;
    body[wire::ENROLLMENT_BYTES..].copy_from_slice(&tag);Some(record)
}

// ------------------------=
// FUNC: open
// DESC: Authenticates owner, model and all metadata before returning decrypted views to an authorized caller; tampered buffers are wiped.
// ------------------=
pub fn open(key:&[u8;32],owner:[u8;16],model:[u8;32],record:&[u8])->Option<Zeroizing<[u8;BYTES]>> {
    if record.len()!=BYTES || &record[..8]!=b"INFFACE1" || record[8..24]!=owner
        || record[24..56]!=model || !(1..=64).contains(&record[120]) {return None;}
    let nonce: [u8;12]=record[128..140].try_into().ok()?;
    let tag: [u8;16]=record[BYTES-16..].try_into().ok()?;
    let mut plain=Zeroizing::new([0;BYTES]);plain.copy_from_slice(record);
    let (aad,body)=plain.split_at_mut(HEADER);
    NodeCrypto::open(key,&nonce,aad,&mut body[..wire::ENROLLMENT_BYTES],&tag).ok()?;
    Some(plain)
}
