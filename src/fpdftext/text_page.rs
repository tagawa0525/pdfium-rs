use crate::fpdfapi::page::page_object::PageObject;
use crate::fpdfapi::page::pdf_page::Page;
use crate::fxcrt::coordinates::{Point, Rect};

/// Per-character information extracted from a page.
pub struct CharInfo {
    /// Unicode scalar value.
    pub unicode: char,
    /// Glyph origin in user space — the point on the baseline where the glyph
    /// is positioned, computed as `CTM × text_matrix × (tx, ty + text_rise)`.
    pub origin: Point,
    /// Approximate bounding box in user space, estimated from the glyph advance
    /// width and font size. Not a tight typographic bounding box.
    pub char_box: Rect,
    /// Font size in text-space units (the operand of the `Tf` operator).
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
    pub fn build(page: &Page) -> TextPage {
        let mut chars: Vec<CharInfo> = Vec::new();
        let mut text = String::new();

        // Track the right edge and Y of the previously emitted glyph for
        // space/newline heuristics.
        let mut prev_right_x: Option<f32> = None;
        let mut prev_y: Option<f32> = None;
        let mut prev_font_size: f64 = 0.0;

        for obj in &page.objects {
            let text_obj = match obj {
                PageObject::Text(t) => t,
                _ => continue,
            };

            for entry in &text_obj.char_entries {
                let unicode_str = match text_obj.font.unicode_from_char_code(entry.code) {
                    Some(s) => s,
                    None => continue,
                };

                let x = entry.origin.x;
                let y = entry.origin.y;
                let font_size = text_obj.font_size;
                // Approximate glyph advance in user space (ignores CTM scale).
                let char_width_user = (entry.width / 1000.0 * font_size) as f32;

                // Insert separator before this glyph if needed.
                //
                // NOTE: These thresholds use the text-space font_size compared
                // against user-space coordinate deltas. When the CTM includes a
                // non-identity scale this can cause incorrect space/newline
                // detection. A future improvement should transform thresholds
                // through the CTM scale factor.
                if let (Some(py), Some(prx)) = (prev_y, prev_right_x) {
                    let y_shift = (y - py).abs();
                    if y_shift >= (prev_font_size * 0.5) as f32 {
                        text.push('\n');
                    } else if x - prx > (prev_font_size * 0.25) as f32 {
                        text.push(' ');
                    }
                }

                let char_box = Rect::new(x, y, x + char_width_user, y + font_size as f32);

                for ch in unicode_str.chars() {
                    chars.push(CharInfo {
                        unicode: ch,
                        origin: entry.origin,
                        char_box,
                        font_size,
                    });
                    text.push(ch);
                }

                prev_right_x = Some(x + char_width_user);
                prev_y = Some(y);
                prev_font_size = font_size;
            }
        }

        TextPage { chars, text }
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

    /// Build a single-page PDF from a raw content stream and the standard
    /// WinAnsi /F1 font resource.
    fn content_stream_pdf(content_stream: &[u8]) -> Vec<u8> {
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
            b"3 0 obj\n<< /Type /Page /Parent 2 0 R /Contents 4 0 R /Resources 5 0 R >>\nendobj\n",
        );

        let obj4_off = pdf.len();
        pdf.extend_from_slice(
            format!("4 0 obj\n<< /Length {} >>\nstream\n", content_stream.len()).as_bytes(),
        );
        pdf.extend_from_slice(content_stream);
        pdf.extend_from_slice(b"\nendstream\nendobj\n");

        let obj5_off = pdf.len();
        pdf.extend_from_slice(
            b"5 0 obj\n<< /Font << /F1 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding /FirstChar 32 /LastChar 122 /Widths [278 278 355 556 556 889 667 191 333 333 389 584 278 333 278 278 556 556 556 556 556 556 556 556 556 556 278 278 584 584 584 556 1015 667 667 722 722 667 611 778 722 278 500 667 556 833 722 778 667 778 722 667 611 722 667 944 667 667 611 278 278 278 469 556 333 556 556 500 556 556 278 556 556 222 222 500 222 833 556 556 556 556 333 500 278 556] >> >> >>\nendobj\n",
        );

        let xref_off = pdf.len();
        pdf.extend_from_slice(b"xref\n0 6\n");
        pdf.extend_from_slice(b"0000000000 65535 f \n");
        for off in [obj1_off, obj2_off, obj3_off, obj4_off, obj5_off] {
            pdf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
        }
        pdf.extend_from_slice(b"trailer\n<< /Size 6 /Root 1 0 R >>\n");
        pdf.extend_from_slice(format!("startxref\n{xref_off}\n%%EOF\n").as_bytes());
        pdf
    }

    // ── Tests ────────────────────────────────────────────────────────────────

    #[test]
    fn empty_page_text_is_empty() {
        let mut doc = Document::from_reader(Cursor::new(empty_page_pdf())).unwrap();
        let page = doc.page(0).unwrap();
        assert_eq!(page.extract_text(), "");
    }

    #[test]
    fn single_ascii_char_extracted() {
        let mut doc = Document::from_reader(Cursor::new(text_pdf(b"A"))).unwrap();
        let page = doc.page(0).unwrap();
        assert_eq!(page.extract_text(), "A");
    }

    #[test]
    fn multi_char_text_extracted() {
        let mut doc = Document::from_reader(Cursor::new(text_pdf(b"Hello"))).unwrap();
        let page = doc.page(0).unwrap();
        assert_eq!(page.extract_text(), "Hello");
    }

    #[test]
    fn char_count_matches_unicode_chars() {
        let mut doc = Document::from_reader(Cursor::new(text_pdf(b"AB"))).unwrap();
        let page = doc.page(0).unwrap();
        let tp = crate::fpdftext::text_page::TextPage::build(&page);
        assert_eq!(tp.char_count(), 2);
    }

    #[test]
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

    // ── Space / newline heuristics ──────────────────────────────────────────

    #[test]
    fn synthetic_space_inserted_between_distant_glyphs() {
        // Two Tj calls with a large horizontal gap → space inserted.
        // "A" at x=100, then move right by 50 (well beyond glyph width) → "B"
        let cs = b"BT /F1 12 Tf 100 700 Td (A) Tj 50 0 Td (B) Tj ET";
        let mut doc = Document::from_reader(Cursor::new(content_stream_pdf(cs))).unwrap();
        let page = doc.page(0).unwrap();
        let text = page.extract_text();
        assert!(
            text.contains("A B"),
            "expected synthetic space: got {:?}",
            text
        );
    }

    #[test]
    fn no_space_for_adjacent_glyphs() {
        // Single Tj with adjacent characters → no synthetic space.
        let mut doc = Document::from_reader(Cursor::new(text_pdf(b"AB"))).unwrap();
        let page = doc.page(0).unwrap();
        let text = page.extract_text();
        assert_eq!(text, "AB");
    }

    #[test]
    fn newline_inserted_on_y_shift() {
        // Two Tj calls with a vertical shift → newline inserted.
        // "A" at y=700, then move down by -14 (> font_size*0.5=6) → "B"
        let cs = b"BT /F1 12 Tf 100 700 Td (A) Tj 0 -14 Td (B) Tj ET";
        let mut doc = Document::from_reader(Cursor::new(content_stream_pdf(cs))).unwrap();
        let page = doc.page(0).unwrap();
        let text = page.extract_text();
        assert!(text.contains("A\nB"), "expected newline: got {:?}", text);
    }
}
