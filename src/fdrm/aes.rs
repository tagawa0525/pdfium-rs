use crate::error::{Error, Result};
use aes::{Aes128, Aes256};
use cbc::cipher::{BlockDecryptMut, BlockEncryptMut, KeyIvInit, block_padding::NoPadding};
use cbc::{Decryptor, Encryptor};

/// Decrypt data with AES-128-CBC, no padding removal.
///
/// `key` must be 16 bytes. `iv` must be 16 bytes. `ciphertext` must be
/// a multiple of 16 bytes. Returns raw decrypted bytes (no PKCS#7 removal).
pub fn decrypt_aes128_cbc(key: &[u8], iv: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>> {
    let key: &[u8; 16] = key
        .try_into()
        .map_err(|_| Error::InvalidPdf("AES-128-CBC: key must be 16 bytes".into()))?;
    let iv: &[u8; 16] = iv
        .try_into()
        .map_err(|_| Error::InvalidPdf("AES-128-CBC: IV must be 16 bytes".into()))?;
    if !ciphertext.len().is_multiple_of(16) {
        return Err(Error::InvalidPdf(
            "AES-128-CBC: ciphertext length must be a multiple of 16".into(),
        ));
    }
    let mut buf = ciphertext.to_vec();
    Decryptor::<Aes128>::new(key.into(), iv.into())
        .decrypt_padded_mut::<NoPadding>(&mut buf)
        .map_err(|e| Error::InvalidPdf(format!("AES-128-CBC: {e}")))?;
    Ok(buf)
}

/// Decrypt data with AES-256-CBC, no padding removal.
///
/// `key` must be 32 bytes. `iv` must be 16 bytes. `ciphertext` must be
/// a multiple of 16 bytes. Returns raw decrypted bytes (no PKCS#7 removal).
pub fn decrypt_aes256_cbc(key: &[u8], iv: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>> {
    let key: &[u8; 32] = key
        .try_into()
        .map_err(|_| Error::InvalidPdf("AES-256-CBC: key must be 32 bytes".into()))?;
    let iv: &[u8; 16] = iv
        .try_into()
        .map_err(|_| Error::InvalidPdf("AES-256-CBC: IV must be 16 bytes".into()))?;
    if !ciphertext.len().is_multiple_of(16) {
        return Err(Error::InvalidPdf(
            "AES-256-CBC: ciphertext length must be a multiple of 16".into(),
        ));
    }
    let mut buf = ciphertext.to_vec();
    Decryptor::<Aes256>::new(key.into(), iv.into())
        .decrypt_padded_mut::<NoPadding>(&mut buf)
        .map_err(|e| Error::InvalidPdf(format!("AES-256-CBC: {e}")))?;
    Ok(buf)
}

/// Encrypt data with AES-128-CBC, no padding.
///
/// `key` must be 16 bytes. `iv` must be 16 bytes. `plaintext` must be
/// a multiple of 16 bytes. Returns raw encrypted bytes.
///
/// Used in PDF Standard Security Handler revision 6 (Revision6_Hash).
pub fn encrypt_aes128_cbc(key: &[u8], iv: &[u8], plaintext: &[u8]) -> Result<Vec<u8>> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    // AES-128-CBC test vector from NIST SP 800-38A, F.2.1
    // Key:   2b7e151628aed2a6abf7158809cf4f3c
    // IV:    000102030405060708090a0b0c0d0e0f
    // PT:    6bc1bee22e409f96e93d7e117393172a ae2d8a571e03ac9c9eb76fac45af8e51
    // CT:    7649abac8119b246cee98e9b12e9197d 5086cb9b507219ee95db113a917678b2

    #[test]
    fn aes128_cbc_decrypt_nist_vector() {
        let key = hex_decode("2b7e151628aed2a6abf7158809cf4f3c");
        let iv = hex_decode("000102030405060708090a0b0c0d0e0f");
        let ciphertext =
            hex_decode("7649abac8119b246cee98e9b12e9197d5086cb9b507219ee95db113a917678b2");
        let expected =
            hex_decode("6bc1bee22e409f96e93d7e117393172aae2d8a571e03ac9c9eb76fac45af8e51");
        let result = decrypt_aes128_cbc(&key, &iv, &ciphertext).unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn aes256_cbc_decrypt_nist_vector() {
        // NIST SP 800-38A F.2.5/F.2.6: AES-256-CBC (2 blocks)
        let key = hex_decode("603deb1015ca71be2b73aef0857d77811f352c073b6108d72d9810a30914dff4");
        let iv = hex_decode("000102030405060708090a0b0c0d0e0f");
        let ciphertext =
            hex_decode("f58c4c04d6e5f1ba779eabfb5f7bfbd69cfc4e967edb808d679f777bc6702c7d");
        let expected =
            hex_decode("6bc1bee22e409f96e93d7e117393172aae2d8a571e03ac9c9eb76fac45af8e51");
        let result = decrypt_aes256_cbc(&key, &iv, &ciphertext).unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn aes128_cbc_wrong_key_size_is_error() {
        let result = decrypt_aes128_cbc(b"short", &[0u8; 16], &[0u8; 16]);
        assert!(result.is_err());
    }

    #[test]
    fn aes128_cbc_wrong_iv_size_is_error() {
        let result = decrypt_aes128_cbc(&[0u8; 16], b"short", &[0u8; 16]);
        assert!(result.is_err());
    }

    #[test]
    fn aes128_cbc_non_multiple_ciphertext_is_error() {
        let result = decrypt_aes128_cbc(&[0u8; 16], &[0u8; 16], &[0u8; 15]);
        assert!(result.is_err());
    }

    #[test]
    fn aes256_cbc_wrong_key_size_is_error() {
        let result = decrypt_aes256_cbc(b"short", &[0u8; 16], &[0u8; 16]);
        assert!(result.is_err());
    }

    #[test]
    fn aes256_cbc_wrong_iv_size_is_error() {
        let result = decrypt_aes256_cbc(&[0u8; 32], b"short", &[0u8; 16]);
        assert!(result.is_err());
    }

    #[test]
    fn aes256_cbc_non_multiple_ciphertext_is_error() {
        let result = decrypt_aes256_cbc(&[0u8; 32], &[0u8; 16], &[0u8; 15]);
        assert!(result.is_err());
    }

    // AES-128-CBC encrypt: NIST SP 800-38A F.2.1 (inverse of decrypt vector)
    #[test]
    #[ignore = "not yet implemented"]
    fn aes128_cbc_encrypt_nist_vector() {
        let key = hex_decode("2b7e151628aed2a6abf7158809cf4f3c");
        let iv = hex_decode("000102030405060708090a0b0c0d0e0f");
        let plaintext =
            hex_decode("6bc1bee22e409f96e93d7e117393172aae2d8a571e03ac9c9eb76fac45af8e51");
        let expected =
            hex_decode("7649abac8119b246cee98e9b12e9197d5086cb9b507219ee95db113a917678b2");
        let result = encrypt_aes128_cbc(&key, &iv, &plaintext).unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn aes128_cbc_encrypt_decrypt_roundtrip() {
        let key = [0x42u8; 16];
        let iv = [0x00u8; 16];
        let plaintext = [0xABu8; 32]; // 2 blocks
        let encrypted = encrypt_aes128_cbc(&key, &iv, &plaintext).unwrap();
        let decrypted = decrypt_aes128_cbc(&key, &iv, &encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    fn hex_decode(s: &str) -> Vec<u8> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    }
}
