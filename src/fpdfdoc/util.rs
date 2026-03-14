/// Decode a PDF text string to a Rust `String`.
///
/// PDF text strings are either:
/// - UTF-16BE with a `\xFE\xFF` BOM, or
/// - PDFDocEncoding (Latin-1 compatible, with Windows-1252 for 0x80-0x9F)
pub(super) fn decode_pdf_text_string(bytes: &[u8]) -> String {
    let raw: String = if bytes.starts_with(b"\xfe\xff") {
        // UTF-16BE with BOM
        let pairs = bytes[2..].chunks_exact(2);
        pairs
            .filter_map(|p| {
                let cp = u16::from_be_bytes([p[0], p[1]]);
                char::from_u32(cp as u32)
            })
            .collect()
    } else {
        // PDFDocEncoding: identical to Latin-1 for 0x20-0x7E and 0xA0-0xFF;
        // 0x80-0x9F are undefined in the spec; use Windows-1252 for the common
        // printable subset.
        bytes.iter().map(|&b| pdf_doc_encoding_char(b)).collect()
    };

    // Replace control characters with space (C++ PDFium compat)
    raw.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect()
}

/// Map a single PDFDocEncoding byte to a `char`.
///
/// 0x00-0x7E and 0xA0-0xFF equal their Unicode code points.
/// 0x80-0x9F use the Windows-1252 map (covers euro, smart-quotes, em-dash, …).
fn pdf_doc_encoding_char(b: u8) -> char {
    match b {
        0x80 => '\u{20AC}', // €
        0x82 => '\u{201A}', // ‚
        0x83 => '\u{0192}', // ƒ
        0x84 => '\u{201E}', // „
        0x85 => '\u{2026}', // …
        0x86 => '\u{2020}', // †
        0x87 => '\u{2021}', // ‡
        0x88 => '\u{02C6}', // ˆ
        0x89 => '\u{2030}', // ‰
        0x8A => '\u{0160}', // Š
        0x8B => '\u{2039}', // ‹
        0x8C => '\u{0152}', // Œ
        0x8E => '\u{017D}', // Ž
        0x91 => '\u{2018}', // '
        0x92 => '\u{2019}', // '
        0x93 => '\u{201C}', // "
        0x94 => '\u{201D}', // "
        0x95 => '\u{2022}', // •
        0x96 => '\u{2013}', // –
        0x97 => '\u{2014}', // —
        0x98 => '\u{02DC}', // ˜
        0x99 => '\u{2122}', // ™
        0x9A => '\u{0161}', // š
        0x9B => '\u{203A}', // ›
        0x9C => '\u{0153}', // œ
        0x9E => '\u{017E}', // ž
        0x9F => '\u{0178}', // Ÿ
        _ => b as char,     // Latin-1 passthrough
    }
}
