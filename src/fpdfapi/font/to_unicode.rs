use std::collections::HashMap;

/// Parsed ToUnicode CMap.
///
/// Maps a character code (`u32`) to a Unicode string (usually a single character,
/// but may be a multi-character sequence for ligatures).
#[derive(Debug, Clone, Default)]
pub struct ToUnicodeMap {
    map: HashMap<u32, String>,
}

impl ToUnicodeMap {
    /// Look up a character code. Returns `None` if not mapped.
    pub fn lookup(&self, char_code: u32) -> Option<&str> {
        self.map.get(&char_code).map(|s| s.as_str())
    }

    /// Parse a ToUnicode CMap stream (`beginbfchar`/`beginbfrange` sections).
    pub fn parse(data: &[u8]) -> Self {
        let mut map = HashMap::new();
        // Search for keyword markers as substrings (handles "N beginbfchar" lines)
        let mut pos = 0;
        while pos < data.len() {
            if let Some(off) = find_keyword(data, pos, b"beginbfchar") {
                let bfrange_off = find_keyword(data, pos, b"beginbfrange");
                // Only handle beginbfchar if it comes before the next beginbfrange
                if bfrange_off.is_none_or(|r| off < r) {
                    pos = off + b"beginbfchar".len();
                    parse_bfchar(data, &mut pos, &mut map);
                    continue;
                }
            }
            if let Some(off) = find_keyword(data, pos, b"beginbfrange") {
                pos = off + b"beginbfrange".len();
                parse_bfrange(data, &mut pos, &mut map);
                continue;
            }
            break;
        }
        ToUnicodeMap { map }
    }
}

// ---------------------------------------------------------------------------
// Internal parser helpers
// ---------------------------------------------------------------------------

fn hex_nibble(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn utf16be_bytes_to_string(bytes: &[u8]) -> Option<String> {
    if !bytes.len().is_multiple_of(2) {
        return None;
    }
    let units: Vec<u16> = bytes
        .chunks(2)
        .map(|c| u16::from_be_bytes([c[0], c[1]]))
        .collect();
    String::from_utf16(&units).ok()
}

/// Decode UTF-16BE bytes to a single Unicode scalar (for sequential bfrange).
fn utf16be_bytes_to_codepoint(bytes: &[u8]) -> Option<u32> {
    if !bytes.len().is_multiple_of(2) {
        return None;
    }
    let units: Vec<u16> = bytes
        .chunks(2)
        .map(|c| u16::from_be_bytes([c[0], c[1]]))
        .collect();
    // For simplicity, only handle single BMP code unit
    if units.len() == 1 {
        Some(units[0] as u32)
    } else {
        None
    }
}

fn bytes_to_u32(bytes: &[u8]) -> u32 {
    let mut val: u32 = 0;
    for &b in bytes {
        val = (val << 8) | (b as u32);
    }
    val
}

/// Find byte offset of keyword in data starting from `from`, or `None`.
fn find_keyword(data: &[u8], from: usize, keyword: &[u8]) -> Option<usize> {
    data[from..]
        .windows(keyword.len())
        .position(|w| w == keyword)
        .map(|p| p + from)
}

/// Parse `beginbfchar` … `endbfchar` section, populating `map`.
fn parse_bfchar(data: &[u8], pos: &mut usize, map: &mut HashMap<u32, String>) {
    loop {
        skip_whitespace(data, pos);
        if *pos >= data.len() || data[*pos..].starts_with(b"endbfchar") {
            if data[*pos..].starts_with(b"endbfchar") {
                *pos += b"endbfchar".len();
            }
            break;
        }
        let Some(src) = read_hex_token(data, pos) else {
            break;
        };
        skip_whitespace(data, pos);
        let Some(dst_bytes) = read_hex_bytes(data, pos) else {
            break;
        };
        let Some(dst) = utf16be_bytes_to_string(&dst_bytes) else {
            continue;
        };
        map.insert(bytes_to_u32(&src), dst);
    }
}

/// Parse `beginbfrange` … `endbfrange` section, populating `map`.
fn parse_bfrange(data: &[u8], pos: &mut usize, map: &mut HashMap<u32, String>) {
    loop {
        skip_whitespace(data, pos);
        if *pos >= data.len() || data[*pos..].starts_with(b"endbfrange") {
            if data[*pos..].starts_with(b"endbfrange") {
                *pos += b"endbfrange".len();
            }
            break;
        }
        let Some(lo_bytes) = read_hex_token(data, pos) else {
            break;
        };
        skip_whitespace(data, pos);
        let Some(hi_bytes) = read_hex_token(data, pos) else {
            break;
        };
        skip_whitespace(data, pos);

        let lo = bytes_to_u32(&lo_bytes);
        let hi = bytes_to_u32(&hi_bytes);

        if *pos < data.len() && data[*pos] == b'[' {
            *pos += 1;
            for code in lo..=hi {
                skip_whitespace(data, pos);
                if *pos < data.len() && data[*pos] == b']' {
                    break;
                }
                let Some(dst_bytes) = read_hex_bytes(data, pos) else {
                    break;
                };
                let Some(dst) = utf16be_bytes_to_string(&dst_bytes) else {
                    continue;
                };
                map.insert(code, dst);
            }
            skip_whitespace(data, pos);
            if *pos < data.len() && data[*pos] == b']' {
                *pos += 1;
            }
        } else {
            let Some(base_bytes) = read_hex_bytes(data, pos) else {
                break;
            };
            let Some(base_cp) = utf16be_bytes_to_codepoint(&base_bytes) else {
                // Non-BMP / surrogate pair: skip this range but continue parsing
                continue;
            };
            for (i, code) in (lo..=hi).enumerate() {
                if let Some(ch) = char::from_u32(base_cp + i as u32) {
                    map.insert(code, ch.to_string());
                }
            }
        }
    }
}

fn skip_whitespace(data: &[u8], pos: &mut usize) {
    loop {
        // Skip ASCII whitespace
        while *pos < data.len() && data[*pos].is_ascii_whitespace() {
            *pos += 1;
        }
        // Skip PostScript/PDF % comments through to end of line
        if *pos < data.len() && data[*pos] == b'%' {
            while *pos < data.len() && data[*pos] != b'\n' {
                *pos += 1;
            }
        } else {
            break;
        }
    }
}

/// Read a `<hexhex...>` token, hex-decoding the contents and returning the decoded bytes.
///
/// Conforms to PDF spec §7.3.4.3:
/// - Whitespace inside `<…>` is ignored.
/// - An odd number of hex digits is allowed; the last nibble is padded with 0.
fn read_hex_token(data: &[u8], pos: &mut usize) -> Option<Vec<u8>> {
    if *pos >= data.len() || data[*pos] != b'<' {
        return None;
    }
    *pos += 1; // skip '<'

    // Collect non-whitespace hex nibbles until '>'
    let mut nibbles: Vec<u8> = Vec::new();
    while *pos < data.len() && data[*pos] != b'>' {
        let b = data[*pos];
        *pos += 1;
        if b.is_ascii_whitespace() {
            continue;
        }
        nibbles.push(hex_nibble(b)?);
    }
    if *pos < data.len() {
        *pos += 1; // skip '>'
    }

    // If odd number of nibbles, pad with a trailing 0
    if nibbles.len() % 2 == 1 {
        nibbles.push(0);
    }

    let bytes = nibbles.chunks(2).map(|c| (c[0] << 4) | c[1]).collect();
    Some(bytes)
}

/// Read a `<hexhex...>` token, returning raw decoded bytes.
fn read_hex_bytes(data: &[u8], pos: &mut usize) -> Option<Vec<u8>> {
    read_hex_token(data, pos)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // Test-only helpers

    /// Parse a hex string like `0041` into a u32 code point value.
    fn parse_hex_u32(s: &[u8]) -> Option<u32> {
        let mut val: u32 = 0;
        for &b in s {
            let nibble = match b {
                b'0'..=b'9' => b - b'0',
                b'a'..=b'f' => b - b'a' + 10,
                b'A'..=b'F' => b - b'A' + 10,
                _ => return None,
            };
            val = val.checked_shl(4)?.checked_add(nibble as u32)?;
        }
        Some(val)
    }

    /// Parse a UTF-16BE hex sequence like `00410042` into a `String`.
    fn parse_utf16be_hex(s: &[u8]) -> Option<String> {
        if !s.len().is_multiple_of(2) {
            return None;
        }
        let mut bytes = Vec::with_capacity(s.len() / 2);
        for chunk in s.chunks(2) {
            let hi = hex_nibble(chunk[0])?;
            let lo = hex_nibble(chunk[1])?;
            bytes.push((hi << 4) | lo);
        }
        utf16be_bytes_to_string(&bytes)
    }

    // --- parse_hex_u32 ---

    #[test]
    fn hex_u32_four_digits() {
        assert_eq!(parse_hex_u32(b"0041"), Some(0x0041));
    }

    #[test]
    fn hex_u32_two_digits() {
        assert_eq!(parse_hex_u32(b"41"), Some(0x41));
    }

    #[test]
    fn hex_u32_invalid_returns_none() {
        assert_eq!(parse_hex_u32(b"ZZZZ"), None);
    }

    // --- parse_utf16be_hex ---

    #[test]
    fn utf16be_single_bmp_char() {
        // <0041> → 'A'
        assert_eq!(parse_utf16be_hex(b"0041"), Some("A".to_string()));
    }

    #[test]
    fn utf16be_two_bmp_chars() {
        // <00410042> → "AB"
        assert_eq!(parse_utf16be_hex(b"00410042"), Some("AB".to_string()));
    }

    #[test]
    fn utf16be_invalid_returns_none() {
        assert_eq!(parse_utf16be_hex(b"ZZZZ"), None);
    }

    // --- ToUnicodeMap::parse ---

    fn simple_bfchar_cmap() -> &'static [u8] {
        b"/CIDInit /ProcSet findresource begin\n\
          12 dict begin\n\
          begincmap\n\
          /CIDSystemInfo 3 dict dup begin\n\
          end def\n\
          /CMapName /Adobe-Identity-UCS def\n\
          /CMapType 2 def\n\
          2 beginbfchar\n\
          <20> <0020>\n\
          <41> <0041>\n\
          endbfchar\n\
          endcmap\n\
          CMapName currentdict /CMap defineresource pop\n\
          end\n\
          end\n"
    }

    fn bfrange_cmap() -> &'static [u8] {
        b"begincmap\n\
          1 beginbfrange\n\
          <41> <43> <0041>\n\
          endbfrange\n\
          endcmap\n"
    }

    fn bfrange_array_cmap() -> &'static [u8] {
        b"begincmap\n\
          1 beginbfrange\n\
          <41> <43> [<0061> <0062> <0063>]\n\
          endbfrange\n\
          endcmap\n"
    }

    #[test]
    fn parse_bfchar_maps_space_and_a() {
        let map = ToUnicodeMap::parse(simple_bfchar_cmap());
        assert_eq!(map.lookup(0x20), Some(" "));
        assert_eq!(map.lookup(0x41), Some("A"));
        assert_eq!(map.lookup(0x42), None);
    }

    #[test]
    fn parse_bfrange_sequential() {
        // <41>–<43> mapped to U+0041 upward → A, B, C
        let map = ToUnicodeMap::parse(bfrange_cmap());
        assert_eq!(map.lookup(0x41), Some("A"));
        assert_eq!(map.lookup(0x42), Some("B"));
        assert_eq!(map.lookup(0x43), Some("C"));
        assert_eq!(map.lookup(0x44), None);
    }

    #[test]
    fn parse_bfrange_array_form() {
        // Array form: each entry in [..] maps to the corresponding code
        let map = ToUnicodeMap::parse(bfrange_array_cmap());
        assert_eq!(map.lookup(0x41), Some("a"));
        assert_eq!(map.lookup(0x42), Some("b"));
        assert_eq!(map.lookup(0x43), Some("c"));
    }

    #[test]
    fn lookup_unmapped_code_returns_none() {
        let map = ToUnicodeMap::parse(simple_bfchar_cmap());
        assert_eq!(map.lookup(0xFF), None);
    }

    #[test]
    fn parse_empty_data_returns_empty_map() {
        let map = ToUnicodeMap::parse(b"");
        assert_eq!(map.lookup(0x41), None);
    }
}
