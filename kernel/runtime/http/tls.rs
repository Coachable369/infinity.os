//! TLS 1.3 certificate verification without a heap or a permissive verifier.
use embedded_tls::{
    Aes128GcmSha256, CertificateEntryRef, CertificateRef, CertificateVerifyRef, CryptoProvider,
    TlsError, TlsVerifier,
};
use p256::ecdsa::signature::Verifier;
use rustls_pki_types::{
    alg_id, AlgorithmIdentifier, CertificateDer, InvalidSignature, ServerName,
    SignatureVerificationAlgorithm, TrustAnchor, UnixTime,
};
use sha2::{Digest, Sha256};

#[derive(Debug)]
struct Ecdsa(bool);
impl SignatureVerificationAlgorithm for Ecdsa {
    // ------------------------=
    // FUNC: verify_signature
    // DESC: Delegates ECDSA signature verification to RustCrypto with the exact certificate algorithm binding.
    // ------------------=
    fn verify_signature(
        &self,
        key: &[u8],
        message: &[u8],
        signature: &[u8],
    ) -> Result<(), InvalidSignature> {
        if self.0 {
            let key =
                p384::ecdsa::VerifyingKey::from_sec1_bytes(key).map_err(|_| InvalidSignature)?;
            let signature =
                p384::ecdsa::Signature::from_der(signature).map_err(|_| InvalidSignature)?;
            key.verify(message, &signature)
                .map_err(|_| InvalidSignature)
        } else {
            let key =
                p256::ecdsa::VerifyingKey::from_sec1_bytes(key).map_err(|_| InvalidSignature)?;
            let signature =
                p256::ecdsa::Signature::from_der(signature).map_err(|_| InvalidSignature)?;
            key.verify(message, &signature)
                .map_err(|_| InvalidSignature)
        }
    }
    // ------------------------=
    // FUNC: public_key_alg_id
    // DESC: Restricts accepted public keys to the selected NIST curve.
    // ------------------=
    fn public_key_alg_id(&self) -> AlgorithmIdentifier {
        if self.0 {
            alg_id::ECDSA_P384
        } else {
            alg_id::ECDSA_P256
        }
    }
    // ------------------------=
    // FUNC: signature_alg_id
    // DESC: Binds P256 to SHA256 and P384 to SHA384 without algorithm substitution.
    // ------------------=
    fn signature_alg_id(&self) -> AlgorithmIdentifier {
        if self.0 {
            alg_id::ECDSA_SHA384
        } else {
            alg_id::ECDSA_SHA256
        }
    }
}
static P256: Ecdsa = Ecdsa(false);
static P384: Ecdsa = Ecdsa(true);
static ALGORITHMS: [&'static dyn SignatureVerificationAlgorithm; 2] = [&P256, &P384];

// ------------------------=
// FUNC: system_roots
// DESC: Exposes versioned Mozilla trust anchors for the native HTTPS service to use in either kernel configuration.
// ------------------=
pub fn system_roots() -> &'static [TrustAnchor<'static>] {
    webpki_roots::TLS_SERVER_ROOTS
}

/// Roots and trusted wall-clock time must come from the installed system, never the peer.
pub struct CertificateVerifier<'a> {
    roots: &'a [TrustAnchor<'a>],
    host: &'a str,
    now: UnixTime,
    leaf: heapless::Vec<u8, 8192>,
    transcript: Option<Sha256>,
}
impl<'a> CertificateVerifier<'a> {
    // ------------------------=
    // FUNC: new
    // DESC: Requires explicit trust anchors, hostname and usable wall-clock time before beginning authentication.
    // ------------------=
    pub fn new(
        roots: &'a [TrustAnchor<'a>],
        host: &'a str,
        unix_seconds: u64,
    ) -> Result<Self, TlsError> {
        if roots.is_empty() || unix_seconds == 0 || ServerName::try_from(host).is_err() {
            return Err(TlsError::InvalidCertificate);
        }
        Ok(Self {
            roots,
            host,
            now: UnixTime::since_unix_epoch(core::time::Duration::from_secs(unix_seconds)),
            leaf: heapless::Vec::new(),
            transcript: None,
        })
    }
    // ------------------------=
    // FUNC: verify_chain
    // DESC: Checks the full chain, validity dates, CA constraints, server usage and expected subject name before retaining the leaf.
    // ------------------=
    pub fn verify_chain(
        &mut self,
        leaf: &[u8],
        intermediates: &[CertificateDer<'_>],
    ) -> Result<(), TlsError> {
        self.leaf.clear();
        self.transcript = None;
        let der = CertificateDer::from(leaf);
        let cert =
            webpki::EndEntityCert::try_from(&der).map_err(|_| TlsError::InvalidCertificate)?;
        cert.verify_for_usage(
            &ALGORITHMS,
            self.roots,
            intermediates,
            self.now,
            webpki::KeyUsage::server_auth(),
            None,
            None,
        )
        .map_err(|_| TlsError::InvalidCertificate)?;
        let name = ServerName::try_from(self.host).map_err(|_| TlsError::InvalidCertificate)?;
        cert.verify_is_valid_for_subject_name(&name)
            .map_err(|_| TlsError::InvalidCertificate)?;
        self.leaf
            .extend_from_slice(leaf)
            .map_err(|_| TlsError::InsufficientSpace)
    }
}
impl TlsVerifier<Aes128GcmSha256> for CertificateVerifier<'_> {
    // ------------------------=
    // FUNC: set_hostname_verification
    // DESC: Prevents a TLS configuration from silently changing the authorized destination name.
    // ------------------=
    fn set_hostname_verification(&mut self, host: &str) -> Result<(), TlsError> {
        if host == self.host {
            Ok(())
        } else {
            Err(TlsError::InvalidCertificate)
        }
    }
    // ------------------------=
    // FUNC: verify_certificate
    // DESC: Validates a bounded server certificate chain and captures its handshake transcript.
    // ------------------=
    fn verify_certificate(
        &mut self,
        transcript: &Sha256,
        cert: CertificateRef<'_>,
    ) -> Result<(), TlsError> {
        let mut chain: heapless::Vec<CertificateDer<'_>, 8> = heapless::Vec::new();
        for entry in &cert.entries {
            let CertificateEntryRef::X509(bytes) = entry else {
                return Err(TlsError::InvalidCertificate);
            };
            chain
                .push(CertificateDer::from(*bytes))
                .map_err(|_| TlsError::InsufficientSpace)?;
        }
        let leaf = chain.first().ok_or(TlsError::InvalidCertificate)?;
        self.verify_chain(leaf.as_ref(), &chain[1..])?;
        self.transcript = Some(transcript.clone());
        Ok(())
    }
    // ------------------------=
    // FUNC: verify_signature
    // DESC: Authenticates possession of the validated server key over the TLS 1.3 handshake transcript.
    // ------------------=
    fn verify_signature(&mut self, signature: CertificateVerifyRef<'_>) -> Result<(), TlsError> {
        let hash = self
            .transcript
            .take()
            .ok_or(TlsError::InvalidHandshake)?
            .finalize();
        let mut message = heapless::Vec::<u8, 160>::new();
        message
            .extend_from_slice(&[0x20; 64])
            .map_err(|_| TlsError::InsufficientSpace)?;
        message
            .extend_from_slice(b"TLS 1.3, server CertificateVerify\0")
            .map_err(|_| TlsError::InsufficientSpace)?;
        message
            .extend_from_slice(&hash)
            .map_err(|_| TlsError::InsufficientSpace)?;
        let algorithm: &dyn SignatureVerificationAlgorithm =
            match signature.signature_scheme.as_u16() {
                0x0403 => &P256,
                0x0503 => &P384,
                _ => return Err(TlsError::InvalidSignatureScheme),
            };
        let der = CertificateDer::from(self.leaf.as_slice());
        webpki::EndEntityCert::try_from(&der)
            .map_err(|_| TlsError::InvalidCertificate)?
            .verify_signature(algorithm, &message, signature.signature)
            .map_err(|_| TlsError::InvalidSignature)
    }
}

pub(crate) struct Provider<'a, R> {
    pub rng: R,
    pub verifier: CertificateVerifier<'a>,
}
impl<R: rand_core::CryptoRngCore> CryptoProvider for Provider<'_, R> {
    type CipherSuite = Aes128GcmSha256;
    type Signature = p256::ecdsa::DerSignature;
    // ------------------------=
    // FUNC: rng
    // DESC: Supplies the caller's cryptographically secure generator without any deterministic fallback.
    // ------------------=
    fn rng(&mut self) -> impl rand_core::CryptoRngCore {
        &mut self.rng
    }
    // ------------------------=
    // FUNC: verifier
    // DESC: Always supplies strict certificate validation; there is no insecure mode.
    // ------------------=
    fn verifier(&mut self) -> Result<&mut impl TlsVerifier<Aes128GcmSha256>, TlsError> {
        Ok(&mut self.verifier)
    }
}
