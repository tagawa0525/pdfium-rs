use crate::fpdftext::text_page::TextPage;

/// A match within a `TextPage`, expressed as character indices (not byte offsets).
pub struct TextMatch {
    /// Start index into `TextPage::chars` (inclusive).
    pub start: usize,
    /// End index into `TextPage::chars` (exclusive).
    pub end: usize,
}

/// Options controlling how text search is performed.
#[derive(Default)]
pub struct FindOptions {
    /// If `true`, the search is case-sensitive. Default: `false`.
    pub case_sensitive: bool,
    /// If `true`, only whole-word matches are returned. Default: `false`.
    pub whole_word: bool,
}

/// Text search over a [`TextPage`].
pub struct TextFind;

impl TextFind {
    /// Find all occurrences of `query` in `text_page` using `options`.
    ///
    /// Returns character-index ranges. Matches are non-overlapping and returned
    /// in order of first occurrence.
    pub fn find_all(text_page: &TextPage, query: &str, options: &FindOptions) -> Vec<TextMatch> {
        if query.is_empty() {
            return Vec::new();
        }

        // Collect the page's unicode chars for scanning.
        let page_chars: Vec<char> = (0..text_page.char_count())
            .filter_map(|i| text_page.char_info(i))
            .map(|ci| ci.unicode)
            .collect();

        let query_chars: Vec<char> = query.chars().collect();
        let qlen = query_chars.len();
        let plen = page_chars.len();

        let mut matches = Vec::new();

        let eq: fn(char, char) -> bool = if options.case_sensitive {
            |a, b| a == b
        } else {
            // ASCII-only case folding (Phase 3 scope).
            |a: char, b: char| a.eq_ignore_ascii_case(&b)
        };

        let mut i = 0;
        while i + qlen <= plen {
            // Check if query matches at position i.
            if query_chars
                .iter()
                .zip(&page_chars[i..i + qlen])
                .all(|(&qc, &pc)| eq(qc, pc))
            {
                // Whole-word check: surrounding chars must be non-alphanumeric.
                let word_ok = if options.whole_word {
                    let before_ok = i == 0 || !page_chars[i - 1].is_alphanumeric();
                    let after_ok = i + qlen == plen || !page_chars[i + qlen].is_alphanumeric();
                    before_ok && after_ok
                } else {
                    true
                };

                if word_ok {
                    matches.push(TextMatch {
                        start: i,
                        end: i + qlen,
                    });
                    i += qlen; // Advance past the match (non-overlapping).
                    continue;
                }
            }
            i += 1;
        }

        matches
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use crate::fpdfapi::parser::document::Document;
    use crate::fpdftext::text_find::{FindOptions, TextFind};
    use crate::fpdftext::text_page::TextPage;

    fn text_pdf(content_bytes: &[u8]) -> Vec<u8> {
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
        pdf.extend_from_slice(&content_stream);
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

    fn make_text_page(content: &[u8]) -> TextPage {
        let mut doc = Document::from_reader(Cursor::new(text_pdf(content))).unwrap();
        let page = doc.page(0).unwrap();
        TextPage::build(&page)
    }

    #[test]
    fn find_exact_match() {
        let tp = make_text_page(b"Hello");
        let opts = FindOptions::default();
        let matches = TextFind::find_all(&tp, "Hello", &opts);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].start, 0);
        assert_eq!(matches[0].end, 5);
    }

    #[test]
    fn find_case_insensitive_default() {
        let tp = make_text_page(b"Hello");
        let opts = FindOptions::default();
        let matches = TextFind::find_all(&tp, "hello", &opts);
        assert_eq!(matches.len(), 1);
    }

    #[test]
    fn find_case_sensitive_no_match() {
        let tp = make_text_page(b"Hello");
        let opts = FindOptions {
            case_sensitive: true,
            whole_word: false,
        };
        let matches = TextFind::find_all(&tp, "hello", &opts);
        assert!(matches.is_empty());
    }

    #[test]
    fn find_no_match_returns_empty() {
        let tp = make_text_page(b"Hello");
        let opts = FindOptions::default();
        let matches = TextFind::find_all(&tp, "xyz", &opts);
        assert!(matches.is_empty());
    }

    #[test]
    fn page_find_text_returns_matches() {
        let mut doc = Document::from_reader(Cursor::new(text_pdf(b"Hello"))).unwrap();
        let page = doc.page(0).unwrap();
        let matches = page.find_text("Hello");
        assert_eq!(matches.len(), 1);
    }
}
