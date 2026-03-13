use crate::error::Result;

/// Decode ASCIIHex-encoded data (PDF ASCIIHexDecode filter).
///
/// Converts pairs of hexadecimal digits to bytes. Whitespace is ignored.
/// The end-of-data marker `>` terminates decoding. An odd trailing nibble
/// is padded with 0.
pub fn decode(_input: &[u8]) -> Result<Vec<u8>> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "not yet implemented"]
    fn decode_simple() {
        let result = decode(b"48656C6C6F>").unwrap();
        assert_eq!(result, b"Hello");
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn decode_lowercase() {
        let result = decode(b"48656c6c6f>").unwrap();
        assert_eq!(result, b"Hello");
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn decode_with_whitespace() {
        let result = decode(b"48 65 6C 6C 6F>").unwrap();
        assert_eq!(result, b"Hello");
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn decode_odd_nibble() {
        // Odd trailing nibble padded with 0: "A" -> 0xA0
        let result = decode(b"4865A>").unwrap();
        assert_eq!(result, &[0x48, 0x65, 0xA0]);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn decode_empty() {
        let result = decode(b">").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn decode_no_eod_marker() {
        // If '>' is missing, decode to end of input
        let result = decode(b"4865").unwrap();
        assert_eq!(result, &[0x48, 0x65]);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn decode_invalid_char() {
        let result = decode(b"48ZZ65>");
        assert!(result.is_err());
    }
}
