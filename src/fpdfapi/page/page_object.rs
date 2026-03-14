use crate::fpdfapi::font::pdf_font::PdfFont;
use crate::fxcrt::coordinates::{Matrix, Point};

/// A single character entry within a text object.
pub struct CharEntry {
    /// Raw character code (single-byte for Simple fonts, multi-byte for CID fonts).
    pub code: u32,
    /// Character origin in user space (after CTM × text_matrix × text_pos transformation).
    pub origin: Point,
    /// Glyph advance width in font units (1/1000 of text-space unit).
    pub width: f64,
}

/// A text object extracted from a content stream (`BT` … `ET` block).
pub struct TextObject {
    /// All character entries in rendering order.
    pub char_codes: Vec<CharEntry>,
    /// The font active when this object was rendered.
    pub font: PdfFont,
    /// Font size in text-space units (from `Tf` operator).
    pub font_size: f64,
    /// Text matrix at the start of this text object.
    pub text_matrix: Matrix,
    /// Current transformation matrix at the start of this text object.
    pub ctm: Matrix,
}

/// A page content object. Only `Text` carries data in Phase 3; the rest are stubs.
pub enum PageObject {
    Text(Box<TextObject>),
    /// Path objects (lines, curves, rectangles) — not extracted in Phase 3.
    Path,
    /// Image XObjects — not extracted in Phase 3.
    Image,
    /// Form XObjects — minimally handled for recursion; stub otherwise.
    Form,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fpdfapi::font::pdf_font::PdfFont;

    #[test]
    #[ignore = "not yet implemented"]
    fn char_entry_fields_accessible() {
        let entry = CharEntry {
            code: 65,
            origin: Point::new(10.0, 20.0),
            width: 600.0,
        };
        assert_eq!(entry.code, 65);
        assert_eq!(entry.origin, Point::new(10.0, 20.0));
        assert!((entry.width - 600.0).abs() < 1e-9);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn text_object_fields_accessible() {
        let obj = TextObject {
            char_codes: vec![CharEntry {
                code: 72,
                origin: Point::new(0.0, 0.0),
                width: 750.0,
            }],
            font: PdfFont::Unsupported {
                base_font: "Helvetica".to_string(),
            },
            font_size: 12.0,
            text_matrix: Matrix::default(),
            ctm: Matrix::default(),
        };
        assert_eq!(obj.char_codes.len(), 1);
        assert!((obj.font_size - 12.0).abs() < 1e-9);
        assert!(obj.text_matrix.is_identity());
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn page_object_text_variant() {
        let obj = PageObject::Text(Box::new(TextObject {
            char_codes: vec![],
            font: PdfFont::Unsupported {
                base_font: "Times-Roman".to_string(),
            },
            font_size: 10.0,
            text_matrix: Matrix::default(),
            ctm: Matrix::default(),
        }));
        assert!(matches!(obj, PageObject::Text(_)));
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn page_object_stub_variants() {
        assert!(matches!(PageObject::Path, PageObject::Path));
        assert!(matches!(PageObject::Image, PageObject::Image));
        assert!(matches!(PageObject::Form, PageObject::Form));
    }
}
