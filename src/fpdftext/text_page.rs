use crate::fpdfapi::page::pdf_page::Page;
use crate::fxcrt::coordinates::{Point, Rect};

/// Per-character information extracted from a page.
pub struct CharInfo {
    /// Unicode scalar value.
    pub unicode: char,
    /// Character origin in user space (lower-left of the glyph).
    pub origin: Point,
    /// Tight bounding box in user space.
    pub char_box: Rect,
    /// Font size in text-space units.
    pub font_size: f64,
}

/// Extracted text with per-character position information.
pub struct TextPage {
    chars: Vec<CharInfo>,
    text: String,
}

impl TextPage {
    /// Build a `TextPage` from a parsed `Page`.
    ///
    /// Iterates `TextObject`s in rendering order, converts character codes to
    /// Unicode, inserts synthetic spaces and newlines based on glyph positions,
    /// and accumulates the result.
    pub fn build(_page: &Page) -> TextPage {
        todo!("TextPage::build — implement in GREEN commit")
    }

    /// The full extracted text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Number of characters in the extracted text.
    pub fn char_count(&self) -> usize {
        self.chars.len()
    }

    /// Per-character info at `index` (character index, not byte offset).
    pub fn char_info(&self, index: usize) -> Option<&CharInfo> {
        self.chars.get(index)
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use crate::fpdfapi::parser::document::Document;

    // ── PDF builder helpers ──────────────────────────────────────────────────

    /// Build a single-page PDF whose content stream encodes the given bytes as
    /// a literal string shown with `Tj`, using a Type1/WinAnsi font at /F1.
    fn text_pdf(content_bytes: &[u8]) -> Vec<u8> {
        // Escape special characters in PDF literal string.
        let mut escaped = Vec::with_capacity(content_bytes.len());
        for &b in content_bytes {
            match b {
                b'(' | b')' | b'\\' => {
                    escaped.push(b'\\');
                    escaped.push(b);
                }
                _ => escaped.push(b),
            }
        }

        let content_stream: Vec<u8> = {
            let mut s = b"BT /F1 12 Tf 100 700 Td (".to_vec();
            s.extend_from_slice(&escaped);
            s.extend_from_slice(b") Tj ET");
            s
        };

        let mut pdf = Vec::new();
        pdf.extend_from_slice(b"%PDF-1.4\n");

        // Object 1: Catalog
        let obj1_off = pdf.len();
        pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

        // Object 2: Pages
        let obj2_off = pdf.len();
        pdf.extend_from_slice(
            b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 612 792] >>\nendobj\n",
        );

        // Object 3: Page
        let obj3_off = pdf.len();
        pdf.extend_from_slice(
            b"3 0 obj\n<< /Type /Page /Parent 2 0 R /Contents 4 0 R /Resources 5 0 R >>\nendobj\n",
        );

        // Object 4: Content stream
        let obj4_off = pdf.len();
        pdf.extend_from_slice(
            format!("4 0 obj\n<< /Length {} >>\nstream\n", content_stream.len()).as_bytes(),
        );
        pdf.extend_from_slice(&content_stream);
        pdf.extend_from_slice(b"\nendstream\nendobj\n");

        // Object 5: Resources (with WinAnsi Type1 font /F1)
        let obj5_off = pdf.len();
        pdf.extend_from_slice(
            b"5 0 obj\n<< /Font << /F1 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding /FirstChar 32 /LastChar 122 /Widths [278 278 355 556 556 889 667 191 333 333 389 584 278 333 278 278 556 556 556 556 556 556 556 556 556 556 278 278 584 584 584 556 1015 667 667 722 722 667 611 778 722 278 500 667 556 833 722 778 667 778 722 667 611 722 667 944 667 667 611 278 278 278 469 556 333 556 556 500 556 556 278 556 556 222 222 500 222 833 556 556 556 556 333 500 278 556] >> >> >>\nendobj\n",
        );

        // Xref
        let xref_off = pdf.len();
        pdf.extend_from_slice(b"xref\n0 6\n");
        pdf.extend_from_slice(b"0000000000 65535 f \n");
        for off in [obj1_off, obj2_off, obj3_off, obj4_off, obj5_off] {
            pdf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
        }

        // Trailer
        pdf.extend_from_slice(b"trailer\n<< /Size 6 /Root 1 0 R >>\n");
        pdf.extend_from_slice(format!("startxref\n{xref_off}\n%%EOF\n").as_bytes());

        pdf
    }

    fn empty_page_pdf() -> Vec<u8> {
        let mut pdf = Vec::new();
        pdf.extend_from_slice(b"%PDF-1.4\n");
        let obj1_off = pdf.len();
        pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
        let obj2_off = pdf.len();
        pdf.extend_from_slice(
            b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 612 792] >>\nendobj\n",
        );
        let obj3_off = pdf.len();
        pdf.extend_from_slice(
            b"3 0 obj\n<< /Type /Page /Parent 2 0 R /Contents 4 0 R /Resources << >> >>\nendobj\n",
        );
        let obj4_off = pdf.len();
        pdf.extend_from_slice(b"4 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n");
        let xref_off = pdf.len();
        pdf.extend_from_slice(b"xref\n0 5\n");
        pdf.extend_from_slice(b"0000000000 65535 f \n");
        for off in [obj1_off, obj2_off, obj3_off, obj4_off] {
            pdf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
        }
        pdf.extend_from_slice(b"trailer\n<< /Size 5 /Root 1 0 R >>\n");
        pdf.extend_from_slice(format!("startxref\n{xref_off}\n%%EOF\n").as_bytes());
        pdf
    }

    // ── Tests ────────────────────────────────────────────────────────────────

    #[test]
    #[ignore = "not yet implemented"]
    fn empty_page_text_is_empty() {
        let mut doc = Document::from_reader(Cursor::new(empty_page_pdf())).unwrap();
        let page = doc.page(0).unwrap();
        assert_eq!(page.extract_text(), "");
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn single_ascii_char_extracted() {
        let mut doc = Document::from_reader(Cursor::new(text_pdf(b"A"))).unwrap();
        let page = doc.page(0).unwrap();
        assert_eq!(page.extract_text(), "A");
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn multi_char_text_extracted() {
        let mut doc = Document::from_reader(Cursor::new(text_pdf(b"Hello"))).unwrap();
        let page = doc.page(0).unwrap();
        assert_eq!(page.extract_text(), "Hello");
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn char_count_matches_unicode_chars() {
        let mut doc = Document::from_reader(Cursor::new(text_pdf(b"AB"))).unwrap();
        let page = doc.page(0).unwrap();
        let tp = crate::fpdftext::text_page::TextPage::build(&page);
        assert_eq!(tp.char_count(), 2);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn char_info_origin_is_accessible() {
        let mut doc = Document::from_reader(Cursor::new(text_pdf(b"A"))).unwrap();
        let page = doc.page(0).unwrap();
        let tp = crate::fpdftext::text_page::TextPage::build(&page);
        let info = tp.char_info(0).unwrap();
        assert_eq!(info.unicode, 'A');
        // origin should be non-zero (100, 700 from Td operator)
        assert!(info.origin.x > 0.0);
        assert!(info.origin.y > 0.0);
    }
}
