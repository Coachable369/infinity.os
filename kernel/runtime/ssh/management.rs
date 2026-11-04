//! Bounded local provisioning syntax; never reachable as a remote SSH command.
use base64ct::{Base64, Base64Unpadded, Encoding};
use sha2::{Digest, Sha256};
use super::{Configuration, Failure};

#[derive(Debug, PartialEq, Eq)]
pub enum Command<'a> {
    Status,
    Configure(Configuration),
    Key { user: &'a [u8], key: [u8; 32], revoke: bool },
}
// ------------------------=
// FUNC: parse
// DESC: Parses exact local management operations with bounded Ed25519 public-key material and validated quotas.
// ------------------=
pub fn parse(command: &[u8]) -> Result<Command<'_>, Failure> {
    let mut words = command.split(u8::is_ascii_whitespace).filter(|word| !word.is_empty());
    if words.next() != Some(b"ssh".as_slice()) { return Err(Failure::Configuration); }
    let operation = match words.next() {
        Some(b"status") => Command::Status,
        Some(b"disable") => Command::Configure(Configuration::default()),
        Some(b"enable") => {
            let port = words.next().map(number).transpose()?.unwrap_or(22);
            let max_connections = words.next().map(number).transpose()?.unwrap_or(2);
            let max_connections = u8::try_from(max_connections).map_err(|_| Failure::Configuration)?;
            Command::Configure(Configuration { enabled: true, port, max_connections }.validate()?)
        }
        Some(b"key") => {
            let revoke = match words.next() { Some(b"add") => false, Some(b"revoke") => true, _ => return Err(Failure::Configuration) };
            let user = words.next().ok_or(Failure::Configuration)?;
            if user.len() > 48 || words.next() != Some(b"ssh-ed25519".as_slice()) { return Err(Failure::Configuration); }
            let encoded = words.next().ok_or(Failure::Configuration)?;
            let text = core::str::from_utf8(encoded).map_err(|_| Failure::Configuration)?;
            let mut blob = [0; 51];
            if Base64::decode(text, &mut blob).map_err(|_| Failure::Configuration)?.len() != blob.len()
                || blob[..19] != *b"\0\0\0\x0bssh-ed25519\0\0\0\x20" { return Err(Failure::Configuration); }
            let key = blob[19..].try_into().map_err(|_| Failure::Configuration)?;
            Command::Key { user, key, revoke }
        }
        _ => return Err(Failure::Configuration),
    };
    if words.next().is_some() { return Err(Failure::Configuration); }
    Ok(operation)
}
// ------------------------=
// FUNC: number
// DESC: Rejects malformed and overflowing decimal configuration values.
// ------------------=
fn number(bytes: &[u8]) -> Result<u16, Failure> {
    let mut value = 0u16;
    if bytes.is_empty() { return Err(Failure::Configuration); }
    for byte in bytes {
        if !byte.is_ascii_digit() { return Err(Failure::Configuration); }
        value = value.checked_mul(10).and_then(|v| v.checked_add((byte - b'0') as u16)).ok_or(Failure::Configuration)?;
    }
    Ok(value)
}
// ------------------------=
// FUNC: fingerprint
// DESC: Formats the standard OpenSSH SHA256 host fingerprint from the public wire key only.
// ------------------=
pub fn fingerprint(public: &[u8; 32]) -> [u8; 50] {
    let mut blob = [0; 51];
    blob[..19].copy_from_slice(b"\0\0\0\x0bssh-ed25519\0\0\0\x20");
    blob[19..].copy_from_slice(public);
    let digest = Sha256::digest(blob);
    let mut out = [0; 50];
    out[..7].copy_from_slice(b"SHA256:");
    let _ = Base64Unpadded::encode(&digest, &mut out[7..]);
    out
}
