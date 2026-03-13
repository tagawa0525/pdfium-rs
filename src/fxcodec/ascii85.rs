use crate::error::{Error, Result};

/// Decode ASCII85 (Base-85) encoded data (PDF ASCII85Decode filter).
///
/// Each group of 5 ASCII characters (base-85 digits, '!' through 'u')
/// encodes 4 bytes. The special character 'z' represents four zero bytes.
/// The end-of-data marker `~>` terminates decoding. Whitespace is ignored.
pub fn decode(input: &[u8]) -> Result<Vec<u8>> {
    let mut output = Vec::new();
    let mut group = Vec::with_capacity(5);
    let mut i = 0;

    while i < input.len() {
        let ch = input[i];
        i += 1;

        // Check for end-of-data marker
        if ch == b'~' {
            if i < input.len() && input[i] == b'>' {
                break;
            }
            return Err(Error::InvalidPdf("ASCII85Decode: '~' without '>'".into()));
        }

        // Skip whitespace
        if ch.is_ascii_whitespace() {
            continue;
        }

        // 'z' shorthand for four zero bytes (only valid between groups)
        if ch == b'z' {
            if !group.is_empty() {
                return Err(Error::InvalidPdf(
                    "ASCII85Decode: 'z' inside a group".into(),
                ));
            }
            output.extend_from_slice(&[0, 0, 0, 0]);
            continue;
        }

        // Valid base-85 digit: '!' (33) through 'u' (117)
        if !(b'!'..=b'u').contains(&ch) {
            return Err(Error::InvalidPdf(format!(
                "ASCII85Decode: invalid character 0x{ch:02X}"
            )));
        }

        group.push(ch - b'!');

        if group.len() == 5 {
            let value = decode_group(&group)?;
            output.extend_from_slice(&value.to_be_bytes());
            group.clear();
        }
    }

    // Handle partial final group (2-4 chars)
    if !group.is_empty() {
        if group.len() == 1 {
            return Err(Error::InvalidPdf(
                "ASCII85Decode: partial group with only 1 character".into(),
            ));
        }
        let n_bytes = group.len() - 1;
        // Pad with 'u' (84) to make 5 characters
        while group.len() < 5 {
            group.push(84); // 'u' - b'!' = 84
        }
        let value = decode_group(&group)?;
        let bytes = value.to_be_bytes();
        output.extend_from_slice(&bytes[..n_bytes]);
    }

    Ok(output)
}

fn decode_group(group: &[u8]) -> Result<u32> {
    let mut value: u64 = 0;
    for &digit in group {
        value = value * 85 + digit as u64;
    }
    if value > u32::MAX as u64 {
        return Err(Error::InvalidPdf("ASCII85Decode: group overflow".into()));
    }
    Ok(value as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    // "test" = 0x74657374 → ASCII85 "FCfN8" (verified by manual computation)

    #[test]
    fn decode_full_group() {
        let result = decode(b"FCfN8~>").unwrap();
        assert_eq!(result, b"test");
    }

    #[test]
    fn decode_z_shorthand() {
        let result = decode(b"z~>").unwrap();
        assert_eq!(result, &[0, 0, 0, 0]);
    }

    #[test]
    fn decode_z_between_groups() {
        // "test" + [0,0,0,0] + "test"
        let result = decode(b"FCfN8zFCfN8~>").unwrap();
        let mut expected = b"test".to_vec();
        expected.extend_from_slice(&[0, 0, 0, 0]);
        expected.extend_from_slice(b"test");
        assert_eq!(result, expected);
    }

    #[test]
    fn decode_partial_group() {
        // "AB" = 0x4142 → padded 0x41420000 → ASCII85 "5sb" (3 chars for 2 bytes)
        let result = decode(b"5sb~>").unwrap();
        assert_eq!(result, b"AB");
    }

    #[test]
    fn decode_with_whitespace() {
        let result = decode(b"FC fN 8~>").unwrap();
        assert_eq!(result, b"test");
    }

    #[test]
    fn decode_empty() {
        let result = decode(b"~>").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn decode_invalid_char() {
        // 'v' (0x76 = 118) is beyond 'u' (117)
        let result = decode(b"v~>");
        assert!(result.is_err());
    }

    #[test]
    fn decode_z_inside_group_is_error() {
        // 'z' after partial group start should be an error
        let result = decode(b"Fz~>");
        assert!(result.is_err());
    }
}
