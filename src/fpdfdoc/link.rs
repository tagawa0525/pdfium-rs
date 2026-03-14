use std::io::{Read, Seek};

use crate::error::Result;
use crate::fpdfapi::parser::document::Document;
use crate::fpdfapi::parser::object::PdfObject;
use crate::fpdfdoc::action::Action;
use crate::fpdfdoc::dest::Dest;
use crate::fpdfdoc::name_tree::NameTree;
use crate::fxcrt::coordinates::Rect;

/// A hyperlink annotation extracted from a page.
#[derive(Debug, Clone)]
pub struct Link {
    pub rect: Rect,
    /// Explicit or named destination (absent when the link uses an `/A` action).
    pub dest: Option<Dest>,
    /// Action (e.g. URI, GoTo, Named).  May coexist with `dest`.
    pub action: Option<Action>,
}

/// Extension trait providing link annotation access on `Document`.
pub trait LinksExt {
    fn page_links(&mut self, page_index: u32) -> Result<Vec<Link>>;
}

impl<R: Read + Seek> LinksExt for Document<R> {
    fn page_links(&mut self, page_index: u32) -> Result<Vec<Link>> {
        let page_dict = self.page_dict(page_index)?;

        let annots_obj = match page_dict.get(b"Annots") {
            Some(obj) => obj.clone(),
            None => return Ok(vec![]),
        };

        let annot_refs: Vec<u32> = match annots_obj {
            PdfObject::Array(arr) => arr
                .iter()
                .filter_map(|o| o.as_reference().map(|id| id.num))
                .collect(),
            PdfObject::Reference(id) => self
                .object(id.num)?
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|o| o.as_reference().map(|r| r.num))
                        .collect()
                })
                .unwrap_or_default(),
            _ => return Ok(vec![]),
        };

        let mut result = Vec::new();
        for ref_num in annot_refs {
            let dict = self
                .object(ref_num)?
                .as_dict()
                .ok_or_else(|| {
                    crate::error::Error::InvalidPdf(format!(
                        "annotation object {ref_num} is not a dictionary"
                    ))
                })?
                .clone();

            if dict.get_name(b"Subtype").map(|n| n.as_bytes()) != Some(b"Link") {
                continue;
            }

            let rect = dict
                .get_array(b"Rect")
                .and_then(|arr| {
                    if arr.len() < 4 {
                        return None;
                    }
                    let l = arr[0].as_f64()? as f32;
                    let b = arr[1].as_f64()? as f32;
                    let r = arr[2].as_f64()? as f32;
                    let t = arr[3].as_f64()? as f32;
                    Some(Rect::new(l, b, r, t))
                })
                .unwrap_or_else(|| Rect::new(0.0, 0.0, 0.0, 0.0));

            let action = if let Some(action_obj) = dict.get(b"A").cloned() {
                resolve_action(self, action_obj)?
            } else {
                None
            };

            let dest = if let Some(dest_obj) = dict.get(b"Dest").cloned() {
                parse_dest(self, dest_obj)?
            } else {
                None
            };

            result.push(Link { rect, dest, action });
        }

        Ok(result)
    }
}

fn resolve_action<R: Read + Seek>(doc: &mut Document<R>, obj: PdfObject) -> Result<Option<Action>> {
    match obj {
        PdfObject::Dictionary(d) => Ok(Some(Action::from_dict(d))),
        PdfObject::Reference(id) => {
            let resolved = doc.object(id.num)?.clone();
            resolve_action(doc, resolved)
        }
        _ => Ok(None),
    }
}

fn parse_dest<R: Read + Seek>(doc: &mut Document<R>, obj: PdfObject) -> Result<Option<Dest>> {
    match obj {
        PdfObject::Array(arr) => {
            // [page_ref /ZoomMode params…] — skip first element (page ref)
            if arr.len() > 1 {
                Ok(Some(Dest::from_array(&arr[1..], None)))
            } else {
                Ok(None)
            }
        }
        PdfObject::String(s) => {
            let arr = NameTree::lookup_named_dest(doc, s.as_bytes())?;
            Ok(arr.and_then(|a| {
                if a.len() > 1 {
                    Some(Dest::from_array(&a[1..], None))
                } else {
                    None
                }
            }))
        }
        PdfObject::Name(n) => {
            let arr = NameTree::lookup_named_dest(doc, n.as_bytes())?;
            Ok(arr.and_then(|a| {
                if a.len() > 1 {
                    Some(Dest::from_array(&a[1..], None))
                } else {
                    None
                }
            }))
        }
        PdfObject::Reference(id) => {
            let resolved = doc.object(id.num)?.clone();
            parse_dest(doc, resolved)
        }
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fpdfdoc::action::ActionType;
    use crate::fpdfdoc::dest::ZoomMode;
    use std::io::Cursor;

    // --- page_links tests ---

    #[test]
    fn page_links_empty_page() {
        let pdf = pdf_with_empty_annots();
        let mut doc = Document::from_reader(Cursor::new(pdf)).unwrap();
        let links = doc.page_links(0).unwrap();
        assert!(links.is_empty());
    }

    #[test]
    fn page_links_with_uri_action() {
        let pdf = pdf_with_uri_link();
        let mut doc = Document::from_reader(Cursor::new(pdf)).unwrap();
        let links = doc.page_links(0).unwrap();
        assert_eq!(links.len(), 1);
        let link = &links[0];
        assert_eq!(link.rect.left, 10.0);
        assert_eq!(link.rect.bottom, 20.0);
        assert_eq!(link.rect.right, 110.0);
        assert_eq!(link.rect.top, 70.0);
        assert!(link.dest.is_none());
        let action = link.action.as_ref().unwrap();
        assert_eq!(action.action_type(), ActionType::Uri);
        assert_eq!(action.uri(), Some("https://example.com".to_string()));
    }

    #[test]
    fn page_links_with_explicit_dest() {
        let pdf = pdf_with_dest_link();
        let mut doc = Document::from_reader(Cursor::new(pdf)).unwrap();
        let links = doc.page_links(0).unwrap();
        assert_eq!(links.len(), 1);
        let link = &links[0];
        assert!(link.action.is_none());
        let dest = link.dest.as_ref().unwrap();
        assert_eq!(dest.zoom_mode, ZoomMode::XYZ);
        assert_eq!(dest.xyz(), Some((Some(0.0), Some(792.0), Some(0.0))));
    }

    #[test]
    fn page_links_non_link_annot_ignored() {
        let pdf = pdf_with_text_annot();
        let mut doc = Document::from_reader(Cursor::new(pdf)).unwrap();
        let links = doc.page_links(0).unwrap();
        assert!(links.is_empty());
    }

    #[test]
    fn page_links_with_named_dest_via_legacy_dests() {
        // Link annotation with /Dest (ch1) — resolved via legacy /Dests dict in catalog
        let pdf = pdf_with_named_dest_link();
        let mut doc = Document::from_reader(Cursor::new(pdf)).unwrap();
        let links = doc.page_links(0).unwrap();
        assert_eq!(links.len(), 1);
        let link = &links[0];
        assert!(link.action.is_none());
        let dest = link.dest.as_ref().unwrap();
        assert_eq!(dest.zoom_mode, ZoomMode::Fit);
    }

    // --- Helper PDFs ---

    fn pdf_with_empty_annots() -> Vec<u8> {
        let mut pdf = Vec::new();
        pdf.extend_from_slice(b"%PDF-1.4\n");
        let o1 = pdf.len();
        pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
        let o2 = pdf.len();
        pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
        let o3 = pdf.len();
        pdf.extend_from_slice(
            b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Annots [] >>\nendobj\n",
        );
        let xref = pdf.len();
        pdf.extend_from_slice(b"xref\n0 4\n0000000000 65535 f \n");
        pdf.extend_from_slice(format!("{o1:010} 00000 n \n").as_bytes());
        pdf.extend_from_slice(format!("{o2:010} 00000 n \n").as_bytes());
        pdf.extend_from_slice(format!("{o3:010} 00000 n \n").as_bytes());
        pdf.extend_from_slice(b"trailer\n<< /Size 4 /Root 1 0 R >>\n");
        pdf.extend_from_slice(format!("startxref\n{xref}\n%%EOF\n").as_bytes());
        pdf
    }

    fn pdf_with_uri_link() -> Vec<u8> {
        // obj 4: Link annotation with /A (URI action) as indirect ref to obj 5
        let mut pdf = Vec::new();
        pdf.extend_from_slice(b"%PDF-1.4\n");
        let o1 = pdf.len();
        pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
        let o2 = pdf.len();
        pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
        let o3 = pdf.len();
        pdf.extend_from_slice(
            b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Annots [4 0 R] >>\nendobj\n",
        );
        let o4 = pdf.len();
        pdf.extend_from_slice(
            b"4 0 obj\n<< /Type /Annot /Subtype /Link /Rect [10 20 110 70] /A 5 0 R >>\nendobj\n",
        );
        let o5 = pdf.len();
        pdf.extend_from_slice(b"5 0 obj\n<< /S /URI /URI (https://example.com) >>\nendobj\n");
        let xref = pdf.len();
        pdf.extend_from_slice(b"xref\n0 6\n0000000000 65535 f \n");
        pdf.extend_from_slice(format!("{o1:010} 00000 n \n").as_bytes());
        pdf.extend_from_slice(format!("{o2:010} 00000 n \n").as_bytes());
        pdf.extend_from_slice(format!("{o3:010} 00000 n \n").as_bytes());
        pdf.extend_from_slice(format!("{o4:010} 00000 n \n").as_bytes());
        pdf.extend_from_slice(format!("{o5:010} 00000 n \n").as_bytes());
        pdf.extend_from_slice(b"trailer\n<< /Size 6 /Root 1 0 R >>\n");
        pdf.extend_from_slice(format!("startxref\n{xref}\n%%EOF\n").as_bytes());
        pdf
    }

    fn pdf_with_dest_link() -> Vec<u8> {
        // obj 4: Link annotation with /Dest [3 0 R /XYZ 0 792 0]
        let mut pdf = Vec::new();
        pdf.extend_from_slice(b"%PDF-1.4\n");
        let o1 = pdf.len();
        pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
        let o2 = pdf.len();
        pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
        let o3 = pdf.len();
        pdf.extend_from_slice(
            b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Annots [4 0 R] >>\nendobj\n",
        );
        let o4 = pdf.len();
        pdf.extend_from_slice(
            b"4 0 obj\n<< /Type /Annot /Subtype /Link /Rect [10 20 110 70] /Dest [3 0 R /XYZ 0 792 0] >>\nendobj\n",
        );
        let xref = pdf.len();
        pdf.extend_from_slice(b"xref\n0 5\n0000000000 65535 f \n");
        pdf.extend_from_slice(format!("{o1:010} 00000 n \n").as_bytes());
        pdf.extend_from_slice(format!("{o2:010} 00000 n \n").as_bytes());
        pdf.extend_from_slice(format!("{o3:010} 00000 n \n").as_bytes());
        pdf.extend_from_slice(format!("{o4:010} 00000 n \n").as_bytes());
        pdf.extend_from_slice(b"trailer\n<< /Size 5 /Root 1 0 R >>\n");
        pdf.extend_from_slice(format!("startxref\n{xref}\n%%EOF\n").as_bytes());
        pdf
    }

    fn pdf_with_text_annot() -> Vec<u8> {
        // Page with a Text annotation (not Link) — should be ignored by page_links
        let mut pdf = Vec::new();
        pdf.extend_from_slice(b"%PDF-1.4\n");
        let o1 = pdf.len();
        pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
        let o2 = pdf.len();
        pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
        let o3 = pdf.len();
        pdf.extend_from_slice(
            b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Annots [4 0 R] >>\nendobj\n",
        );
        let o4 = pdf.len();
        pdf.extend_from_slice(
            b"4 0 obj\n<< /Type /Annot /Subtype /Text /Rect [10 20 110 70] /Contents (note) >>\nendobj\n",
        );
        let xref = pdf.len();
        pdf.extend_from_slice(b"xref\n0 5\n0000000000 65535 f \n");
        pdf.extend_from_slice(format!("{o1:010} 00000 n \n").as_bytes());
        pdf.extend_from_slice(format!("{o2:010} 00000 n \n").as_bytes());
        pdf.extend_from_slice(format!("{o3:010} 00000 n \n").as_bytes());
        pdf.extend_from_slice(format!("{o4:010} 00000 n \n").as_bytes());
        pdf.extend_from_slice(b"trailer\n<< /Size 5 /Root 1 0 R >>\n");
        pdf.extend_from_slice(format!("startxref\n{xref}\n%%EOF\n").as_bytes());
        pdf
    }

    fn pdf_with_named_dest_link() -> Vec<u8> {
        // Catalog has legacy /Dests << /ch1 [3 0 R /Fit] >>
        // Link annotation has /Dest (ch1) — string named dest
        let mut pdf = Vec::new();
        pdf.extend_from_slice(b"%PDF-1.4\n");
        let o1 = pdf.len();
        pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R /Dests 5 0 R >>\nendobj\n");
        let o2 = pdf.len();
        pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
        let o3 = pdf.len();
        pdf.extend_from_slice(
            b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Annots [4 0 R] >>\nendobj\n",
        );
        let o4 = pdf.len();
        pdf.extend_from_slice(
            b"4 0 obj\n<< /Type /Annot /Subtype /Link /Rect [10 20 110 70] /Dest (ch1) >>\nendobj\n",
        );
        let o5 = pdf.len();
        pdf.extend_from_slice(b"5 0 obj\n<< /ch1 [3 0 R /Fit] >>\nendobj\n");
        let xref = pdf.len();
        pdf.extend_from_slice(b"xref\n0 6\n0000000000 65535 f \n");
        pdf.extend_from_slice(format!("{o1:010} 00000 n \n").as_bytes());
        pdf.extend_from_slice(format!("{o2:010} 00000 n \n").as_bytes());
        pdf.extend_from_slice(format!("{o3:010} 00000 n \n").as_bytes());
        pdf.extend_from_slice(format!("{o4:010} 00000 n \n").as_bytes());
        pdf.extend_from_slice(format!("{o5:010} 00000 n \n").as_bytes());
        pdf.extend_from_slice(b"trailer\n<< /Size 6 /Root 1 0 R >>\n");
        pdf.extend_from_slice(format!("startxref\n{xref}\n%%EOF\n").as_bytes());
        pdf
    }
}
