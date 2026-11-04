//! Owner-held Ed25519 credentials in OpenSSH's portable private/public formats.
use base64ct::{Base64, Encoding};
use zeroize::Zeroizing;
use crate::{ed25519_dalek::SigningKey, Failure};

/// No Debug/Clone: private bytes are erased when the provisioning transaction ends.
pub struct UserKeyPair {
    private: Zeroizing<[u8; 512]>,
    private_len: usize,
    public_line: [u8; 96],
    public_len: usize,
    public: [u8; 32],
}
impl UserKeyPair {
    // ------------------------=
    // FUNC: generate
    // DESC: Generates a fresh Ed25519 key using fallible native entropy and encodes standard OpenSSH files without heap allocation.
    // ------------------=
    pub fn generate(entropy: impl FnOnce(&mut [u8]) -> bool) -> Result<Self, Failure> {
        let mut random = Zeroizing::new([0; 36]);
        if !entropy(&mut *random) { return Err(Failure::EntropyUnavailable); }
        let seed = Zeroizing::new(<[u8; 32]>::try_from(&random[..32]).map_err(|_| Failure::EntropyUnavailable)?);
        let key = SigningKey::from_bytes(&seed);
        let public = key.verifying_key().to_bytes();
        let mut blob = [0; 51];
        blob[..19].copy_from_slice(b"\0\0\0\x0bssh-ed25519\0\0\0\x20");
        blob[19..].copy_from_slice(&public);
        let mut binary = Zeroizing::new([0; 234]);
        let mut at = 0;
        append(&mut *binary, &mut at, b"openssh-key-v1\0");
        string(&mut *binary, &mut at, b"none");
        string(&mut *binary, &mut at, b"none");
        string(&mut *binary, &mut at, b"");
        append(&mut *binary, &mut at, &1u32.to_be_bytes());
        string(&mut *binary, &mut at, &blob);
        append(&mut *binary, &mut at, &136u32.to_be_bytes());
        append(&mut *binary, &mut at, &random[32..]);
        append(&mut *binary, &mut at, &random[32..]);
        string(&mut *binary, &mut at, b"ssh-ed25519");
        string(&mut *binary, &mut at, &public);
        append(&mut *binary, &mut at, &64u32.to_be_bytes());
        append(&mut *binary, &mut at, &*seed);
        append(&mut *binary, &mut at, &public);
        string(&mut *binary, &mut at, b"");
        append(&mut *binary, &mut at, &[1, 2, 3, 4, 5]);
        let mut encoded = Zeroizing::new([0; 312]);
        let text = Base64::encode(&binary[..at], &mut *encoded).map_err(|_| Failure::Configuration)?;
        let mut result = Self { private: Zeroizing::new([0; 512]), private_len: 0,
            public_line: [0; 96], public_len: 0, public };
        append(&mut *result.private, &mut result.private_len, b"-----BEGIN OPENSSH PRIVATE KEY-----\n");
        for line in text.as_bytes().chunks(70) {
            append(&mut *result.private, &mut result.private_len, line);
            append(&mut *result.private, &mut result.private_len, b"\n");
        }
        append(&mut *result.private, &mut result.private_len, b"-----END OPENSSH PRIVATE KEY-----\n");
        append(&mut result.public_line, &mut result.public_len, b"ssh-ed25519 ");
        let length = Base64::encode(&blob, &mut result.public_line[result.public_len..])
            .map_err(|_| Failure::Configuration)?.len();
        result.public_len += length;
        append(&mut result.public_line, &mut result.public_len, b"\n");
        Ok(result)
    }
    // ------------------------=
    // FUNC: private_file
    // DESC: Borrows the PEM only for owner-private storage or explicit owner export.
    // ------------------=
    pub fn private_file(&self) -> &[u8] { &self.private[..self.private_len] }
    // ------------------------=
    // FUNC: public_file
    // DESC: Returns the portable OpenSSH public-key line.
    // ------------------=
    pub fn public_file(&self) -> &[u8] { &self.public_line[..self.public_len] }
    // ------------------------=
    // FUNC: public_key
    // DESC: Supplies the exact generated public key for native credential registration.
    // ------------------=
    pub fn public_key(&self) -> [u8; 32] { self.public }
}
// ------------------------=
// FUNC: append
// DESC: Appends fixed-size format fields to statically sized encoding buffers.
// ------------------=
fn append(out: &mut [u8], at: &mut usize, value: &[u8]) {
    out[*at..*at + value.len()].copy_from_slice(value); *at += value.len();
}
// ------------------------=
// FUNC: string
// DESC: Encodes an SSH length-prefixed binary string.
// ------------------=
fn string(out: &mut [u8], at: &mut usize, value: &[u8]) {
    append(out, at, &(value.len() as u32).to_be_bytes()); append(out, at, value);
}
