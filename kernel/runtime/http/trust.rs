//! Explicit conservative trust contexts. Unknown root-set versions fail closed.
use rustls_pki_types::TrustAnchor;
use sha2::{Digest, Sha256};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Policy { System, Certifi20260617 }

pub const CERTIFI_PEM_SHA256: [u8; 32] = [187,199,233,192,29,117,81,187,138,21,155,93,237,217,137,184,238,60,225,5,175,245,34,182,142,177,176,27,248,84,202,176];
const NATIVE_SHA256: [u8; 32] = [178,233,131,64,44,54,207,79,255,122,189,144,241,187,119,121,7,83,201,237,242,166,198,82,221,118,202,107,128,211,220,138];
const EXCLUDED: [usize; 5] = [4,6,33,54,99];

// ------------------------=
// FUNC: fingerprint
// DESC: Binds trust selection to every anchor's ordered subject, public key and exact name constraints, including absent constraints.
// ------------------=
pub fn fingerprint(roots: &[TrustAnchor<'_>]) -> [u8; 32] {
    let mut digest = Sha256::new();
    for anchor in roots {
        for bytes in [Some(anchor.subject.as_ref()), Some(anchor.subject_public_key_info.as_ref()),
            anchor.name_constraints.as_ref().map(|value| value.as_ref())] {
            digest.update(bytes.map_or(u64::MAX, |value| value.len() as u64).to_le_bytes());
            if let Some(bytes) = bytes { digest.update(bytes); }
        }
    }
    digest.finalize().into()
}

// ------------------------=
// FUNC: certifi_roots
// DESC: Narrows an exactly pinned native root set to its proven certifi intersection; changed roots or constraints are rejected, never admitted implicitly.
// ------------------=
pub fn certifi_roots<'a>(roots: &[TrustAnchor<'a>]) -> Result<heapless::Vec<TrustAnchor<'a>, 128>, ()> {
    if roots.len() != 121 || fingerprint(roots) != NATIVE_SHA256 { return Err(()); }
    let mut selected = heapless::Vec::new();
    for (index, anchor) in roots.iter().enumerate() {
        if !EXCLUDED.contains(&index) { selected.push(anchor.clone()).map_err(|_| ())?; }
    }
    Ok(selected)
}
