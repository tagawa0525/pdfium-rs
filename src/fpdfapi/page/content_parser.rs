use std::io::{Read, Seek};

use crate::fpdfapi::page::page_object::PageObject;
use crate::fpdfapi::parser::document::Document;
use crate::fpdfapi::parser::object::PdfDictionary;

/// Parse a PDF content stream buffer and return the page objects it defines.
///
/// `resources` is the page's /Resources dictionary (used for font lookup).
/// `doc` is the document, needed to resolve indirect font references.
pub fn parse_content_stream<R: Read + Seek>(
    _data: &[u8],
    _resources: &PdfDictionary,
    _doc: &mut Document<R>,
) -> Vec<PageObject> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fpdfapi::parser::document::Document;
    use crate::fpdfapi::parser::object::{PdfDictionary, PdfObject};
    use crate::fxcrt::bytestring::PdfByteString;
    use std::io::Cursor;

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

    fn make_doc() -> Document<Cursor<Vec<u8>>> {
        Document::from_reader(Cursor::new(minimal_pdf())).unwrap()
    }

    /// Build minimal /Resources with an inline Type1 font at /F1.
    fn resources_with_type1() -> PdfDictionary {
        let mut font = PdfDictionary::new();
        font.set("Type", PdfObject::Name(PdfByteString::from("Font")));
        font.set("Subtype", PdfObject::Name(PdfByteString::from("Type1")));
        font.set(
            "BaseFont",
            PdfObject::Name(PdfByteString::from("Helvetica")),
        );
        font.set(
            "Encoding",
            PdfObject::Name(PdfByteString::from("WinAnsiEncoding")),
        );
        font.set("FirstChar", PdfObject::Integer(65));
        font.set("LastChar", PdfObject::Integer(66));
        font.set(
            "Widths",
            PdfObject::Array(vec![
                PdfObject::Integer(722), // A
                PdfObject::Integer(667), // B
            ]),
        );

        let mut font_map = PdfDictionary::new();
        font_map.set("F1", PdfObject::Dictionary(font));

        let mut resources = PdfDictionary::new();
        resources.set("Font", PdfObject::Dictionary(font_map));
        resources
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn empty_stream_returns_empty() {
        let mut doc = make_doc();
        let result = parse_content_stream(b"", &PdfDictionary::new(), &mut doc);
        assert!(result.is_empty());
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn whitespace_only_stream_returns_empty() {
        let mut doc = make_doc();
        let result = parse_content_stream(b"  \n\r\n  ", &PdfDictionary::new(), &mut doc);
        assert!(result.is_empty());
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn bt_et_no_text_produces_no_objects() {
        let mut doc = make_doc();
        // BT/ET without any Tj should produce no objects (nothing to render)
        let result = parse_content_stream(b"BT ET", &PdfDictionary::new(), &mut doc);
        assert!(result.is_empty());
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn tj_creates_char_entries_for_each_byte() {
        let mut doc = make_doc();
        let resources = resources_with_type1();
        // /F1 12 Tf sets font; (AB) Tj renders 2 chars
        let stream = b"BT /F1 12 Tf (AB) Tj ET";
        let result = parse_content_stream(stream, &resources, &mut doc);
        assert_eq!(result.len(), 1);
        if let PageObject::Text(obj) = &result[0] {
            assert_eq!(obj.char_entries.len(), 2);
            assert_eq!(obj.char_entries[0].code, 65); // 'A'
            assert_eq!(obj.char_entries[1].code, 66); // 'B'
        } else {
            panic!("expected PageObject::Text");
        }
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn char_entries_have_correct_widths() {
        let mut doc = make_doc();
        let resources = resources_with_type1();
        let stream = b"BT /F1 10 Tf (AB) Tj ET";
        let result = parse_content_stream(stream, &resources, &mut doc);
        assert_eq!(result.len(), 1);
        if let PageObject::Text(obj) = &result[0] {
            // A: width=722 font units
            assert!((obj.char_entries[0].width - 722.0).abs() < 1e-6);
            // B: width=667 font units
            assert!((obj.char_entries[1].width - 667.0).abs() < 1e-6);
        } else {
            panic!("expected PageObject::Text");
        }
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn td_moves_origin_of_subsequent_chars() {
        let mut doc = make_doc();
        let resources = resources_with_type1();
        // After 100 200 Td, text_line_pos = (100, 200)
        // With identity matrices, char origin should be near (100, 200)
        let stream = b"BT /F1 10 Tf 100 200 Td (A) Tj ET";
        let result = parse_content_stream(stream, &resources, &mut doc);
        assert_eq!(result.len(), 1);
        if let PageObject::Text(obj) = &result[0] {
            assert_eq!(obj.char_entries.len(), 1);
            let origin = obj.char_entries[0].origin;
            assert!((origin.x - 100.0).abs() < 1e-3, "x={}", origin.x);
            assert!((origin.y - 200.0).abs() < 1e-3, "y={}", origin.y);
        } else {
            panic!("expected PageObject::Text");
        }
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn tm_sets_text_matrix_for_char_origin() {
        let mut doc = make_doc();
        let resources = resources_with_type1();
        // 1 0 0 1 50 75 Tm → text matrix translate to (50, 75)
        let stream = b"BT /F1 10 Tf 1 0 0 1 50 75 Tm (A) Tj ET";
        let result = parse_content_stream(stream, &resources, &mut doc);
        assert_eq!(result.len(), 1);
        if let PageObject::Text(obj) = &result[0] {
            let origin = obj.char_entries[0].origin;
            assert!((origin.x - 50.0).abs() < 1e-3, "x={}", origin.x);
            assert!((origin.y - 75.0).abs() < 1e-3, "y={}", origin.y);
        } else {
            panic!("expected PageObject::Text");
        }
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn q_q_restores_graphics_state() {
        // After q/Q, the graphics state is restored.
        // We can't easily observe this from PageObjects, but we can verify
        // that chars rendered after Q use the pre-q state.
        let mut doc = make_doc();
        let resources = resources_with_type1();
        // Set font, save state, change position, restore, then render
        let stream = b"BT /F1 10 Tf 1 0 0 1 10 20 Tm q 1 0 0 1 999 999 Tm Q (A) Tj ET";
        let result = parse_content_stream(stream, &resources, &mut doc);
        assert_eq!(result.len(), 1);
        if let PageObject::Text(obj) = &result[0] {
            let origin = obj.char_entries[0].origin;
            // After Q restores, text_matrix from before q is back: (10, 20)
            assert!((origin.x - 10.0).abs() < 1e-3, "x={}", origin.x);
            assert!((origin.y - 20.0).abs() < 1e-3, "y={}", origin.y);
        } else {
            panic!("expected PageObject::Text");
        }
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn tj_array_with_kerning_adjusts_position() {
        let mut doc = make_doc();
        let resources = resources_with_type1();
        // [(A) -1000 (B)] TJ: A at 0, then -1000/1000*10=−10 adjustment, then B
        let stream = b"BT /F1 10 Tf [(A) -1000 (B)] TJ ET";
        let result = parse_content_stream(stream, &resources, &mut doc);
        assert_eq!(result.len(), 1);
        if let PageObject::Text(obj) = &result[0] {
            assert_eq!(obj.char_entries.len(), 2);
            let a_x = obj.char_entries[0].origin.x;
            let b_x = obj.char_entries[1].origin.x;
            // After A (width=722, size=10): advance = 722/1000*10 = 7.22
            // After -1000 kerning: adjust = -(-1000)/1000*10 = +10 (moves right)
            // Actually TJ: text_pos.x -= num/1000 * font_size * horz_scale
            // So -1000: text_pos.x -= -1000/1000 * 10 = text_pos.x += 10
            // B origin = a_x + 7.22 + 10
            assert!(b_x > a_x, "B should be right of A, a_x={a_x}, b_x={b_x}");
        } else {
            panic!("expected PageObject::Text");
        }
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn multiple_bt_et_blocks_produce_multiple_objects() {
        let mut doc = make_doc();
        let resources = resources_with_type1();
        let stream = b"BT /F1 10 Tf (A) Tj ET BT /F1 10 Tf (B) Tj ET";
        let result = parse_content_stream(stream, &resources, &mut doc);
        assert_eq!(result.len(), 2);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn unknown_operators_are_skipped() {
        let mut doc = make_doc();
        let resources = resources_with_type1();
        // Path operators should not crash or produce objects
        let stream = b"1 0 0 RG 0 0 100 100 re f BT /F1 10 Tf (A) Tj ET";
        let result = parse_content_stream(stream, &resources, &mut doc);
        assert_eq!(result.len(), 1);
    }
}
