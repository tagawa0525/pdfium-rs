use crate::error::Result;

/// Decode ASCII85 (Base-85) encoded data (PDF ASCII85Decode filter).
///
/// Each group of 5 ASCII characters (base-85 digits, '!' through 'u')
/// encodes 4 bytes. The special character 'z' represents four zero bytes.
/// The end-of-data marker `~>` terminates decoding. Whitespace is ignored.
pub fn decode(_input: &[u8]) -> Result<Vec<u8>> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "not yet implemented"]
    fn decode_simple() {
        // "Hello" in ASCII85: 87cURD]j7BEbo7~>
        let result = decode(b"87cURD]j7BEbo7~>").unwrap();
        assert_eq!(result, b"Hello, World!");
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn decode_z_shorthand() {
        // 'z' = four zero bytes
        let result = decode(b"z~>").unwrap();
        assert_eq!(result, &[0, 0, 0, 0]);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn decode_z_mixed() {
        // 'z' can appear between normal groups
        let result = decode(b"87cURDz]j7BEbo7~>").unwrap();
        let mut expected = b"Hello".to_vec();
        expected.extend_from_slice(&[0, 0, 0, 0]);
        expected.extend_from_slice(b", World!");
        assert_eq!(result, expected);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn decode_partial_group() {
        // Partial final group (< 5 chars) is padded with 'u' (84)
        // "AB" = 2 bytes. Encoded as 3 ASCII85 chars
        let input = b"6<~>"; // incomplete test, will refine in GREEN
        let result = decode(input);
        // At minimum, should not panic
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn decode_with_whitespace() {
        let result = decode(b"87cUR D]j7B Ebo7~>").unwrap();
        assert_eq!(result, b"Hello, World!");
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn decode_empty() {
        let result = decode(b"~>").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn decode_invalid_char() {
        // Characters outside '!'..'u' and 'z' are invalid (except whitespace)
        let result = decode(b"v~>");
        assert!(result.is_err());
    }
}
