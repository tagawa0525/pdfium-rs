use crate::error::{Error, Result};

const CLEAR_CODE: u16 = 256;
const EOD_CODE: u16 = 257;
const FIRST_CODE: u16 = 258;
const MAX_CODE: u16 = 4096; // 12-bit maximum

/// Decode LZW-compressed data (PDF LZWDecode filter).
///
/// Implements the LZW decompression algorithm as specified in the PDF spec
/// (derived from the TIFF 6.0 spec). Uses variable-width codes starting
/// at 9 bits, with clear code = 256 and EOD code = 257.
///
/// The `early_change` parameter controls when the code width increases:
/// - `true` (default, PDF spec): code width increases one code early
/// - `false`: code width increases after the code is actually used
pub fn decode(input: &[u8], early_change: bool) -> Result<Vec<u8>> {
    let mut reader = BitReader::new(input);
    let mut output = Vec::new();

    let mut table = DictTable::new();
    let mut code_width: u8 = 9;
    let mut prev_code: Option<u16> = None;

    loop {
        let code = reader.read_bits(code_width)?;

        if code == EOD_CODE {
            break;
        }

        if code == CLEAR_CODE {
            table.reset();
            code_width = 9;
            prev_code = None;
            continue;
        }

        let entry = if code < table.next_code {
            // Code is in the dictionary
            table.get(code).to_vec()
        } else if code == table.next_code {
            // Special case: code == next_code (cScSc pattern)
            let prev = prev_code.ok_or_else(|| {
                Error::InvalidPdf("LZWDecode: code == next_code with no previous".into())
            })?;
            let mut entry = table.get(prev).to_vec();
            entry.push(entry[0]);
            entry
        } else {
            return Err(Error::InvalidPdf(format!(
                "LZWDecode: code {code} out of range (next={})",
                table.next_code
            )));
        };

        output.extend_from_slice(&entry);

        // Add new dictionary entry: previous string + first char of current
        if let Some(prev) = prev_code
            && table.next_code < MAX_CODE
        {
            let first_char = entry[0];
            table.add(prev, first_char);
        }

        // Update code width
        let threshold = if early_change {
            table.next_code
        } else {
            table.next_code - 1
        };
        if threshold >= (1 << code_width) && code_width < 12 {
            code_width += 1;
        }

        prev_code = Some(code);
    }

    Ok(output)
}

/// Dictionary table for LZW decompression.
struct DictTable {
    /// For codes 0-255: single byte.
    /// For codes 258+: (prefix_code, appended_byte).
    entries: Vec<(u16, u8)>,
    next_code: u16,
}

impl DictTable {
    fn new() -> Self {
        DictTable {
            entries: Vec::with_capacity(4096),
            next_code: FIRST_CODE,
        }
    }

    fn reset(&mut self) {
        self.entries.clear();
        self.next_code = FIRST_CODE;
    }

    fn get(&self, code: u16) -> Vec<u8> {
        if code < 256 {
            return vec![code as u8];
        }
        let idx = (code - FIRST_CODE) as usize;
        let (prefix, byte) = self.entries[idx];
        let mut result = self.get(prefix);
        result.push(byte);
        result
    }

    fn add(&mut self, prefix: u16, byte: u8) {
        self.entries.push((prefix, byte));
        self.next_code += 1;
    }
}

/// MSB-first bit reader for LZW.
struct BitReader<'a> {
    data: &'a [u8],
    byte_pos: usize,
    bit_pos: u8, // 0-7, counts from MSB (0 = MSB)
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        BitReader {
            data,
            byte_pos: 0,
            bit_pos: 0,
        }
    }

    fn read_bits(&mut self, n: u8) -> Result<u16> {
        let mut value: u16 = 0;
        for _ in 0..n {
            if self.byte_pos >= self.data.len() {
                return Err(Error::InvalidPdf(
                    "LZWDecode: unexpected end of data".into(),
                ));
            }
            let bit = (self.data[self.byte_pos] >> (7 - self.bit_pos)) & 1;
            value = (value << 1) | bit as u16;
            self.bit_pos += 1;
            if self.bit_pos == 8 {
                self.bit_pos = 0;
                self.byte_pos += 1;
            }
        }
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_simple() {
        // Manually encode "ABABAB" as LZW with 9-bit codes MSB-first:
        // Clear(256) A(65) B(66) 258(AB) 258(AB) EOD(257)
        let input = encode_lzw_codes(&[256, 65, 66, 258, 258, 257], true);
        let result = decode(&input, true).unwrap();
        assert_eq!(result, b"ABABAB");
    }

    #[test]
    fn decode_empty_stream() {
        let input = encode_lzw_codes(&[256, 257], true);
        let result = decode(&input, true).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn decode_single_byte() {
        let input = encode_lzw_codes(&[256, 88, 257], true);
        let result = decode(&input, true).unwrap();
        assert_eq!(result, b"X");
    }

    #[test]
    fn decode_repeated_byte() {
        // Clear(256) A(65) 258(AA) EOD(257)
        // After outputting A(65), dict adds nothing (no prev).
        // Next: 258. But 258 == next_code (special case: cScSc).
        // Entry = "A" + "A"[0] = "AA".
        // Dict now adds entry 258 = (65, 'A').
        let input = encode_lzw_codes(&[256, 65, 258, 257], true);
        let result = decode(&input, true).unwrap();
        assert_eq!(result, b"AAA");
    }

    #[test]
    fn decode_early_change_flag() {
        let input = encode_lzw_codes(&[256, 65, 66, 258, 258, 257], true);
        let result = decode(&input, true).unwrap();
        assert_eq!(result, b"ABABAB");
    }

    /// Helper: encode a sequence of LZW codes into bytes using MSB-first
    /// packing with the same code width logic as the decoder.
    fn encode_lzw_codes(codes: &[u16], early_change: bool) -> Vec<u8> {
        let mut bits = Vec::new();
        let mut code_width: u8 = 9;
        let mut next_code: u16 = FIRST_CODE;
        let mut prev_was_clear = false;
        let mut prev_code: Option<u16> = None;

        for &code in codes {
            // Write code using current width
            for bit_idx in (0..code_width).rev() {
                bits.push(((code >> bit_idx) & 1) as u8);
            }

            if code == CLEAR_CODE {
                next_code = FIRST_CODE;
                code_width = 9;
                prev_was_clear = true;
                prev_code = None;
                continue;
            }

            if code == EOD_CODE {
                break;
            }

            if !prev_was_clear && let Some(_prev) = prev_code {
                if next_code < MAX_CODE {
                    next_code += 1;
                }
                let threshold = if early_change {
                    next_code
                } else {
                    next_code - 1
                };
                if threshold >= (1 << code_width) && code_width < 12 {
                    code_width += 1;
                }
            }

            prev_was_clear = false;
            prev_code = Some(code);
        }

        // Pack bits into bytes
        let mut bytes = Vec::new();
        for chunk in bits.chunks(8) {
            let mut byte = 0u8;
            for (i, &bit) in chunk.iter().enumerate() {
                byte |= bit << (7 - i);
            }
            bytes.push(byte);
        }
        bytes
    }
}
