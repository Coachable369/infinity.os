//! Build-time behavioral check for a conservative native-system-trust alias.
//! This executable is never linked into the OS or used as its TLS verifier.
use rustls_pki_types::{pem::PemObject, CertificateDer};
use sha2::{Digest, Sha256};

// ------------------------=
// FUNC: main
// DESC: Decodes the exact candidate PEM with the production certificate parser and proves that native trust cannot exceed the requested anchor set.
// ------------------=
fn main() {
    let path = std::env::args_os().nth(1).expect("candidate PEM path required");
    let bytes = std::fs::read(path).expect("candidate PEM readable");
    assert!(bytes.len() <= 512 * 1024);
    let certificates: Vec<_> = CertificateDer::pem_slice_iter(&bytes)
        .collect::<Result<_, _>>().expect("valid PEM certificates");
    assert!(!certificates.is_empty());
    let anchors: Vec<_> = certificates.iter().map(|certificate|
        webpki::anchor_from_trusted_cert(certificate).expect("valid X.509 trust anchor")
    ).collect();
    let native = infinity_http::tls::system_roots();
    assert!(!native.is_empty());
    let mut digest = Sha256::new();
    let mut excluded = Vec::new();
    for (index, anchor) in native.iter().enumerate() {
        for bytes in [Some(anchor.subject.as_ref()), Some(anchor.subject_public_key_info.as_ref()),
                      anchor.name_constraints.as_ref().map(|value| value.as_ref())] {
            digest.update(bytes.map_or(u64::MAX, |value| value.len() as u64).to_le_bytes());
            if let Some(bytes) = bytes { digest.update(bytes); }
        }
        if !anchors.contains(anchor) { excluded.push(index); }
    }
    let fingerprint: [u8; 32] = digest.finalize().into();
    let pem: [u8; 32] = Sha256::digest(&bytes).into();
    assert_eq!(pem, infinity_http::trust::CERTIFI_PEM_SHA256);
    let selected = infinity_http::trust::certifi_roots(native).expect("pinned native set");
    assert!(!selected.is_empty());
    for anchor in &selected { assert!(anchors.contains(anchor)); }
    assert_eq!(selected.len(), native.len() - excluded.len());
    let mut reordered = native.to_vec(); reordered.swap(0, 1);
    assert!(infinity_http::trust::certifi_roots(&reordered).is_err());
    let mut altered = native.to_vec(); altered[0].name_constraints = Some((&b"altered"[..]).into());
    assert!(infinity_http::trust::certifi_roots(&altered).is_err());
    assert_eq!(infinity_http::tls::system_roots(), native);
    println!("{{\"native_anchors\":{},\"requested_anchors\":{},\"selected_anchors\":{},\"exact_constraints\":true,\"native_subset\":true,\"excluded\":{:?},\"native_digest\":{:?},\"pem_digest\":{:?}}}",
        native.len(), anchors.len(), selected.len(), excluded, fingerprint, pem);
}
