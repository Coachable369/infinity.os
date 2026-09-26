//! Host behavioral test of the exact staged native WebPKI adapter.
//! This is not a guest TLS handshake or installed-browser test.
#![allow(unexpected_cfgs, dead_code)]

#[path = "../../build/servo-native-deps/rustls-platform-verifier-0.7.0/src/verification/others.rs"]
mod adapter;

#[derive(Debug)]
pub struct EkuError;
impl core::fmt::Display for EkuError {
    // ------------------------=
    // FUNC: fmt
    // DESC: Supplies the upstream verifier's structured EKU error support.
    // ------------------=
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("invalid EKU")
    }
}
impl std::error::Error for EkuError {}

// ------------------------=
// FUNC: log_server_cert
// DESC: Avoids logging certificate bytes in the test harness.
// ------------------=
fn log_server_cert(_: &rustls::pki_types::CertificateDer<'_>) {}

#[cfg(test)]
mod tests {
    use super::adapter::Verifier;
    use rustls::client::danger::ServerCertVerifier;
    use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
    use std::{sync::Arc, time::Duration};

    // ------------------------=
    // FUNC: verify
    // DESC: Validates an authenticated upstream real-world chain with native roots and explicit time/name.
    // ------------------=
    fn verify(name: &str, seconds: u64, corrupt: bool) -> Result<(), rustls::Error> {
        assert!(cfg!(infinity_certificate_test));
        let verifier = Verifier::new(Arc::new(rustls::crypto::ring::default_provider())).unwrap();
        let base = concat!(env!("CARGO_MANIFEST_DIR"), "/../../build/servo-native-deps/rustls-platform-verifier-0.7.0/src/tests/verification_real_world/");
        let mut chain: Vec<CertificateDer<'static>> = (1..=4)
            .map(|i| CertificateDer::from(std::fs::read(format!("{base}aws_amazon_com_valid_{i}.crt")).unwrap()))
            .collect();
        if corrupt { chain[0] = CertificateDer::from(vec![0; 64]); }
        verifier.verify_server_cert(&chain[0], &chain[1..], &ServerName::try_from(name).unwrap(), &[],
                                    UnixTime::since_unix_epoch(Duration::from_secs(seconds))).map(|_| ())
    }

    #[test]
    // ------------------------=
    // FUNC: accepts_public_chain
    // DESC: Accepts the recorded authentic public chain at its known valid date.
    // ------------------=
    fn accepts_public_chain() { assert!(verify("aws.amazon.com", 1_775_649_786, false).is_ok()); }

    #[test]
    // ------------------------=
    // FUNC: rejects_wrong_host
    // DESC: Rejects a valid chain presented for another website.
    // ------------------=
    fn rejects_wrong_host() { assert!(verify("example.org", 1_775_649_786, false).is_err()); }

    #[test]
    // ------------------------=
    // FUNC: rejects_expired_chain
    // DESC: Rejects the recorded chain after certificate expiry.
    // ------------------=
    fn rejects_expired_chain() { assert!(verify("aws.amazon.com", 2_000_000_000, false).is_err()); }

    #[test]
    // ------------------------=
    // FUNC: rejects_malformed_chain
    // DESC: Rejects corrupt certificate data without bypassing authentication.
    // ------------------=
    fn rejects_malformed_chain() { assert!(verify("aws.amazon.com", 1_775_649_786, true).is_err()); }
}
