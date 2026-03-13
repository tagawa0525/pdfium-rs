use crate::error::{Error, Result};

/// Decode ASCIIHex-encoded data (PDF ASCIIHexDecode filter).
///
/// Converts pairs of hexadecimal digits to bytes. Whitespace is ignored.
/// The end-of-data marker `>` terminates decoding. An odd trailing nibble
/// is padded with 0.
pub fn decode(input: &[u8]) -> Result<Vec<u8>> {
    let mut output = Vec::new();
    let mut high_nibble: Option<u8> = None;

    for &ch in input {
        if ch == b'>' {
            break;
        }
        if ch.is_ascii_whitespace() {
            continue;
        }

        let nibble = hex_nibble(ch)?;

        match high_nibble.take() {
            None => high_nibble = Some(nibble),
            Some(hi) => output.push((hi << 4) | nibble),
        }
    }

    // Odd trailing nibble: pad with 0
    if let Some(hi) = high_nibble {
        output.push(hi << 4);
    }

    Ok(output)
}

fn hex_nibble(ch: u8) -> Result<u8> {
    match ch {
        b'0'..=b'9' => Ok(ch - b'0'),
        b'a'..=b'f' => Ok(ch - b'a' + 10),
        b'A'..=b'F' => Ok(ch - b'A' + 10),
        _ => Err(Error::InvalidPdf(format!(
            "ASCIIHexDecode: invalid hex character 0x{ch:02X}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_simple() {
        let result = decode(b"48656C6C6F>").unwrap();
        assert_eq!(result, b"Hello");
    }

    #[test]
    fn decode_lowercase() {
        let result = decode(b"48656c6c6f>").unwrap();
        assert_eq!(result, b"Hello");
    }

    #[test]
    fn decode_with_whitespace() {
        let result = decode(b"48 65 6C 6C 6F>").unwrap();
        assert_eq!(result, b"Hello");
    }

    #[test]
    fn decode_odd_nibble() {
        // Odd trailing nibble padded with 0: "A" -> 0xA0
        let result = decode(b"4865A>").unwrap();
        assert_eq!(result, &[0x48, 0x65, 0xA0]);
    }

    #[test]
    fn decode_empty() {
        let result = decode(b">").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn decode_no_eod_marker() {
        // If '>' is missing, decode to end of input
        let result = decode(b"4865").unwrap();
        assert_eq!(result, &[0x48, 0x65]);
    }

    #[test]
    fn decode_invalid_char() {
        let result = decode(b"48ZZ65>");
        assert!(result.is_err());
    }
}
