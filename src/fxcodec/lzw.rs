use crate::error::Result;

/// Decode LZW-compressed data (PDF LZWDecode filter).
///
/// Implements the LZW decompression algorithm as specified in the PDF spec
/// (derived from the TIFF 6.0 spec). Uses variable-width codes starting
/// at 9 bits, with clear code = 256 and EOD code = 257.
///
/// The `early_change` parameter controls when the code width increases:
/// - `true` (default, PDF spec): code width increases one code early
/// - `false`: code width increases after the code is actually used
pub fn decode(_input: &[u8], _early_change: bool) -> Result<Vec<u8>> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "not yet implemented"]
    fn decode_simple() {
        // LZW compressed "ABABAB" with clear code at start
        // This is a well-known LZW test vector
        // Clear(256) + 'A'(65) + 'B'(66) + 258(AB) + 258(AB) + EOD(257)
        // Encoded as 9-bit codes packed MSB-first
        let input = [0x80, 0x20, 0xA1, 0x2C, 0x85, 0x02, 0x80];
        let result = decode(&input, true).unwrap();
        assert_eq!(result, b"ABABAB");
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn decode_empty_stream() {
        // Just clear code + EOD
        // 256 (clear) + 257 (EOD) as 9-bit codes MSB-first
        let input = [0x80, 0x40, 0x80];
        let result = decode(&input, true).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn decode_single_byte() {
        // Clear(256) + 'X'(88) + EOD(257)
        // 9-bit codes: 100000000 01011000 100000001
        let input = [0x80, 0x2C, 0x40, 0x80];
        let result = decode(&input, true).unwrap();
        assert_eq!(result, b"X");
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn decode_repeated_byte() {
        // Clear(256) + 'A'(65) + 258(AA) + EOD(257)
        // Tests dictionary entry creation: after outputting 'A',
        // next code 258 refers to "AA"
        let input = [0x80, 0x20, 0x93, 0x02, 0x80];
        let result = decode(&input, true).unwrap();
        assert_eq!(result, b"AAA");
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn decode_early_change_flag() {
        // With early_change=true (PDF default), code width increases
        // one code before the dictionary is actually full at that width.
        // This is a subtle but important difference for real PDF streams.
        let input = [0x80, 0x20, 0xA1, 0x2C, 0x85, 0x02, 0x80];
        let result_early = decode(&input, true).unwrap();
        assert_eq!(result_early, b"ABABAB");
    }
}
