use crate::error::{Error, Result};

/// Decrypt data with AES-128-CBC.
///
/// `key` must be 16 bytes. `iv` must be 16 bytes. `ciphertext` must be
/// a multiple of 16 bytes. Returns the decrypted plaintext with PKCS#7
/// padding removed.
pub fn decrypt_aes128_cbc(key: &[u8], iv: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>> {
    let _ = key;
    let _ = iv;
    let _ = ciphertext;
    Err(Error::InvalidPdf("AES-128-CBC: not implemented".into()))
}

/// Decrypt data with AES-256-CBC.
///
/// `key` must be 32 bytes. `iv` must be 16 bytes. `ciphertext` must be
/// a multiple of 16 bytes. Returns the decrypted plaintext with PKCS#7
/// padding removed.
pub fn decrypt_aes256_cbc(key: &[u8], iv: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>> {
    let _ = key;
    let _ = iv;
    let _ = ciphertext;
    Err(Error::InvalidPdf("AES-256-CBC: not implemented".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    // AES-128-CBC test vector from NIST SP 800-38A, F.2.1
    // Key:   2b7e151628aed2a6abf7158809cf4f3c
    // IV:    000102030405060708090a0b0c0d0e0f
    // PT:    6bc1bee22e409f96e93d7e117393172aae2d8a571e03ac9c9eb76fac45af8e5130c81c46a35ce411
    // CT:    7649abac8119b246cee98e9b12e9197d5086cb9b507219ee95db113a917678b273bed6b8e3c1743b7116e69e22229516

    #[test]
    #[ignore = "not yet implemented"]
    fn aes128_cbc_decrypt_nist_vector() {
        let key = hex_decode("2b7e151628aed2a6abf7158809cf4f3c");
        let iv = hex_decode("000102030405060708090a0b0c0d0e0f");
        // First two blocks of NIST test vector (32 bytes plaintext, 32 bytes ciphertext)
        let ciphertext =
            hex_decode("7649abac8119b246cee98e9b12e9197d5086cb9b507219ee95db113a917678b");
        let expected =
            hex_decode("6bc1bee22e409f96e93d7e117393172aae2d8a571e03ac9c9eb76fac45af8e5");
        // Note: PKCS#7 padding is not present in the middle blocks, so we test without padding removal
        let result = decrypt_aes128_cbc_nopad(&key, &iv, &ciphertext).unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn aes256_cbc_decrypt_nist_vector() {
        // NIST SP 800-38A F.2.5: AES-256-CBC
        let key = hex_decode("603deb1015ca71be2b73aef0857d77811f352c073b6108d72d9810a30914dff4");
        let iv = hex_decode("000102030405060708090a0b0c0d0e0f");
        let ciphertext =
            hex_decode("f58c4c04d6e5f1ba779eabfb5f7bfbd6485a5c81519cf378fa36d42b8547edc0");
        let expected =
            hex_decode("6bc1bee22e409f96e93d7e117393172aae2d8a571e03ac9c9eb76fac45af8e5");
        let result = decrypt_aes256_cbc_nopad(&key, &iv, &ciphertext).unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn aes128_cbc_wrong_key_size_is_error() {
        let result = decrypt_aes128_cbc(b"short", &[0u8; 16], &[0u8; 16]);
        assert!(result.is_err());
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn aes256_cbc_wrong_key_size_is_error() {
        let result = decrypt_aes256_cbc(b"short", &[0u8; 16], &[0u8; 16]);
        assert!(result.is_err());
    }

    /// Variant without PKCS#7 padding removal (for testing mid-stream blocks).
    fn decrypt_aes128_cbc_nopad(key: &[u8], iv: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>> {
        let _ = key;
        let _ = iv;
        let _ = ciphertext;
        Err(Error::InvalidPdf("nopad: not implemented".into()))
    }

    fn decrypt_aes256_cbc_nopad(key: &[u8], iv: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>> {
        let _ = key;
        let _ = iv;
        let _ = ciphertext;
        Err(Error::InvalidPdf("nopad: not implemented".into()))
    }

    fn hex_decode(s: &str) -> Vec<u8> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    }
}
