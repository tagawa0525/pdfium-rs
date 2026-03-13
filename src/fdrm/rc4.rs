/// RC4 stream cipher.
///
/// Used in PDF Standard Security Handler revisions 2, 3, and 4.
/// Key length: 5-16 bytes (40-128 bit).
#[allow(dead_code)]
pub struct Rc4 {
    s: [u8; 256],
    i: u8,
    j: u8,
}

impl Rc4 {
    /// Initialize RC4 with the given key.
    pub fn new(key: &[u8]) -> Self {
        let _ = key;
        Rc4 {
            s: [0u8; 256],
            i: 0,
            j: 0,
        }
    }

    /// Encrypt or decrypt data in-place (RC4 is symmetric).
    pub fn apply_keystream(&mut self, data: &mut [u8]) {
        let _ = data;
    }
}

/// Convenience: encrypt/decrypt a slice with RC4 using the given key.
pub fn crypt(key: &[u8], data: &[u8]) -> Vec<u8> {
    let _ = key;
    let _ = data;
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    // RC4 test vectors from RFC 6229 (Test Vector 1: key = 0x0102030405)
    // Input:  0x00 * 16
    // Output: b707 0d3f 4058 9891 d8a6 a2c4 9057 9e59

    #[test]
    #[ignore = "not yet implemented"]
    fn rc4_encrypt_rfc6229_vector1() {
        let key = [0x01u8, 0x02, 0x03, 0x04, 0x05];
        let plaintext = [0x00u8; 16];
        let expected = [
            0xb7u8, 0x07, 0x0d, 0x3f, 0x40, 0x58, 0x98, 0x91, 0xd8, 0xa6, 0xa2, 0xc4, 0x90, 0x57,
            0x9e, 0x59,
        ];
        let result = crypt(&key, &plaintext);
        assert_eq!(result, expected);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn rc4_symmetric() {
        let key = b"secret";
        let plaintext = b"Hello, RC4!";
        let ciphertext = crypt(key, plaintext);
        let recovered = crypt(key, &ciphertext);
        assert_eq!(recovered, plaintext);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn rc4_empty_input() {
        let result = crypt(b"key", b"");
        assert!(result.is_empty());
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn rc4_stateful_streaming() {
        let key = b"key";
        let plaintext = b"Hello, streaming RC4!";
        // Streaming should match single-call crypt
        let mut rc4 = Rc4::new(key);
        let mut out = plaintext.to_vec();
        rc4.apply_keystream(&mut out);
        let expected = crypt(key, plaintext);
        assert_eq!(out, expected);
    }
}
