use std::io::{Read, Seek};

use crate::error::Result;
use crate::fpdfapi::font::encoding::{CustomEncoding, FontEncoding, PredefinedEncoding};
use crate::fpdfapi::font::to_unicode::ToUnicodeMap;
use crate::fpdfapi::parser::document::Document;
use crate::fpdfapi::parser::object::{PdfDictionary, PdfObject};

/// A PDF font, simplified to the two variants needed for Phase 3.
#[derive(Debug, Clone)]
pub enum PdfFont {
    /// Simple font (Type1, TrueType, MMType1) — single-byte character codes.
    Simple {
        base_font: String,
        encoding: FontEncoding,
        /// Index of the first character code covered by `widths`.
        first_char: u32,
        /// Character widths in 1/1000 of text-space units, starting at `first_char`.
        widths: Vec<u16>,
        to_unicode: Option<ToUnicodeMap>,
    },
    /// CIDFont (Type0), Type3, or any subtype not yet supported.
    Unsupported { base_font: String },
}

impl PdfFont {
    /// Load a `PdfFont` from a font dictionary.
    ///
    /// `doc` is needed to resolve `/ToUnicode` stream references.
    pub fn load<R: Read + Seek>(
        font_dict: &PdfDictionary,
        doc: &mut Document<R>,
    ) -> Result<PdfFont> {
        let base_font = base_font_name(font_dict);
        let subtype = font_dict
            .get_name(b"Subtype")
            .and_then(|s| s.as_str().map(|s| s.to_string()))
            .unwrap_or_default();

        if !matches!(subtype.as_str(), "Type1" | "TrueType" | "MMType1") {
            return Ok(PdfFont::Unsupported { base_font });
        }

        let encoding = parse_encoding(font_dict);
        let first_char = font_dict.get_i32(b"FirstChar").unwrap_or(0).max(0) as u32;
        let widths = parse_widths(font_dict);

        // Resolve /ToUnicode stream (clone to release borrow before decode)
        let to_unicode = if let Some(tu_ref) = font_dict.get_reference(b"ToUnicode") {
            let stream_opt = doc.object(tu_ref.num)?.as_stream().cloned();
            if let Some(stream) = stream_opt {
                let data = doc.decode_stream(&stream, tu_ref.num, tu_ref.gen_num)?;
                Some(ToUnicodeMap::parse(&data))
            } else {
                None
            }
        } else {
            None
        };

        Ok(PdfFont::Simple {
            base_font,
            encoding,
            first_char,
            widths,
            to_unicode,
        })
    }

    /// Convert a single-byte character code to a Unicode string.
    ///
    /// `/ToUnicode` is consulted first; falls back to the encoding table.
    /// Returns `None` if no mapping exists.
    pub fn unicode_from_char_code(&self, code: u32) -> Option<String> {
        match self {
            PdfFont::Simple {
                encoding,
                to_unicode,
                ..
            } => {
                // ToUnicode takes priority
                if let Some(map) = to_unicode
                    && let Some(s) = map.lookup(code)
                {
                    return Some(s.to_string());
                }
                // Fall back to encoding table
                if code <= 0xFF {
                    encoding.decode(code as u8).map(|ch| ch.to_string())
                } else {
                    None
                }
            }
            PdfFont::Unsupported { .. } => None,
        }
    }

    /// Return the advance width of a character in 1/1000 text-space units.
    ///
    /// Returns `1000.0` as the default when the code is out of the widths array range
    /// or for unsupported fonts.
    pub fn char_width(&self, code: u32) -> f64 {
        match self {
            PdfFont::Simple {
                first_char, widths, ..
            } => {
                if code >= *first_char {
                    let idx = (code - first_char) as usize;
                    if idx < widths.len() {
                        return widths[idx] as f64;
                    }
                }
                1000.0
            }
            PdfFont::Unsupported { .. } => 1000.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Parse the glyph name → char mapping for `/Differences` entries.
///
/// Uses a minimal subset of the Adobe Glyph List sufficient for Latin text.
fn glyph_name_to_char(name: &[u8]) -> Option<char> {
    // Single-byte ASCII names map directly
    if name.len() == 1 && name[0].is_ascii_alphanumeric() {
        return char::from_u32(name[0] as u32);
    }
    // Common Adobe Glyph List entries for Latin PDFs
    match name {
        b"space" => Some(' '),
        b"exclam" => Some('!'),
        b"quotedbl" => Some('"'),
        b"numbersign" => Some('#'),
        b"dollar" => Some('$'),
        b"percent" => Some('%'),
        b"ampersand" => Some('&'),
        b"quoteright" => Some('\u{2019}'),
        b"quoteleft" => Some('\u{2018}'),
        b"parenleft" => Some('('),
        b"parenright" => Some(')'),
        b"asterisk" => Some('*'),
        b"plus" => Some('+'),
        b"comma" => Some(','),
        b"hyphen" => Some('-'),
        b"minus" => Some('\u{2212}'),
        b"period" => Some('.'),
        b"slash" => Some('/'),
        b"colon" => Some(':'),
        b"semicolon" => Some(';'),
        b"less" => Some('<'),
        b"equal" => Some('='),
        b"greater" => Some('>'),
        b"question" => Some('?'),
        b"at" => Some('@'),
        b"bracketleft" => Some('['),
        b"backslash" => Some('\\'),
        b"bracketright" => Some(']'),
        b"asciicircum" => Some('^'),
        b"underscore" => Some('_'),
        b"braceleft" => Some('{'),
        b"bar" => Some('|'),
        b"braceright" => Some('}'),
        b"asciitilde" => Some('~'),
        b"emdash" => Some('\u{2014}'),
        b"endash" => Some('\u{2013}'),
        b"bullet" => Some('\u{2022}'),
        b"ellipsis" => Some('\u{2026}'),
        b"quotedblleft" => Some('\u{201C}'),
        b"quotedblright" => Some('\u{201D}'),
        b"quotedblbase" => Some('\u{201E}'),
        b"quotesinglbase" => Some('\u{201A}'),
        b"dagger" => Some('\u{2020}'),
        b"daggerdbl" => Some('\u{2021}'),
        b"fi" => Some('\u{FB01}'),
        b"fl" => Some('\u{FB02}'),
        b"florin" => Some('\u{0192}'),
        b"fraction" => Some('\u{2044}'),
        b"guilsinglleft" => Some('\u{2039}'),
        b"guilsinglright" => Some('\u{203A}'),
        b"perthousand" => Some('\u{2030}'),
        b"trademark" => Some('\u{2122}'),
        b"Euro" | b"euro" => Some('\u{20AC}'),
        b"AE" => Some('Æ'),
        b"ae" => Some('æ'),
        b"OE" => Some('Œ'),
        b"oe" => Some('œ'),
        b"Oslash" => Some('Ø'),
        b"oslash" => Some('ø'),
        b"germandbls" => Some('ß'),
        b"dotlessi" => Some('\u{0131}'),
        b"Lslash" => Some('\u{0141}'),
        b"lslash" => Some('\u{0142}'),
        b"grave" => Some('`'),
        b"acute" => Some('\u{00B4}'),
        b"circumflex" => Some('^'),
        b"tilde" => Some('~'),
        b"macron" => Some('\u{00AF}'),
        b"breve" => Some('\u{02D8}'),
        b"dotaccent" => Some('\u{02D9}'),
        b"dieresis" => Some('\u{00A8}'),
        b"ring" => Some('\u{02DA}'),
        b"cedilla" => Some('\u{00B8}'),
        b"caron" => Some('\u{02C7}'),
        b"hungarumlaut" => Some('\u{02DD}'),
        b"ogonek" => Some('\u{02DB}'),
        _ => None,
    }
}

/// Parse the `/Encoding` entry of a font dictionary.
fn parse_encoding(font_dict: &PdfDictionary) -> FontEncoding {
    match font_dict.get(b"Encoding") {
        Some(PdfObject::Name(name)) => {
            let enc = match name.as_bytes() {
                b"WinAnsiEncoding" => PredefinedEncoding::WinAnsi,
                b"MacRomanEncoding" => PredefinedEncoding::MacRoman,
                b"StandardEncoding" => PredefinedEncoding::Standard,
                b"MacExpertEncoding" => PredefinedEncoding::MacExpert,
                b"SymbolEncoding" => PredefinedEncoding::Symbol,
                b"ZapfDingbatsEncoding" => PredefinedEncoding::ZapfDingbats,
                _ => PredefinedEncoding::Standard, // unknown name → Standard
            };
            FontEncoding::Predefined(enc)
        }
        Some(PdfObject::Dictionary(enc_dict)) => {
            let base = enc_dict
                .get_name(b"BaseEncoding")
                .and_then(|n| match n.as_bytes() {
                    b"WinAnsiEncoding" => Some(PredefinedEncoding::WinAnsi),
                    b"MacRomanEncoding" => Some(PredefinedEncoding::MacRoman),
                    b"StandardEncoding" => Some(PredefinedEncoding::Standard),
                    _ => None,
                })
                .unwrap_or(PredefinedEncoding::Standard);

            let mut overrides: Vec<(u8, char)> = Vec::new();
            if let Some(diffs) = enc_dict.get_array(b"Differences") {
                let mut code: u8 = 0;
                for item in diffs {
                    match item {
                        PdfObject::Integer(n) => {
                            code = (*n).clamp(0, 255) as u8;
                        }
                        PdfObject::Name(name) => {
                            if let Some(ch) = glyph_name_to_char(name.as_bytes()) {
                                overrides.push((code, ch));
                            }
                            code = code.wrapping_add(1);
                        }
                        _ => {}
                    }
                }
            }
            FontEncoding::Custom(CustomEncoding { base, overrides })
        }
        _ => FontEncoding::Predefined(PredefinedEncoding::Standard),
    }
}

/// Parse `/Widths` array into a `Vec<u16>`.
fn parse_widths(font_dict: &PdfDictionary) -> Vec<u16> {
    font_dict
        .get_array(b"Widths")
        .map(|arr| {
            arr.iter()
                .filter_map(|o| o.as_f64().map(|v| v.clamp(0.0, 65535.0) as u16))
                .collect()
        })
        .unwrap_or_default()
}

/// Extract a base font name string from a font dictionary.
fn base_font_name(font_dict: &PdfDictionary) -> String {
    font_dict
        .get_name(b"BaseFont")
        .and_then(|b| b.as_str().map(|s| s.to_string()))
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fxcrt::bytestring::PdfByteString;
    use std::io::Cursor;

    // -----------------------------------------------------------------------
    // Test PDF builders
    // -----------------------------------------------------------------------

    /// Minimal PDF (no font objects).
    fn minimal_pdf() -> Vec<u8> {
        let mut pdf = Vec::new();
        pdf.extend_from_slice(b"%PDF-1.4\n");
        let obj1_off = pdf.len();
        pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
        let obj2_off = pdf.len();
        pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [] /Count 0 >>\nendobj\n");
        let xref_off = pdf.len();
        pdf.extend_from_slice(b"xref\n0 3\n");
        pdf.extend_from_slice(b"0000000000 65535 f \n");
        pdf.extend_from_slice(format!("{:010} 00000 n \n", obj1_off).as_bytes());
        pdf.extend_from_slice(format!("{:010} 00000 n \n", obj2_off).as_bytes());
        pdf.extend_from_slice(b"trailer\n<< /Size 3 /Root 1 0 R >>\n");
        pdf.extend_from_slice(format!("startxref\n{xref_off}\n%%EOF\n").as_bytes());
        pdf
    }

    /// PDF that includes a raw (uncompressed) ToUnicode stream as object 3.
    fn pdf_with_to_unicode() -> Vec<u8> {
        let cmap_data = b"begincmap\n\
                          2 beginbfchar\n\
                          <41> <0041>\n\
                          <42> <0042>\n\
                          endbfchar\n\
                          endcmap\n";
        let cmap_len = cmap_data.len();

        let mut pdf = Vec::new();
        pdf.extend_from_slice(b"%PDF-1.4\n");
        let obj1_off = pdf.len();
        pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
        let obj2_off = pdf.len();
        pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [] /Count 0 >>\nendobj\n");
        let obj3_off = pdf.len();
        pdf.extend_from_slice(format!("3 0 obj\n<< /Length {cmap_len} >>\nstream\n").as_bytes());
        pdf.extend_from_slice(cmap_data);
        pdf.extend_from_slice(b"\nendstream\nendobj\n");

        let xref_off = pdf.len();
        pdf.extend_from_slice(b"xref\n0 4\n");
        pdf.extend_from_slice(b"0000000000 65535 f \n");
        pdf.extend_from_slice(format!("{:010} 00000 n \n", obj1_off).as_bytes());
        pdf.extend_from_slice(format!("{:010} 00000 n \n", obj2_off).as_bytes());
        pdf.extend_from_slice(format!("{:010} 00000 n \n", obj3_off).as_bytes());
        pdf.extend_from_slice(b"trailer\n<< /Size 4 /Root 1 0 R >>\n");
        pdf.extend_from_slice(format!("startxref\n{xref_off}\n%%EOF\n").as_bytes());
        pdf
    }

    /// Build a simple Type1 font dictionary with WinAnsiEncoding.
    fn type1_winansi_dict() -> PdfDictionary {
        let mut d = PdfDictionary::new();
        d.set("Type", PdfObject::Name(PdfByteString::from("Font")));
        d.set("Subtype", PdfObject::Name(PdfByteString::from("Type1")));
        d.set(
            "BaseFont",
            PdfObject::Name(PdfByteString::from("Helvetica")),
        );
        d.set(
            "Encoding",
            PdfObject::Name(PdfByteString::from("WinAnsiEncoding")),
        );
        d.set("FirstChar", PdfObject::Integer(32));
        d.set("LastChar", PdfObject::Integer(34));
        d.set(
            "Widths",
            PdfObject::Array(vec![
                PdfObject::Integer(278),
                PdfObject::Integer(278),
                PdfObject::Integer(355),
            ]),
        );
        d
    }

    // -----------------------------------------------------------------------
    // PdfFont::load tests
    // -----------------------------------------------------------------------

    #[test]
    fn load_type1_is_simple_variant() {
        let mut doc = Document::from_reader(Cursor::new(minimal_pdf())).unwrap();
        let dict = type1_winansi_dict();
        let font = PdfFont::load(&dict, &mut doc).unwrap();
        assert!(matches!(font, PdfFont::Simple { .. }));
    }

    #[test]
    fn load_truetype_is_simple_variant() {
        let mut d = PdfDictionary::new();
        d.set("Type", PdfObject::Name(PdfByteString::from("Font")));
        d.set("Subtype", PdfObject::Name(PdfByteString::from("TrueType")));
        d.set("BaseFont", PdfObject::Name(PdfByteString::from("Arial")));
        let mut doc = Document::from_reader(Cursor::new(minimal_pdf())).unwrap();
        let font = PdfFont::load(&d, &mut doc).unwrap();
        assert!(matches!(font, PdfFont::Simple { .. }));
    }

    #[test]
    fn load_type0_is_unsupported() {
        let mut d = PdfDictionary::new();
        d.set("Type", PdfObject::Name(PdfByteString::from("Font")));
        d.set("Subtype", PdfObject::Name(PdfByteString::from("Type0")));
        d.set("BaseFont", PdfObject::Name(PdfByteString::from("CIDFont")));
        let mut doc = Document::from_reader(Cursor::new(minimal_pdf())).unwrap();
        let font = PdfFont::load(&d, &mut doc).unwrap();
        assert!(matches!(font, PdfFont::Unsupported { .. }));
    }

    #[test]
    fn load_type3_is_unsupported() {
        let mut d = PdfDictionary::new();
        d.set("Type", PdfObject::Name(PdfByteString::from("Font")));
        d.set("Subtype", PdfObject::Name(PdfByteString::from("Type3")));
        d.set(
            "BaseFont",
            PdfObject::Name(PdfByteString::from("CustomFont")),
        );
        let mut doc = Document::from_reader(Cursor::new(minimal_pdf())).unwrap();
        let font = PdfFont::load(&d, &mut doc).unwrap();
        assert!(matches!(font, PdfFont::Unsupported { .. }));
    }

    #[test]
    fn load_with_differences_encoding() {
        let mut d = type1_winansi_dict();
        let enc_dict = {
            let mut e = PdfDictionary::new();
            e.set("Type", PdfObject::Name(PdfByteString::from("Encoding")));
            e.set(
                "BaseEncoding",
                PdfObject::Name(PdfByteString::from("WinAnsiEncoding")),
            );
            // /Differences [65 /Aacute]  → code 0x41 ('A') → 'Á'
            e.set(
                "Differences",
                PdfObject::Array(vec![
                    PdfObject::Integer(65),
                    PdfObject::Name(PdfByteString::from("Aacute")),
                ]),
            );
            e
        };
        d.set("Encoding", PdfObject::Dictionary(enc_dict));
        let mut doc = Document::from_reader(Cursor::new(minimal_pdf())).unwrap();
        // Load should succeed; encoding is Custom
        let font = PdfFont::load(&d, &mut doc).unwrap();
        assert!(matches!(font, PdfFont::Simple { .. }));
    }

    #[test]
    fn load_with_to_unicode_stream() {
        // Font dict references object 3 as ToUnicode
        let mut d = type1_winansi_dict();
        d.set(
            "ToUnicode",
            PdfObject::Reference(crate::fpdfapi::parser::object::ObjectId::new(3, 0)),
        );
        let mut doc = Document::from_reader(Cursor::new(pdf_with_to_unicode())).unwrap();
        let font = PdfFont::load(&d, &mut doc).unwrap();
        // 0x41 ('A') should be mapped via ToUnicode
        assert_eq!(font.unicode_from_char_code(0x41), Some("A".to_string()));
    }

    // -----------------------------------------------------------------------
    // unicode_from_char_code tests
    // -----------------------------------------------------------------------

    #[test]
    fn unicode_via_encoding_winansi() {
        let mut doc = Document::from_reader(Cursor::new(minimal_pdf())).unwrap();
        let font = PdfFont::load(&type1_winansi_dict(), &mut doc).unwrap();
        // 0x41 in WinAnsi = 'A'
        assert_eq!(font.unicode_from_char_code(0x41), Some("A".to_string()));
    }

    #[test]
    fn unicode_returns_none_for_undefined_code() {
        let mut doc = Document::from_reader(Cursor::new(minimal_pdf())).unwrap();
        let font = PdfFont::load(&type1_winansi_dict(), &mut doc).unwrap();
        // 0x81 is undefined in WinAnsi
        assert_eq!(font.unicode_from_char_code(0x81), None);
    }

    #[test]
    fn to_unicode_takes_priority_over_encoding() {
        let mut d = type1_winansi_dict();
        d.set(
            "ToUnicode",
            PdfObject::Reference(crate::fpdfapi::parser::object::ObjectId::new(3, 0)),
        );
        let mut doc = Document::from_reader(Cursor::new(pdf_with_to_unicode())).unwrap();
        let font = PdfFont::load(&d, &mut doc).unwrap();
        // ToUnicode maps 0x41 → "A" (same as encoding here, but proves priority)
        assert_eq!(font.unicode_from_char_code(0x41), Some("A".to_string()));
        // 0x43 is not in ToUnicode; falls back to WinAnsi encoding
        assert_eq!(font.unicode_from_char_code(0x43), Some("C".to_string()));
    }

    #[test]
    fn unsupported_font_unicode_returns_none() {
        let mut d = PdfDictionary::new();
        d.set("Type", PdfObject::Name(PdfByteString::from("Font")));
        d.set("Subtype", PdfObject::Name(PdfByteString::from("Type0")));
        d.set("BaseFont", PdfObject::Name(PdfByteString::from("CIDFont")));
        let mut doc = Document::from_reader(Cursor::new(minimal_pdf())).unwrap();
        let font = PdfFont::load(&d, &mut doc).unwrap();
        assert_eq!(font.unicode_from_char_code(0x41), None);
    }

    // -----------------------------------------------------------------------
    // char_width tests
    // -----------------------------------------------------------------------

    #[test]
    fn char_width_in_range() {
        let mut doc = Document::from_reader(Cursor::new(minimal_pdf())).unwrap();
        let font = PdfFont::load(&type1_winansi_dict(), &mut doc).unwrap();
        // FirstChar=32, Widths=[278, 278, 355]
        // code 32 (space) → 278
        assert!((font.char_width(32) - 278.0).abs() < 1e-6);
        // code 34 (quotedbl) → 355
        assert!((font.char_width(34) - 355.0).abs() < 1e-6);
    }

    #[test]
    fn char_width_out_of_range_returns_default() {
        let mut doc = Document::from_reader(Cursor::new(minimal_pdf())).unwrap();
        let font = PdfFont::load(&type1_winansi_dict(), &mut doc).unwrap();
        // code 0x41 ('A') is above LastChar=34 → default 1000.0
        assert!((font.char_width(0x41) - 1000.0).abs() < 1e-6);
    }

    #[test]
    fn char_width_no_widths_returns_default() {
        let mut d = PdfDictionary::new();
        d.set("Type", PdfObject::Name(PdfByteString::from("Font")));
        d.set("Subtype", PdfObject::Name(PdfByteString::from("Type1")));
        d.set(
            "BaseFont",
            PdfObject::Name(PdfByteString::from("Helvetica")),
        );
        let mut doc = Document::from_reader(Cursor::new(minimal_pdf())).unwrap();
        let font = PdfFont::load(&d, &mut doc).unwrap();
        assert!((font.char_width(0x41) - 1000.0).abs() < 1e-6);
    }

    #[test]
    fn unsupported_font_char_width_returns_default() {
        let mut d = PdfDictionary::new();
        d.set("Type", PdfObject::Name(PdfByteString::from("Font")));
        d.set("Subtype", PdfObject::Name(PdfByteString::from("Type0")));
        d.set("BaseFont", PdfObject::Name(PdfByteString::from("CIDFont")));
        let mut doc = Document::from_reader(Cursor::new(minimal_pdf())).unwrap();
        let font = PdfFont::load(&d, &mut doc).unwrap();
        assert!((font.char_width(0x41) - 1000.0).abs() < 1e-6);
    }

    // -----------------------------------------------------------------------
    // parse_encoding helper tests
    // -----------------------------------------------------------------------

    #[test]
    fn parse_encoding_named_winansi() {
        let mut d = PdfDictionary::new();
        d.set(
            "Encoding",
            PdfObject::Name(PdfByteString::from("WinAnsiEncoding")),
        );
        let enc = parse_encoding(&d);
        assert!(matches!(
            enc,
            FontEncoding::Predefined(PredefinedEncoding::WinAnsi)
        ));
    }

    #[test]
    fn parse_encoding_no_entry_defaults_to_standard() {
        let d = PdfDictionary::new();
        let enc = parse_encoding(&d);
        assert!(matches!(
            enc,
            FontEncoding::Predefined(PredefinedEncoding::Standard)
        ));
    }
}
