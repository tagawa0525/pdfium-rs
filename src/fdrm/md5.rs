use md5::{Digest, Md5};

/// Compute MD5 digest of the given data.
///
/// Used in PDF Standard Security Handler for encryption key derivation.
pub fn digest(data: &[u8]) -> [u8; 16] {
    Md5::digest(data).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Standard MD5 test vectors from RFC 1321

    #[test]
    fn md5_empty() {
        // MD5("") = d41d8cd98f00b204e9800998ecf8427e
        let result = digest(b"");
        let expected = [
            0xd4u8, 0x1d, 0x8c, 0xd9, 0x8f, 0x00, 0xb2, 0x04, 0xe9, 0x80, 0x09, 0x98, 0xec, 0xf8,
            0x42, 0x7e,
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn md5_abc() {
        // MD5("abc") = 900150983cd24fb0d6963f7d28e17f72
        let result = digest(b"abc");
        let expected = [
            0x90u8, 0x01, 0x50, 0x98, 0x3c, 0xd2, 0x4f, 0xb0, 0xd6, 0x96, 0x3f, 0x7d, 0x28, 0xe1,
            0x7f, 0x72,
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn md5_message_digest() {
        // MD5("message digest") = f96b697d7cb7938d525a2f31aaf161d0
        let result = digest(b"message digest");
        let expected = [
            0xf9u8, 0x6b, 0x69, 0x7d, 0x7c, 0xb7, 0x93, 0x8d, 0x52, 0x5a, 0x2f, 0x31, 0xaa, 0xf1,
            0x61, 0xd0,
        ];
        assert_eq!(result, expected);
    }
}
