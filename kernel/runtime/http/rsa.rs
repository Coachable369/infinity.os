//! Bounded RSA verification only: RFC 8017, using RustCrypto integer arithmetic.
//! No private-key operations, allocation, SHA1 or permissive padding parsing.
use crypto_bigint::{
    modular::runtime_mod::{DynResidue, DynResidueParams},
    Encoding, U4096, U64,
};
use rustls_pki_types::{
    alg_id, AlgorithmIdentifier, InvalidSignature, SignatureVerificationAlgorithm,
};
use sha2::{Digest, Sha256, Sha384, Sha512};

#[derive(Debug, Clone, Copy)]
pub(crate) struct Rsa(pub u8, pub bool);
pub(crate) static PKCS256: Rsa = Rsa(32, false);
pub(crate) static PKCS384: Rsa = Rsa(48, false);
pub(crate) static PKCS512: Rsa = Rsa(64, false);
pub(crate) static PSS256: Rsa = Rsa(32, true);
pub(crate) static PSS384: Rsa = Rsa(48, true);
pub(crate) static PSS512: Rsa = Rsa(64, true);

impl Rsa {
    // ------------------------=
    // FUNC: hash
    // DESC: Hashes bounded signature inputs with the algorithm bound by the certificate identifier.
    // ------------------=
    fn hash(self, input: &[u8]) -> [u8; 64] {
        let mut result = [0; 64];
        match self.0 {
            32 => result[..32].copy_from_slice(&Sha256::digest(input)),
            48 => result[..48].copy_from_slice(&Sha384::digest(input)),
            64 => result.copy_from_slice(&Sha512::digest(input)),
            _ => unreachable!(),
        }
        result
    }
    // ------------------------=
    // FUNC: padding
    // DESC: Requires exact PKCS1 DigestInfo or PSS with matching MGF1 and digest-length salt; rejects all trailing padding data.
    // ------------------=
    fn padding(self, encoded: &[u8], bits: usize, message: &[u8]) -> bool {
        let h = self.0 as usize;
        let digest = self.hash(message);
        if !self.1 {
            // SHA2 DigestInfo OIDs differ only in their final arc.
            let mut prefix = [
                0x30, 0, 0x30, 0x0d, 0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0,
                0x05, 0x00, 0x04, 0,
            ];
            prefix[1] = (h + 17) as u8;
            prefix[14] = match h {
                32 => 1,
                48 => 2,
                _ => 3,
            };
            prefix[18] = h as u8;
            if encoded.len() < h + prefix.len() + 11 {
                return false;
            }
            let end = encoded.len() - h - prefix.len();
            return encoded[..2] == [0, 1]
                && encoded[2..end - 1].iter().all(|b| *b == 255)
                && encoded[end - 1] == 0
                && encoded[end..end + prefix.len()] == prefix
                && encoded[end + prefix.len()..] == digest[..h];
        }
        let em_bits = bits - 1;
        let length = em_bits.div_ceil(8);
        if encoded.len() < length || encoded[..encoded.len() - length].iter().any(|b| *b != 0) {
            return false;
        }
        let encoded = &encoded[encoded.len() - length..];
        if length < 2 * h + 2 || encoded[length - 1] != 0xbc {
            return false;
        }
        let db_len = length - h - 1;
        let top_mask = 0xffu8 >> (8 * length - em_bits);
        if encoded[0] & !top_mask != 0 {
            return false;
        }
        let expected = &encoded[db_len..length - 1];
        let mut db = [0u8; 512];
        let mut seed = [0u8; 68];
        seed[..h].copy_from_slice(expected);
        for (counter, block) in db[..db_len].chunks_mut(h).enumerate() {
            seed[h..h + 4].copy_from_slice(&(counter as u32).to_be_bytes());
            let mask = self.hash(&seed[..h + 4]);
            for (i, byte) in block.iter_mut().enumerate() {
                *byte = encoded[counter * h + i] ^ mask[i];
            }
        }
        db[0] &= top_mask;
        let delimiter = db_len - h - 1;
        if db[..delimiter].iter().any(|b| *b != 0) || db[delimiter] != 1 {
            return false;
        }
        let mut hashed = [0u8; 136];
        hashed[8..8 + h].copy_from_slice(&digest[..h]);
        hashed[8 + h..8 + 2 * h].copy_from_slice(&db[db_len - h..db_len]);
        self.hash(&hashed[..8 + 2 * h])[..h] == *expected
    }
}
impl SignatureVerificationAlgorithm for Rsa {
    // ------------------------=
    // FUNC: verify_signature
    // DESC: Validates DER RSA public keys and representatives before bounded 2048-4096 bit public exponentiation.
    // ------------------=
    fn verify_signature(
        &self,
        key: &[u8],
        message: &[u8],
        signature: &[u8],
    ) -> Result<(), InvalidSignature> {
        let key = pkcs1::RsaPublicKey::try_from(key).map_err(|_| InvalidSignature)?;
        let n = key.modulus.as_bytes();
        let e = key.public_exponent.as_bytes();
        if !(256..=512).contains(&n.len())
            || e.is_empty()
            || e.len() > 4
            || signature.len() != n.len()
            || n[n.len() - 1] & 1 == 0
        {
            return Err(InvalidSignature);
        }
        let bits = n.len() * 8 - n[0].leading_zeros() as usize;
        let exponent = e.iter().fold(0u64, |v, b| (v << 8) | *b as u64);
        if bits < 2048 || exponent < 3 || exponent & 1 == 0 {
            return Err(InvalidSignature);
        }
        let mut modulus = [0; 512];
        let mut representative = [0; 512];
        modulus[512 - n.len()..].copy_from_slice(n);
        representative[512 - n.len()..].copy_from_slice(signature);
        let n = U4096::from_be_slice(&modulus);
        let s = U4096::from_be_slice(&representative);
        if s >= n {
            return Err(InvalidSignature);
        }
        let value = DynResidue::new(&s, DynResidueParams::new(&n))
            .pow_bounded_exp(&U64::from_u64(exponent), 32)
            .retrieve()
            .to_be_bytes();
        if self.padding(&value[512 - signature.len()..], bits, message) {
            Ok(())
        } else {
            Err(InvalidSignature)
        }
    }
    // ------------------------=
    // FUNC: public_key_alg_id
    // DESC: Accepts ordinary rsaEncryption keys only, not restricted PSS keys with different parameters.
    // ------------------=
    fn public_key_alg_id(&self) -> AlgorithmIdentifier {
        alg_id::RSA_ENCRYPTION
    }
    // ------------------------=
    // FUNC: signature_alg_id
    // DESC: Binds RSA verification to exact SHA2 and PSS parameter identifiers understood by webpki.
    // ------------------=
    fn signature_alg_id(&self) -> AlgorithmIdentifier {
        match (self.0, self.1) {
            (32, false) => alg_id::RSA_PKCS1_SHA256,
            (48, false) => alg_id::RSA_PKCS1_SHA384,
            (64, false) => alg_id::RSA_PKCS1_SHA512,
            (32, true) => alg_id::RSA_PSS_SHA256,
            (48, true) => alg_id::RSA_PSS_SHA384,
            (64, true) => alg_id::RSA_PSS_SHA512,
            _ => unreachable!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        io::Write,
        process::{Command, Stdio},
    };
    // ------------------------=
    // FUNC: openssl
    // DESC: Runs an independent host signing implementation and checks its exit status, never diagnostic prose.
    // ------------------=
    fn openssl(args: &[&str], input: &[u8]) -> std::vec::Vec<u8> {
        let mut child = Command::new("openssl")
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(input).unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(output.status.success());
        output.stdout
    }
    // ------------------------=
    // FUNC: independent_signatures_and_rejections
    // DESC: Verifies all supported SHA2 and padding combinations against OpenSSL with three key sizes and rejects altered inputs.
    // ------------------=
    #[test]
    fn independent_signatures_and_rejections() {
        let directory =
            std::env::temp_dir().join(std::format!("infinity-rsa-{}", std::process::id()));
        fs::create_dir(&directory).unwrap();
        let private = directory.join("test.pem");
        let message = b"InfinityOS native certificate verification";
        for size in [2048, 3072, 4096] {
            let key = openssl(
                &[
                    "genpkey",
                    "-algorithm",
                    "RSA",
                    "-pkeyopt",
                    &std::format!("rsa_keygen_bits:{size}"),
                ],
                b"",
            );
            fs::write(&private, &key).unwrap();
            let public = openssl(
                &[
                    "rsa",
                    "-in",
                    private.to_str().unwrap(),
                    "-RSAPublicKey_out",
                    "-outform",
                    "DER",
                ],
                b"",
            );
            for algorithm in [PKCS256, PKCS384, PKCS512, PSS256, PSS384, PSS512] {
                let hash = std::format!("-sha{}", algorithm.0 as usize * 8);
                let mut args = std::vec!["dgst", &hash, "-sign", private.to_str().unwrap()];
                if algorithm.1 {
                    args.extend_from_slice(&[
                        "-sigopt",
                        "rsa_padding_mode:pss",
                        "-sigopt",
                        "rsa_pss_saltlen:digest",
                    ]);
                }
                let mut signature = openssl(&args, message);
                assert!(
                    algorithm
                        .verify_signature(&public, message, &signature)
                        .is_ok(),
                    "{size} {algorithm:?}"
                );
                assert!(algorithm
                    .verify_signature(&public, b"changed", &signature)
                    .is_err());
                assert!(algorithm
                    .verify_signature(&public, message, &signature[1..])
                    .is_err());
                signature[0] ^= 1;
                assert!(algorithm
                    .verify_signature(&public, message, &signature)
                    .is_err());
                let mut invalid_key = public.clone();
                invalid_key.push(0);
                assert!(algorithm
                    .verify_signature(&invalid_key, message, &signature)
                    .is_err());
                signature.fill(255);
                assert!(algorithm
                    .verify_signature(&public, message, &signature)
                    .is_err());
            }
            // TLS requires digest-length PSS salt: OpenSSL's otherwise-valid zero-salt signature must fail.
            let signature = openssl(
                &[
                    "dgst",
                    "-sha256",
                    "-sign",
                    private.to_str().unwrap(),
                    "-sigopt",
                    "rsa_padding_mode:pss",
                    "-sigopt",
                    "rsa_pss_saltlen:0",
                ],
                message,
            );
            assert!(PSS256
                .verify_signature(&public, message, &signature)
                .is_err());
        }
        fs::remove_file(private).unwrap();
        fs::remove_dir(directory).unwrap();
    }
}
