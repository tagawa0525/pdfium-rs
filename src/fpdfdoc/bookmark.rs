use std::collections::HashSet;
use std::io::{Read, Seek};

use crate::error::{Error, Result};
use crate::fpdfapi::parser::document::Document;
use crate::fpdfapi::parser::object::PdfObject;
use crate::fpdfdoc::action::Action;
use crate::fpdfdoc::util::decode_pdf_text_string;

/// A single bookmark (outline item) in the PDF outline tree.
///
/// Corresponds to C++ `CPDF_Bookmark`.
#[derive(Debug, Clone)]
pub struct Bookmark {
    pub title: String,
    pub action: Option<Action>,
    pub dest_array: Option<Vec<PdfObject>>,
    /// Negative means the bookmark is closed (children hidden).
    pub count: i32,
    pub children: Vec<Bookmark>,
}

/// Extension trait that adds outline (bookmark) access to `Document`.
///
/// Lives in `fpdfdoc` (Level 5) so that `fpdfapi/parser` (Level 2) stays
/// independent of higher-level document-structure types.
pub trait BookmarksExt {
    /// Return the document outline (bookmarks) tree.
    ///
    /// Returns `Ok(vec![])` if the document has no `/Outlines` entry or no
    /// first bookmark.  Returns `Err` if the PDF catalog (`/Root`) is absent
    /// or otherwise malformed.
    fn bookmarks(&mut self) -> Result<Vec<Bookmark>>;
}

impl<R: Read + Seek> BookmarksExt for Document<R> {
    fn bookmarks(&mut self) -> Result<Vec<Bookmark>> {
        // /Root (required — error if missing)
        let root_ref = self
            .trailer()
            .get(b"Root")
            .and_then(|o| o.as_reference())
            .ok_or_else(|| Error::InvalidPdf("trailer missing /Root".into()))?;

        // /Root → /Outlines (optional — empty if absent)
        let outlines_ref = {
            let root = self.object(root_ref.num)?;
            match root.as_dict().and_then(|d| d.get_reference(b"Outlines")) {
                Some(r) => r,
                None => return Ok(vec![]),
            }
        };

        // /Outlines → /First (optional — empty if absent)
        let first_ref = {
            let outlines = self.object(outlines_ref.num)?;
            match outlines.as_dict().and_then(|d| d.get_reference(b"First")) {
                Some(r) => r,
                None => return Ok(vec![]),
            }
        };

        let mut seen = HashSet::new();
        collect_bookmarks(self, first_ref.num, &mut seen)
    }
}

fn collect_bookmarks<R: Read + Seek>(
    doc: &mut Document<R>,
    first_num: u32,
    seen: &mut HashSet<u32>,
) -> Result<Vec<Bookmark>> {
    let mut result = Vec::new();
    let mut current = first_num;

    loop {
        if !seen.insert(current) {
            // Circular reference detected — stop silently
            break;
        }

        let dict = doc
            .object(current)?
            .as_dict()
            .ok_or_else(|| {
                Error::InvalidPdf(format!("outline item {current} is not a dictionary"))
            })?
            .clone();

        // /Title: decode PDF text string (UTF-16BE BOM or PDFDocEncoding)
        let title = dict
            .get_string(b"Title")
            .map(|s| decode_pdf_text_string(s.as_bytes()))
            .unwrap_or_default();

        let count = dict.get_i32(b"Count").unwrap_or(0);

        // /A action — resolve indirect reference if needed
        let action = match dict.get(b"A") {
            Some(PdfObject::Dictionary(d)) => Some(Action::from_dict(d.clone())),
            Some(PdfObject::Reference(id)) => {
                let num = id.num;
                doc.object(num)?
                    .as_dict()
                    .map(|d| Action::from_dict(d.clone()))
            }
            _ => None,
        };

        // /Dest destination array — resolve indirect reference if needed
        let dest_array = match dict.get(b"Dest") {
            Some(PdfObject::Array(arr)) => Some(arr.clone()),
            Some(PdfObject::Reference(id)) => {
                let num = id.num;
                doc.object(num)?.as_array().map(|a| a.to_vec())
            }
            _ => None,
        };

        // Recurse into children via /First
        let children = if let Some(child_ref) = dict.get_reference(b"First") {
            collect_bookmarks(doc, child_ref.num, seen)?
        } else {
            vec![]
        };

        result.push(Bookmark {
            title,
            action,
            dest_array,
            count,
            children,
        });

        // Advance to /Next sibling
        match dict.get_reference(b"Next") {
            Some(next) => current = next.num,
            None => break,
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::BookmarksExt;
    use crate::fpdfapi::parser::document::Document;
    use std::io::Cursor;

    // --- Helper to build minimal PDFs ---

    fn minimal_pdf_no_outlines() -> Vec<u8> {
        let mut pdf = Vec::new();
        pdf.extend_from_slice(b"%PDF-1.4\n");
        let obj1_offset = pdf.len();
        pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
        let obj2_offset = pdf.len();
        pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [] /Count 0 >>\nendobj\n");
        let xref_offset = pdf.len();
        pdf.extend_from_slice(b"xref\n0 3\n");
        pdf.extend_from_slice(b"0000000000 65535 f \n");
        pdf.extend_from_slice(format!("{:010} 00000 n \n", obj1_offset).as_bytes());
        pdf.extend_from_slice(format!("{:010} 00000 n \n", obj2_offset).as_bytes());
        pdf.extend_from_slice(b"trailer\n<< /Size 3 /Root 1 0 R >>\n");
        pdf.extend_from_slice(format!("startxref\n{xref_offset}\n%%EOF\n").as_bytes());
        pdf
    }

    /// PDF with a single bookmark "Chapter 1" with no action or destination.
    fn pdf_with_single_bookmark() -> Vec<u8> {
        let mut pdf = Vec::new();
        pdf.extend_from_slice(b"%PDF-1.4\n");

        // obj 1: Catalog (with /Outlines -> obj 3)
        let obj1_offset = pdf.len();
        pdf.extend_from_slice(
            b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R /Outlines 3 0 R >>\nendobj\n",
        );

        // obj 2: Pages
        let obj2_offset = pdf.len();
        pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [] /Count 0 >>\nendobj\n");

        // obj 3: Outlines root (empty outline dict with /First -> obj 4)
        let obj3_offset = pdf.len();
        pdf.extend_from_slice(
            b"3 0 obj\n<< /Type /Outlines /First 4 0 R /Last 4 0 R /Count 1 >>\nendobj\n",
        );

        // obj 4: Bookmark item
        let obj4_offset = pdf.len();
        pdf.extend_from_slice(b"4 0 obj\n<< /Title (Chapter 1) /Count 0 >>\nendobj\n");

        let xref_offset = pdf.len();
        pdf.extend_from_slice(b"xref\n0 5\n");
        pdf.extend_from_slice(b"0000000000 65535 f \n");
        pdf.extend_from_slice(format!("{:010} 00000 n \n", obj1_offset).as_bytes());
        pdf.extend_from_slice(format!("{:010} 00000 n \n", obj2_offset).as_bytes());
        pdf.extend_from_slice(format!("{:010} 00000 n \n", obj3_offset).as_bytes());
        pdf.extend_from_slice(format!("{:010} 00000 n \n", obj4_offset).as_bytes());
        pdf.extend_from_slice(b"trailer\n<< /Size 5 /Root 1 0 R >>\n");
        pdf.extend_from_slice(format!("startxref\n{xref_offset}\n%%EOF\n").as_bytes());
        pdf
    }

    /// PDF with nested bookmarks: Root -> Part I -> Chapter 1
    fn pdf_with_nested_bookmarks() -> Vec<u8> {
        let mut pdf = Vec::new();
        pdf.extend_from_slice(b"%PDF-1.4\n");

        let obj1_offset = pdf.len();
        pdf.extend_from_slice(
            b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R /Outlines 3 0 R >>\nendobj\n",
        );
        let obj2_offset = pdf.len();
        pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [] /Count 0 >>\nendobj\n");

        // obj 3: Outlines root, /First -> obj 4
        let obj3_offset = pdf.len();
        pdf.extend_from_slice(
            b"3 0 obj\n<< /Type /Outlines /First 4 0 R /Last 4 0 R /Count 1 >>\nendobj\n",
        );
        // obj 4: "Part I", has child obj 5
        let obj4_offset = pdf.len();
        pdf.extend_from_slice(
            b"4 0 obj\n<< /Title (Part I) /Count 1 /First 5 0 R /Last 5 0 R >>\nendobj\n",
        );
        // obj 5: "Chapter 1", no children
        let obj5_offset = pdf.len();
        pdf.extend_from_slice(b"5 0 obj\n<< /Title (Chapter 1) /Count 0 >>\nendobj\n");

        let xref_offset = pdf.len();
        pdf.extend_from_slice(b"xref\n0 6\n");
        pdf.extend_from_slice(b"0000000000 65535 f \n");
        pdf.extend_from_slice(format!("{:010} 00000 n \n", obj1_offset).as_bytes());
        pdf.extend_from_slice(format!("{:010} 00000 n \n", obj2_offset).as_bytes());
        pdf.extend_from_slice(format!("{:010} 00000 n \n", obj3_offset).as_bytes());
        pdf.extend_from_slice(format!("{:010} 00000 n \n", obj4_offset).as_bytes());
        pdf.extend_from_slice(format!("{:010} 00000 n \n", obj5_offset).as_bytes());
        pdf.extend_from_slice(b"trailer\n<< /Size 6 /Root 1 0 R >>\n");
        pdf.extend_from_slice(format!("startxref\n{xref_offset}\n%%EOF\n").as_bytes());
        pdf
    }

    // --- Tests ---

    #[test]

    fn bookmarks_empty_when_no_outlines() {
        let mut doc = Document::from_reader(Cursor::new(minimal_pdf_no_outlines())).unwrap();
        let bookmarks = doc.bookmarks().unwrap();
        assert!(bookmarks.is_empty());
    }

    #[test]

    fn single_bookmark_title() {
        let mut doc = Document::from_reader(Cursor::new(pdf_with_single_bookmark())).unwrap();
        let bookmarks = doc.bookmarks().unwrap();
        assert_eq!(bookmarks.len(), 1);
        assert_eq!(bookmarks[0].title, "Chapter 1");
    }

    #[test]

    fn single_bookmark_no_children() {
        let mut doc = Document::from_reader(Cursor::new(pdf_with_single_bookmark())).unwrap();
        let bookmarks = doc.bookmarks().unwrap();
        assert!(bookmarks[0].children.is_empty());
    }

    #[test]

    fn single_bookmark_count_zero() {
        let mut doc = Document::from_reader(Cursor::new(pdf_with_single_bookmark())).unwrap();
        let bookmarks = doc.bookmarks().unwrap();
        assert_eq!(bookmarks[0].count, 0);
    }

    #[test]

    fn nested_bookmark_tree() {
        let mut doc = Document::from_reader(Cursor::new(pdf_with_nested_bookmarks())).unwrap();
        let bookmarks = doc.bookmarks().unwrap();
        assert_eq!(bookmarks.len(), 1);
        assert_eq!(bookmarks[0].title, "Part I");
        assert_eq!(bookmarks[0].children.len(), 1);
        assert_eq!(bookmarks[0].children[0].title, "Chapter 1");
    }

    #[test]

    fn bookmark_with_uri_action() {
        let mut pdf = Vec::new();
        pdf.extend_from_slice(b"%PDF-1.4\n");

        let obj1_offset = pdf.len();
        pdf.extend_from_slice(
            b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R /Outlines 3 0 R >>\nendobj\n",
        );
        let obj2_offset = pdf.len();
        pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [] /Count 0 >>\nendobj\n");
        let obj3_offset = pdf.len();
        pdf.extend_from_slice(
            b"3 0 obj\n<< /Type /Outlines /First 4 0 R /Last 4 0 R /Count 1 >>\nendobj\n",
        );
        // Bookmark with inline /A action
        let obj4_offset = pdf.len();
        pdf.extend_from_slice(
            b"4 0 obj\n<< /Title (Link) /Count 0 /A << /S /URI /URI (https://example.com) >> >>\nendobj\n",
        );
        let xref_offset = pdf.len();
        pdf.extend_from_slice(b"xref\n0 5\n");
        pdf.extend_from_slice(b"0000000000 65535 f \n");
        pdf.extend_from_slice(format!("{:010} 00000 n \n", obj1_offset).as_bytes());
        pdf.extend_from_slice(format!("{:010} 00000 n \n", obj2_offset).as_bytes());
        pdf.extend_from_slice(format!("{:010} 00000 n \n", obj3_offset).as_bytes());
        pdf.extend_from_slice(format!("{:010} 00000 n \n", obj4_offset).as_bytes());
        pdf.extend_from_slice(b"trailer\n<< /Size 5 /Root 1 0 R >>\n");
        pdf.extend_from_slice(format!("startxref\n{xref_offset}\n%%EOF\n").as_bytes());

        let mut doc = Document::from_reader(Cursor::new(pdf)).unwrap();
        let bookmarks = doc.bookmarks().unwrap();
        assert_eq!(bookmarks.len(), 1);
        let action = bookmarks[0].action.as_ref().unwrap();
        use crate::fpdfdoc::action::ActionType;
        assert_eq!(action.action_type(), ActionType::Uri);
        assert_eq!(action.uri(), Some("https://example.com".to_string()));
    }
}
