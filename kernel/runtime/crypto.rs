//! Protected-key cryptographic mechanisms for node identity and mesh sessions.

use chacha20poly1305::aead::{AeadInPlace, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce, Tag};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use hkdf::Hkdf;
use sha2::{Digest, Sha256};
use x25519_dalek::{PublicKey as AgreementPublicKey, StaticSecret};
use zeroize::Zeroize;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct KeyRef(pub u64);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CryptoError {
    EntropyUnavailable,
    KeyUnavailable,
    InvalidSignature,
    InvalidCiphertext,
    OutputTooSmall,
}

pub struct NodeCrypto {
    identity: Option<SigningKey>,
    key_ref: KeyRef,
    generation: u64,
}

impl NodeCrypto {
    // ------------------------=
    // FUNC: new
    // DESC: Creates an empty protected-key service that has no authority until seeded.
    // ------------------=
    pub const fn new() -> Self {
        Self { identity: None, key_ref: KeyRef(0), generation: 0 }
    }

    // ------------------------=
    // FUNC: initialize
    // DESC: Derives a node signing key from firmware entropy and retains only a non-exportable key reference.
    // ------------------=
    pub fn initialize(&mut self, entropy: &[u8; 32], valid: bool) -> Result<KeyRef, CryptoError> {
        if !valid || entropy.iter().all(|value| *value == 0) {
            return Err(CryptoError::EntropyUnavailable);
        }
        let mut hasher = Sha256::new();
        hasher.update(b"InfinityOS Node Identity v1");
        hasher.update(entropy);
        let mut seed: [u8; 32] = hasher.finalize().into();
        let key_ref = self.initialize_seed(&seed)?;
        seed.zeroize();
        Ok(key_ref)
    }

    // ------------------------=
    // FUNC: initialize_seed
    // DESC: Restores a protected signing key from the Crypto Service's typed persistent key envelope.
    // ------------------=
    pub(crate) fn initialize_seed(&mut self, seed: &[u8; 32]) -> Result<KeyRef, CryptoError> {
        if seed.iter().all(|value| *value == 0) {
            return Err(CryptoError::EntropyUnavailable);
        }
        self.generation = self.generation.wrapping_add(1).max(1);
        self.key_ref = KeyRef(self.generation);
        self.identity = Some(SigningKey::from_bytes(seed));
        Ok(self.key_ref)
    }

    // ------------------------=
    // FUNC: key_ref
    // DESC: Returns the opaque identity-key reference without exposing private key material.
    // ------------------=
    pub fn key_ref(&self) -> Result<KeyRef, CryptoError> {
        self.identity.as_ref().map(|_| self.key_ref).ok_or(CryptoError::KeyUnavailable)
    }

    // ------------------------=
    // FUNC: public_identity
    // DESC: Returns the public Ed25519 identity key for discovery and verification.
    // ------------------=
    pub fn public_identity(&self) -> Result<[u8; 32], CryptoError> {
        self.identity.as_ref().map(|key| key.verifying_key().to_bytes()).ok_or(CryptoError::KeyUnavailable)
    }

    // ------------------------=
    // FUNC: sign
    // DESC: Signs a bounded protocol transcript with the protected node identity key.
    // ------------------=
    pub fn sign(&self, key_ref: KeyRef, message: &[u8]) -> Result<[u8; 64], CryptoError> {
        if key_ref != self.key_ref { return Err(CryptoError::KeyUnavailable); }
        self.identity.as_ref().map(|key| key.sign(message).to_bytes()).ok_or(CryptoError::KeyUnavailable)
    }

    // ------------------------=
    // FUNC: verify
    // DESC: Verifies an Ed25519 signature against an explicitly supplied node public key.
    // ------------------=
    pub fn verify(public: &[u8; 32], message: &[u8], signature: &[u8; 64]) -> Result<(), CryptoError> {
        let key = VerifyingKey::from_bytes(public).map_err(|_| CryptoError::InvalidSignature)?;
        key.verify(message, &Signature::from_bytes(signature)).map_err(|_| CryptoError::InvalidSignature)
    }

    // ------------------------=
    // FUNC: agreement_keypair
    // DESC: Derives a boot-scoped X25519 key pair from fresh caller-supplied entropy.
    // ------------------=
    pub fn agreement_keypair(entropy: &[u8; 32]) -> ([u8; 32], [u8; 32]) {
        let secret = StaticSecret::from(*entropy);
        let public = AgreementPublicKey::from(&secret);
        (secret.to_bytes(), public.to_bytes())
    }

    // ------------------------=
    // FUNC: derive_session_key
    // DESC: Performs X25519 agreement and HKDF-SHA256 transcript binding for one peer session.
    // ------------------=
    pub fn derive_session_key(secret: &[u8; 32], peer_public: &[u8; 32], transcript: &[u8]) -> [u8; 32] {
        let shared = StaticSecret::from(*secret).diffie_hellman(&AgreementPublicKey::from(*peer_public));
        let hkdf = Hkdf::<Sha256>::new(Some(transcript), shared.as_bytes());
        let mut output = [0u8; 32];
        let _ = hkdf.expand(b"InfinityOS Mesh Session v1", &mut output);
        output
    }

    // ------------------------=
    // FUNC: seal
    // DESC: Authenticates and encrypts a bounded payload in place using ChaCha20-Poly1305.
    // ------------------=
    pub fn seal(key: &[u8; 32], nonce: &[u8; 12], aad: &[u8], payload: &mut [u8]) -> Result<[u8; 16], CryptoError> {
        let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
        cipher.encrypt_in_place_detached(Nonce::from_slice(nonce), aad, payload)
            .map(|tag| tag.into()).map_err(|_| CryptoError::InvalidCiphertext)
    }

    // ------------------------=
    // FUNC: open
    // DESC: Authenticates and decrypts a bounded payload in place and rejects tampering.
    // ------------------=
    pub fn open(key: &[u8; 32], nonce: &[u8; 12], aad: &[u8], payload: &mut [u8], tag: &[u8; 16]) -> Result<(), CryptoError> {
        let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
        cipher.decrypt_in_place_detached(Nonce::from_slice(nonce), aad, payload, Tag::from_slice(tag))
            .map_err(|_| CryptoError::InvalidCiphertext)
    }
}
