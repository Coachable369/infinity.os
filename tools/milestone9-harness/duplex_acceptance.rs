//! HOST regression only: no claim of network negotiation or human verification.
use crate::{crypto::NodeCrypto, node::types::NodeError};

// ------------------------=
// FUNC: run
// DESC: Exercises independently numbered duplex sessions, directional replay, tampering, expiry and fresh reconnect.
// ------------------=
pub fn run() {
    let (mut a, mut b, aid, bid) = crate::paired_nodes();
    b.grant_remote(aid, 0xd002, 1, 1, 100, 19, 4).unwrap();
    let (sa, pa) = NodeCrypto::agreement_keypair(&[51; 32]);
    let (sb, pb) = NodeCrypto::agreement_keypair(&[67; 32]);
    let ah = a.open_session(bid, &sa, &pb, b"fresh-handshake-1", 20, 8).unwrap();
    let bh = b.open_session(aid, &sb, &pa, b"fresh-handshake-1", 20, 8).unwrap();
    assert_ne!(ah, bh);
    assert_eq!(a.sessions().iter().flatten().find(|s| s.id == ah).unwrap().protocol_reference, b.sessions().iter().flatten().find(|s| s.id == bh).unwrap().protocol_reference);
    let mut ab = [41; 32]; let (aseq, atag) = a.protect(ah, b"iop", &mut ab, 21).unwrap();
    let old_ab = ab;
    let mut wrong = ab;
    assert!(a.unprotect(ah, aseq, b"iop", &mut wrong, &atag, 21).is_err());
    let mut bad_tag = atag; bad_tag[0] ^= 1;
    assert!(b.unprotect(bh, aseq, b"iop", &mut wrong, &bad_tag, 21).is_err());
    b.unprotect(bh, aseq, b"iop", &mut ab, &atag, 21).unwrap();
    assert_eq!(ab, [41; 32]);
    let mut replay = old_ab;
    assert_eq!(b.unprotect(bh, aseq, b"iop", &mut replay, &atag, 21), Err(NodeError::ReplayDetected));
    let mut ba = [41; 32]; let (bseq, btag) = b.protect(bh, b"iop", &mut ba, 21).unwrap();
    assert_eq!(aseq, bseq); assert_ne!(ba, old_ab);
    let old_ba = ba;
    a.unprotect(ah, bseq, b"iop", &mut ba, &btag, 21).unwrap(); assert_eq!(ba, [41; 32]);
    assert_eq!(a.unprotect(ah, bseq, b"iop", &mut ba, &btag, 21), Err(NodeError::ReplayDetected));
    // B independently initiates; A replies. Sequence spaces remain direction-local.
    let mut request = [7; 32]; let (seq, tag) = b.protect(bh, b"iop", &mut request, 22).unwrap();
    assert_eq!(seq, 2); a.unprotect(ah, seq, b"iop", &mut request, &tag, 22).unwrap();
    let mut reply = [8; 32]; let (seq, tag) = a.protect(ah, b"iop", &mut reply, 22).unwrap();
    assert_eq!(seq, 2); b.unprotect(bh, seq, b"iop", &mut reply, &tag, 22).unwrap();
    assert_eq!(a.protect(u64::MAX, b"iop", &mut reply, 22), Err(NodeError::SessionNotFound));
    assert_eq!(a.protect(ah, b"iop", &mut reply, 4000), Err(NodeError::SessionExpired));
    a.close_session(ah, 23, 9).unwrap(); b.close_session(bh, 23, 9).unwrap();
    assert_eq!(a.open_session(bid, &sa, &pb, b"fresh-handshake-1", 24, 10), Err(NodeError::ReplayDetected));
    let (sa2, pa2) = NodeCrypto::agreement_keypair(&[52; 32]);
    let (sb2, pb2) = NodeCrypto::agreement_keypair(&[68; 32]);
    let ah2 = a.open_session(bid, &sa2, &pb2, b"fresh-handshake-2", 24, 10).unwrap();
    let bh2 = b.open_session(aid, &sb2, &pa2, b"fresh-handshake-2", 24, 10).unwrap();
    let mut old = old_ab; assert!(b.unprotect(bh2, aseq, b"iop", &mut old, &atag, 25).is_err());
    let mut old = old_ba; assert!(a.unprotect(ah2, bseq, b"iop", &mut old, &btag, 25).is_err());
    let mut fresh = [9; 32]; let (seq, tag) = a.protect(ah2, b"iop", &mut fresh, 25).unwrap();
    assert_eq!(seq, 1); b.unprotect(bh2, seq, b"iop", &mut fresh, &tag, 25).unwrap(); assert_eq!(fresh, [9; 32]);
    let mut fresh = [10; 32]; let (seq, tag) = b.protect(bh2, b"iop", &mut fresh, 25).unwrap();
    assert_eq!(seq, 1); a.unprotect(ah2, seq, b"iop", &mut fresh, &tag, 25).unwrap(); assert_eq!(fresh, [10; 32]);
    assert!(NodeCrypto::derive_duplex_keys(&sa, &[0; 32], &aid.0, &bid.0, b"fresh").is_err());
}
