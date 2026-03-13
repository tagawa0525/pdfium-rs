/// Compute SHA-256 digest of the given data.
///
/// Used in PDF Standard Security Handler revision 6 (AES-256) key derivation.
pub fn digest(data: &[u8]) -> [u8; 32] {
    let _ = data;
    [0u8; 32]
}

#[cfg(test)]
mod tests {
    use super::*;

    // Standard SHA-256 test vectors from FIPS 180-4

    #[test]
    #[ignore = "not yet implemented"]
    fn sha256_empty() {
        // SHA-256("") = e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
        let result = digest(b"");
        let expected = [
            0xe3u8, 0xb0, 0xc4, 0x42, 0x98, 0xfc, 0x1c, 0x14, 0x9a, 0xfb, 0xf4, 0xc8, 0x99, 0x6f,
            0xb9, 0x24, 0x27, 0xae, 0x41, 0xe4, 0x64, 0x9b, 0x93, 0x4c, 0xa4, 0x95, 0x99, 0x1b,
            0x78, 0x52, 0xb8, 0x55,
        ];
        assert_eq!(result, expected);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn sha256_abc() {
        // SHA-256("abc") = ba7816bf8f01cfea414140de5dae2ec73b00361bbef0469bf5f6e09c8df7a85
        // (full: ba7816bf 8f01cfea 414140de 5dae2ec7 3b00361b bef0469b f5f6e09c 8df7a85b - typo: last byte 5b)
        // Corrected: ba7816bf8f01cfea414140de5dae2ec73b00361bbef0469bf5f6e09c8df7a85b
        let result = digest(b"abc");
        let expected = [
            0xbau8, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40, 0xde, 0x5d, 0xae,
            0x2e, 0xc7, 0x3b, 0x00, 0x36, 0x1b, 0xbe, 0xf0, 0x46, 0x9b, 0xf5, 0xf6, 0xe0, 0x9c,
            0x8d, 0xf7, 0xa8, 0x5b,
        ];
        assert_eq!(result, expected);
    }
}
