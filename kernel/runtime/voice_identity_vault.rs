//! Account and model-bound encrypted speaker enrollment, never raw microphone recordings.
use super::{crypto::NodeCrypto,identity::StableId};
use zeroize::Zeroizing;
pub const PAYLOAD:usize=3*256*4;
pub const HEADER:usize=80;
pub const BYTES:usize=HEADER+PAYLOAD+16;
// ------------------------=
// FUNC: seal
// DESC: Encrypts a validated speaker template with a fresh nonce and authenticated account/model metadata.
// ------------------=
pub fn seal(key:&[u8;32],account:StableId,model:&[u8;32],nonce:[u8;12],payload:&[u8;PAYLOAD])->Option<Zeroizing<[u8;BYTES]>>{
    if account.0==[0;16]||*model==[0;32]||nonce==[0;12]{return None;}
    let mut record=Zeroizing::new([0u8;BYTES]);
    record[..8].copy_from_slice(b"INFVOICE");record[8..24].copy_from_slice(&account.0);record[24..56].copy_from_slice(model);
    record[56..68].copy_from_slice(&nonce);record[68..72].copy_from_slice(&1u32.to_le_bytes());
    record[HEADER..BYTES-16].copy_from_slice(payload);
    let(aad,body)=record.split_at_mut(HEADER);
    let tag=NodeCrypto::seal(key,&nonce,aad,&mut body[..PAYLOAD]).ok()?;body[PAYLOAD..].copy_from_slice(&tag);Some(record)
}
// ------------------------=
// FUNC: open
// DESC: Rejects account substitution, model changes, version mismatches and ciphertext tampering before exposing private embeddings.
// ------------------=
pub fn open(key:&[u8;32],account:StableId,model:&[u8;32],record:&[u8;BYTES])->Option<Zeroizing<[u8;PAYLOAD]>>{
    if &record[..8]!=b"INFVOICE"||record[8..24]!=account.0||record[24..56]!=*model||record[68..72]!=1u32.to_le_bytes(){return None;}
    let nonce=record[56..68].try_into().ok()?;let tag=record[BYTES-16..].try_into().ok()?;
    let mut plain=Zeroizing::new([0u8;PAYLOAD]);plain.copy_from_slice(&record[HEADER..BYTES-16]);
    NodeCrypto::open(key,&nonce,&record[..HEADER],&mut *plain,&tag).ok()?;Some(plain)
}
