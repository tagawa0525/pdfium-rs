/// Predefined PDF font encodings.
///
/// Each encoding maps a single byte (0–255) to a Unicode code point.
/// `0xFFFF` means "undefined / not mapped".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PredefinedEncoding {
    WinAnsi,
    MacRoman,
    Standard,
    PdfDoc,
    MacExpert,
    Symbol,
    ZapfDingbats,
}

/// A custom encoding built from a base encoding plus a `/Differences` array.
///
/// Positions not listed in `overrides` fall back to `base`.
#[derive(Debug, Clone)]
pub struct CustomEncoding {
    pub base: PredefinedEncoding,
    /// `(char_code, unicode)` overrides from the `/Differences` array.
    pub overrides: Vec<(u8, char)>,
}

/// Map a single byte character code to a Unicode scalar using a predefined encoding.
///
/// Returns `None` if the code is not mapped (undefined slot).
pub fn unicode_from_predefined(enc: PredefinedEncoding, code: u8) -> Option<char> {
    let table: &[u32; 256] = match enc {
        PredefinedEncoding::WinAnsi => &WIN_ANSI,
        PredefinedEncoding::MacRoman => &MAC_ROMAN,
        PredefinedEncoding::Standard => &STANDARD,
        // Remaining encodings are not yet implemented; treat as unmapped.
        _ => return None,
    };
    let cp = table[code as usize];
    if cp == 0 { None } else { char::from_u32(cp) }
}

/// Map a single byte character code using a custom encoding (base + differences).
pub fn unicode_from_custom(enc: &CustomEncoding, code: u8) -> Option<char> {
    // Check overrides first
    if let Some(&(_, ch)) = enc.overrides.iter().find(|&&(c, _)| c == code) {
        return Some(ch);
    }
    unicode_from_predefined(enc.base, code)
}

// ---------------------------------------------------------------------------
// Static encoding tables
// ---------------------------------------------------------------------------
//
// Each table is `[u32; 256]` where each entry is the Unicode scalar value,
// or `0` for an undefined slot.  Using `u32` avoids the need for `char` in
// a const context while keeping arithmetic simple.

/// WinAnsiEncoding (Windows-1252).
///
/// The 0x80–0x9F range uses the Windows-1252 extensions (same as cp1252).
const WIN_ANSI: [u32; 256] = {
    let mut t = [0u32; 256];

    // 0x00–0x1F: control characters (undefined in PDF sense)
    // 0x20–0x7E: ASCII printable
    let mut i = 0x20u32;
    while i <= 0x7E {
        t[i as usize] = i;
        i += 1;
    }
    // 0x7F: undefined
    // 0x80–0x9F: Windows-1252 extensions
    t[0x80] = 0x20AC; // €
    // 0x81 undefined
    t[0x82] = 0x201A; // ‚
    t[0x83] = 0x0192; // ƒ
    t[0x84] = 0x201E; // „
    t[0x85] = 0x2026; // …
    t[0x86] = 0x2020; // †
    t[0x87] = 0x2021; // ‡
    t[0x88] = 0x02C6; // ˆ
    t[0x89] = 0x2030; // ‰
    t[0x8A] = 0x0160; // Š
    t[0x8B] = 0x2039; // ‹
    t[0x8C] = 0x0152; // Œ
    // 0x8D undefined
    t[0x8E] = 0x017D; // Ž
    // 0x8F undefined
    // 0x90 undefined
    t[0x91] = 0x2018; // '
    t[0x92] = 0x2019; // '
    t[0x93] = 0x201C; // "
    t[0x94] = 0x201D; // "
    t[0x95] = 0x2022; // •
    t[0x96] = 0x2013; // –
    t[0x97] = 0x2014; // —
    t[0x98] = 0x02DC; // ˜
    t[0x99] = 0x2122; // ™
    t[0x9A] = 0x0161; // š
    t[0x9B] = 0x203A; // ›
    t[0x9C] = 0x0153; // œ
    // 0x9D undefined
    t[0x9E] = 0x017E; // ž
    t[0x9F] = 0x0178; // Ÿ
    // 0xA0–0xFF: Latin-1 Supplement (same as ISO-8859-1)
    let mut i = 0xA0u32;
    while i <= 0xFF {
        t[i as usize] = i;
        i += 1;
    }
    t
};

/// Standard (Adobe StandardEncoding).
///
/// Derived from Adobe's StandardEncoding glyph list.
const STANDARD: [u32; 256] = {
    let mut t = [0u32; 256];
    // 0x20: space
    t[0x20] = 0x0020;
    t[0x21] = 0x0021; // exclam
    t[0x22] = 0x0022; // quotedbl
    t[0x23] = 0x0023; // numbersign
    t[0x24] = 0x0024; // dollar
    t[0x25] = 0x0025; // percent
    t[0x26] = 0x0026; // ampersand
    t[0x27] = 0x2019; // quoteright
    t[0x28] = 0x0028; // parenleft
    t[0x29] = 0x0029; // parenright
    t[0x2A] = 0x002A; // asterisk
    t[0x2B] = 0x002B; // plus
    t[0x2C] = 0x002C; // comma
    t[0x2D] = 0x002D; // hyphen
    t[0x2E] = 0x002E; // period
    t[0x2F] = 0x002F; // slash
    // 0x30–0x39: digits
    let mut i = 0x30u32;
    while i <= 0x39 {
        t[i as usize] = i;
        i += 1;
    }
    t[0x3A] = 0x003A; // colon
    t[0x3B] = 0x003B; // semicolon
    t[0x3C] = 0x003C; // less
    t[0x3D] = 0x003D; // equal
    t[0x3E] = 0x003E; // greater
    t[0x3F] = 0x003F; // question
    t[0x40] = 0x0040; // at
    // 0x41–0x5A: A–Z
    let mut i = 0x41u32;
    while i <= 0x5A {
        t[i as usize] = i;
        i += 1;
    }
    t[0x5B] = 0x005B; // bracketleft
    t[0x5C] = 0x005C; // backslash
    t[0x5D] = 0x005D; // bracketright
    t[0x5E] = 0x005E; // asciicircum
    t[0x5F] = 0x005F; // underscore
    t[0x60] = 0x2018; // quoteleft
    // 0x61–0x7A: a–z
    let mut i = 0x61u32;
    while i <= 0x7A {
        t[i as usize] = i;
        i += 1;
    }
    t[0x7B] = 0x007B; // braceleft
    t[0x7C] = 0x007C; // bar
    t[0x7D] = 0x007D; // braceright
    t[0x7E] = 0x007E; // asciitilde
    // 0xA1
    t[0xA1] = 0x00A1; // exclamdown
    t[0xA2] = 0x00A2; // cent
    t[0xA3] = 0x00A3; // sterling
    t[0xA4] = 0x2044; // fraction (not currency sign)
    t[0xA5] = 0x00A5; // yen
    t[0xA6] = 0x0192; // florin
    t[0xA7] = 0x00A7; // section
    t[0xA8] = 0x00A4; // currency
    t[0xA9] = 0x0027; // quotesingle
    t[0xAA] = 0x201C; // quotedblleft
    t[0xAB] = 0x00AB; // guillemotleft
    t[0xAC] = 0x2039; // guilsinglleft
    t[0xAD] = 0x203A; // guilsinglright
    t[0xAE] = 0xFB01; // fi
    t[0xAF] = 0xFB02; // fl
    t[0xB1] = 0x2013; // endash
    t[0xB2] = 0x2020; // dagger
    t[0xB3] = 0x2021; // daggerdbl
    t[0xB4] = 0x00B7; // periodcentered
    t[0xB6] = 0x00B6; // paragraph
    t[0xB7] = 0x2022; // bullet
    t[0xB8] = 0x201A; // quotesinglbase
    t[0xB9] = 0x201E; // quotedblbase
    t[0xBA] = 0x201D; // quotedblright
    t[0xBB] = 0x00BB; // guillemotright
    t[0xBC] = 0x2026; // ellipsis
    t[0xBD] = 0x2030; // perthousand
    t[0xBF] = 0x00BF; // questiondown
    t[0xC1] = 0x0060; // grave
    t[0xC2] = 0x00B4; // acute
    t[0xC3] = 0x02C6; // circumflex
    t[0xC4] = 0x02DC; // tilde
    t[0xC5] = 0x00AF; // macron
    t[0xC6] = 0x02D8; // breve
    t[0xC7] = 0x02D9; // dotaccent
    t[0xC8] = 0x00A8; // dieresis
    t[0xCA] = 0x02DA; // ring
    t[0xCB] = 0x00B8; // cedilla
    t[0xCD] = 0x02DD; // hungarumlaut
    t[0xCE] = 0x02DB; // ogonek
    t[0xCF] = 0x02C7; // caron
    t[0xD0] = 0x2014; // emdash
    t[0xE1] = 0x00C6; // AE
    t[0xE3] = 0x00AA; // ordfeminine
    t[0xE8] = 0x0141; // Lslash
    t[0xE9] = 0x00D8; // Oslash
    t[0xEA] = 0x0152; // OE
    t[0xEB] = 0x00BA; // ordmasculine
    t[0xF1] = 0x00E6; // ae
    t[0xF5] = 0x0131; // dotlessi
    t[0xF8] = 0x0142; // lslash
    t[0xF9] = 0x00F8; // oslash
    t[0xFA] = 0x0153; // oe
    t[0xFB] = 0x00DF; // germandbls
    t
};

/// MacRomanEncoding (Mac OS Roman).
const MAC_ROMAN: [u32; 256] = {
    let mut t = [0u32; 256];
    // 0x20–0x7E: same as ASCII
    let mut i = 0x20u32;
    while i <= 0x7E {
        t[i as usize] = i;
        i += 1;
    }
    // 0x80–0xFF: Mac OS Roman upper half
    t[0x80] = 0x00C4; // Ä
    t[0x81] = 0x00C5; // Å
    t[0x82] = 0x00C7; // Ç
    t[0x83] = 0x00C9; // É
    t[0x84] = 0x00D1; // Ñ
    t[0x85] = 0x00D6; // Ö
    t[0x86] = 0x00DC; // Ü
    t[0x87] = 0x00E1; // á
    t[0x88] = 0x00E0; // à
    t[0x89] = 0x00E2; // â
    t[0x8A] = 0x00E4; // ä
    t[0x8B] = 0x00E5; // å
    t[0x8C] = 0x00E7; // ç
    t[0x8D] = 0x00E9; // é
    t[0x8E] = 0x00E8; // è
    t[0x8F] = 0x00EA; // ê
    t[0x90] = 0x00EB; // ë
    t[0x91] = 0x00ED; // í
    t[0x92] = 0x00EC; // ì
    t[0x93] = 0x00EE; // î
    t[0x94] = 0x00EF; // ï
    t[0x95] = 0x00F1; // ñ
    t[0x96] = 0x00F3; // ó
    t[0x97] = 0x00F2; // ò
    t[0x98] = 0x00F4; // ô
    t[0x99] = 0x00F6; // ö
    t[0x9A] = 0x00FA; // ú
    t[0x9B] = 0x00F9; // ù
    t[0x9C] = 0x00FB; // û
    t[0x9D] = 0x00FC; // ü
    t[0x9E] = 0x2020; // †
    t[0x9F] = 0x00B0; // °
    t[0xA0] = 0x00A2; // ¢
    t[0xA1] = 0x00A3; // £
    t[0xA2] = 0x00A7; // §
    t[0xA3] = 0x2022; // •
    t[0xA4] = 0x00B6; // ¶
    t[0xA5] = 0x00DF; // ß
    t[0xA6] = 0x00AE; // ®
    t[0xA7] = 0x00A9; // ©
    t[0xA8] = 0x2122; // ™
    t[0xA9] = 0x00B4; // ´
    t[0xAA] = 0x00A8; // ¨
    t[0xAB] = 0x2260; // ≠
    t[0xAC] = 0x00C6; // Æ
    t[0xAD] = 0x00D8; // Ø
    t[0xAE] = 0x221E; // ∞
    t[0xAF] = 0x00B1; // ±
    t[0xB0] = 0x2264; // ≤
    t[0xB1] = 0x2265; // ≥
    t[0xB2] = 0x00A5; // ¥
    t[0xB3] = 0x00B5; // µ
    t[0xB4] = 0x2202; // ∂
    t[0xB5] = 0x2211; // ∑
    t[0xB6] = 0x220F; // ∏
    t[0xB7] = 0x03C0; // π
    t[0xB8] = 0x222B; // ∫
    t[0xB9] = 0x00AA; // ª
    t[0xBA] = 0x00BA; // º
    t[0xBB] = 0x03A9; // Ω
    t[0xBC] = 0x00E6; // æ
    t[0xBD] = 0x00F8; // ø
    t[0xBE] = 0x00BF; // ¿
    t[0xBF] = 0x00A1; // ¡
    t[0xC0] = 0x00AC; // ¬
    t[0xC1] = 0x221A; // √
    t[0xC2] = 0x0192; // ƒ
    t[0xC3] = 0x2248; // ≈
    t[0xC4] = 0x2206; // ∆
    t[0xC5] = 0x00AB; // «
    t[0xC6] = 0x00BB; // »
    t[0xC7] = 0x2026; // …
    t[0xC8] = 0x00A0; // NBSP
    t[0xC9] = 0x00C0; // À
    t[0xCA] = 0x00C3; // Ã
    t[0xCB] = 0x00D5; // Õ
    t[0xCC] = 0x0152; // Œ
    t[0xCD] = 0x0153; // œ
    t[0xCE] = 0x2013; // –
    t[0xCF] = 0x2014; // —
    t[0xD0] = 0x201C; // "
    t[0xD1] = 0x201D; // "
    t[0xD2] = 0x2018; // '
    t[0xD3] = 0x2019; // '
    t[0xD4] = 0x00F7; // ÷
    t[0xD5] = 0x25CA; // ◊
    t[0xD6] = 0x00FF; // ÿ
    t[0xD7] = 0x0178; // Ÿ
    t[0xD8] = 0x2044; // ⁄
    t[0xD9] = 0x20AC; // €
    t[0xDA] = 0x2039; // ‹
    t[0xDB] = 0x203A; // ›
    t[0xDC] = 0xFB01; // ﬁ
    t[0xDD] = 0xFB02; // ﬂ
    t[0xDE] = 0x2021; // ‡
    t[0xDF] = 0x00B7; // ·
    t[0xE0] = 0x201A; // ‚
    t[0xE1] = 0x201E; // „
    t[0xE2] = 0x2030; // ‰
    t[0xE3] = 0x00C2; // Â
    t[0xE4] = 0x00CA; // Ê
    t[0xE5] = 0x00C1; // Á
    t[0xE6] = 0x00CB; // Ë
    t[0xE7] = 0x00C8; // È
    t[0xE8] = 0x00CD; // Í
    t[0xE9] = 0x00CE; // Î
    t[0xEA] = 0x00CF; // Ï
    t[0xEB] = 0x00CC; // Ì
    t[0xEC] = 0x00D3; // Ó
    t[0xED] = 0x00D4; // Ô
    t[0xEE] = 0xF8FF; // Apple logo (private use)
    t[0xEF] = 0x00D2; // Ò
    t[0xF0] = 0x00DA; // Ú
    t[0xF1] = 0x00DB; // Û
    t[0xF2] = 0x00D9; // Ù
    t[0xF3] = 0x0131; // ı
    t[0xF4] = 0x02C6; // ˆ
    t[0xF5] = 0x02DC; // ˜
    t[0xF6] = 0x00AF; // ¯
    t[0xF7] = 0x02D8; // ˘
    t[0xF8] = 0x02D9; // ˙
    t[0xF9] = 0x02DA; // ˚
    t[0xFA] = 0x00B8; // ¸
    t[0xFB] = 0x02DD; // ˝
    t[0xFC] = 0x02DB; // ˛
    t[0xFD] = 0x02C7; // ˇ
    t
};

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- unicode_from_predefined ---

    #[test]
    fn winansi_ascii_printable_maps_to_itself() {
        // ASCII printable range 0x20–0x7E must map to themselves in WinAnsi
        for code in 0x20u8..=0x7E {
            let ch = unicode_from_predefined(PredefinedEncoding::WinAnsi, code);
            assert_eq!(
                ch,
                char::from_u32(code as u32),
                "WinAnsi 0x{code:02X} should map to U+{code:04X}"
            );
        }
    }

    #[test]
    fn winansi_euro_sign_at_0x80() {
        let ch = unicode_from_predefined(PredefinedEncoding::WinAnsi, 0x80);
        assert_eq!(ch, Some('€'));
    }

    #[test]
    fn winansi_latin1_supplement_at_0xa0() {
        // 0xA0 = NO-BREAK SPACE in both WinAnsi and Latin-1
        let ch = unicode_from_predefined(PredefinedEncoding::WinAnsi, 0xA0);
        assert_eq!(ch, Some('\u{00A0}'));
    }

    #[test]
    fn winansi_undefined_slot_returns_none() {
        // 0x81 is undefined in WinAnsi / cp1252
        let ch = unicode_from_predefined(PredefinedEncoding::WinAnsi, 0x81);
        assert_eq!(ch, None);
    }

    #[test]
    fn standard_quoteright_at_0x27() {
        // StandardEncoding maps 0x27 to RIGHT SINGLE QUOTATION MARK (U+2019)
        // not ASCII apostrophe
        let ch = unicode_from_predefined(PredefinedEncoding::Standard, 0x27);
        assert_eq!(ch, Some('\u{2019}'));
    }

    #[test]
    fn standard_digits_map_to_themselves() {
        for code in 0x30u8..=0x39 {
            let ch = unicode_from_predefined(PredefinedEncoding::Standard, code);
            assert_eq!(ch, char::from_u32(code as u32));
        }
    }

    #[test]
    fn standard_undefined_slot_returns_none() {
        // 0x80 is undefined in StandardEncoding
        let ch = unicode_from_predefined(PredefinedEncoding::Standard, 0x80);
        assert_eq!(ch, None);
    }

    #[test]
    fn macroman_a_umlaut_at_0x80() {
        let ch = unicode_from_predefined(PredefinedEncoding::MacRoman, 0x80);
        assert_eq!(ch, Some('Ä'));
    }

    // --- unicode_from_custom ---

    #[test]
    fn custom_encoding_override_replaces_base() {
        let enc = CustomEncoding {
            base: PredefinedEncoding::WinAnsi,
            overrides: vec![(0x41, 'Á')], // A → Á
        };
        assert_eq!(unicode_from_custom(&enc, 0x41), Some('Á'));
    }

    #[test]
    fn custom_encoding_falls_back_to_base() {
        let enc = CustomEncoding {
            base: PredefinedEncoding::WinAnsi,
            overrides: vec![(0x41, 'Á')],
        };
        // 0x42 = B — not overridden, falls back to WinAnsi
        assert_eq!(unicode_from_custom(&enc, 0x42), Some('B'));
    }

    #[test]
    fn custom_encoding_undefined_base_slot_returns_none() {
        let enc = CustomEncoding {
            base: PredefinedEncoding::WinAnsi,
            overrides: vec![],
        };
        // 0x81 is undefined in WinAnsi
        assert_eq!(unicode_from_custom(&enc, 0x81), None);
    }
}
