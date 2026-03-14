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
        todo!()
    }
}

// ---------------------------------------------------------------------------
// Internal parser helpers
// ---------------------------------------------------------------------------

/// Parse a hex string like `<0041>` into a u32 code point value.
fn parse_hex_u32(s: &[u8]) -> Option<u32> {
    todo!()
}

/// Parse a UTF-16BE hex sequence like `<00410042>` into a `String`.
fn parse_utf16be_hex(s: &[u8]) -> Option<String> {
    todo!()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- parse_hex_u32 ---

    #[test]
    #[ignore = "not yet implemented"]
    fn hex_u32_four_digits() {
        assert_eq!(parse_hex_u32(b"0041"), Some(0x0041));
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn hex_u32_two_digits() {
        assert_eq!(parse_hex_u32(b"41"), Some(0x41));
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn hex_u32_invalid_returns_none() {
        assert_eq!(parse_hex_u32(b"ZZZZ"), None);
    }

    // --- parse_utf16be_hex ---

    #[test]
    #[ignore = "not yet implemented"]
    fn utf16be_single_bmp_char() {
        // <0041> → 'A'
        assert_eq!(parse_utf16be_hex(b"0041"), Some("A".to_string()));
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn utf16be_two_bmp_chars() {
        // <00410042> → "AB"
        assert_eq!(parse_utf16be_hex(b"00410042"), Some("AB".to_string()));
    }

    #[test]
    #[ignore = "not yet implemented"]
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
    #[ignore = "not yet implemented"]
    fn parse_bfchar_maps_space_and_a() {
        let map = ToUnicodeMap::parse(simple_bfchar_cmap());
        assert_eq!(map.lookup(0x20), Some(" "));
        assert_eq!(map.lookup(0x41), Some("A"));
        assert_eq!(map.lookup(0x42), None);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn parse_bfrange_sequential() {
        // <41>–<43> mapped to U+0041 upward → A, B, C
        let map = ToUnicodeMap::parse(bfrange_cmap());
        assert_eq!(map.lookup(0x41), Some("A"));
        assert_eq!(map.lookup(0x42), Some("B"));
        assert_eq!(map.lookup(0x43), Some("C"));
        assert_eq!(map.lookup(0x44), None);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn parse_bfrange_array_form() {
        // Array form: each entry in [..] maps to the corresponding code
        let map = ToUnicodeMap::parse(bfrange_array_cmap());
        assert_eq!(map.lookup(0x41), Some("a"));
        assert_eq!(map.lookup(0x42), Some("b"));
        assert_eq!(map.lookup(0x43), Some("c"));
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn lookup_unmapped_code_returns_none() {
        let map = ToUnicodeMap::parse(simple_bfchar_cmap());
        assert_eq!(map.lookup(0xFF), None);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn parse_empty_data_returns_empty_map() {
        let map = ToUnicodeMap::parse(b"");
        assert_eq!(map.lookup(0x41), None);
    }
}
