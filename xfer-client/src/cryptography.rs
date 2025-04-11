use anyhow::{Context, Result};
use blake3::Hasher;
use chacha20poly1305::{
    AeadCore, KeyInit,
    aead::{Aead, AeadMutInPlace, OsRng, generic_array::typenum::Unsigned},
};
use hex::ToHex;

type CryptoImpl = chacha20poly1305::XChaCha20Poly1305;
type CryptoNonce = chacha20poly1305::XNonce;
type CryptoNonceSize = <CryptoImpl as AeadCore>::NonceSize;

pub const REMOTE_ID_HASH_SNIP_AT: usize = 24;

#[derive(Debug)]
pub struct Cryptography;

impl Cryptography {
    /// Encrypt a byte array in-place using a random key & nonce.
    ///
    /// Upon success the decryption key is returned.
    pub fn encrypt_in_place(bytes: &mut Vec<u8>) -> Result<String> {
        let key = CryptoImpl::generate_key(&mut OsRng);
        let nonce = CryptoImpl::generate_nonce(&mut OsRng);
        let mut cipher = CryptoImpl::new(&key);
        cipher
            .encrypt_in_place(&nonce, b"", bytes)
            .context("failed to encrypt bytes in place")?;
        bytes.splice(..0, nonce.iter().copied());
        Ok(key.encode_hex_upper())
    }

    /// Decrypt a byte array with its decryption key.
    pub fn decrypt(bytes: &[u8], key: &str) -> Result<Vec<u8>> {
        let (nonce, encrypted_bytes) = bytes.split_at(CryptoNonceSize::to_usize());
        let key = hex::decode(key).context("failed to decode hex from key input")?;
        let cipher = CryptoImpl::new_from_slice(&key)?;
        cipher
            .decrypt(CryptoNonce::from_slice(nonce), encrypted_bytes)
            .context("failed to decrypt bytes")
    }

    /// Create a hash of the given data.
    pub fn create_hash(data: impl AsRef<[u8]>) -> String {
        let mut hasher = Hasher::new();
        hasher.update(data.as_ref());
        hasher.finalize().to_hex().to_string()
    }
}
