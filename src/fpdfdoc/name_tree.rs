use std::io::{Read, Seek};

use crate::error::Result;
use crate::fpdfapi::parser::document::Document;
use crate::fpdfapi::parser::object::{PdfDictionary, PdfObject};

const MAX_DEPTH: usize = 32;

/// Utility for traversing PDF name trees.
///
/// A name tree is a balanced B-tree whose leaves carry `(name, value)` pairs.
/// Each intermediate node has `/Kids` (array of references to child nodes),
/// and each leaf node has `/Names` (flat array of alternating name/value pairs).
pub struct NameTree;

impl NameTree {
    /// Look up a name in the given name-tree root dictionary.
    ///
    /// Returns `Ok(Some(value))` if found, `Ok(None)` if absent.
    pub fn lookup<R: Read + Seek>(
        doc: &mut Document<R>,
        root: &PdfDictionary,
        name: &[u8],
    ) -> Result<Option<PdfObject>> {
        lookup_in_dict(doc, root, name, 0)
    }

    /// Look up a named destination.
    ///
    /// Checks `/Names/Dests` name tree first, then falls back to the
    /// legacy `/Dests` dictionary in the catalog.
    pub fn lookup_named_dest<R: Read + Seek>(
        _doc: &mut Document<R>,
        name: &[u8],
    ) -> Result<Option<Vec<PdfObject>>> {
        todo!("name={name:?}")
    }
}

fn lookup_in_dict<R: Read + Seek>(
    doc: &mut Document<R>,
    dict: &PdfDictionary,
    name: &[u8],
    depth: usize,
) -> Result<Option<PdfObject>> {
    if depth > MAX_DEPTH {
        return Ok(None);
    }

    // Leaf node: scan the flat /Names array
    if let Some(names_arr) = dict.get_array(b"Names") {
        return Ok(find_in_names_array(names_arr, name));
    }

    // Intermediate node: recurse into /Kids
    if let Some(kids) = dict.get_array(b"Kids") {
        let kid_refs: Vec<u32> = kids
            .iter()
            .filter_map(|o| o.as_reference().map(|id| id.num))
            .collect();
        for kid_num in kid_refs {
            let kid_dict = doc.object(kid_num)?.as_dict().cloned().unwrap_or_default();
            if let Some(v) = lookup_in_dict(doc, &kid_dict, name, depth + 1)? {
                return Ok(Some(v));
            }
        }
    }

    Ok(None)
}

/// Scan a flat `/Names` array `[name value name value …]` for `target`.
fn find_in_names_array(arr: &[PdfObject], target: &[u8]) -> Option<PdfObject> {
    let mut i = 0;
    while i + 1 < arr.len() {
        let key = match &arr[i] {
            PdfObject::String(s) => s.as_bytes(),
            PdfObject::Name(s) => s.as_bytes(),
            _ => {
                i += 2;
                continue;
            }
        };
        if key == target {
            return Some(arr[i + 1].clone());
        }
        i += 2;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fpdfapi::parser::document::Document;
    use crate::fpdfapi::parser::object::{PdfDictionary, PdfObject};
    use crate::fxcrt::bytestring::PdfByteString;
    use std::io::Cursor;

    fn str_obj(s: &str) -> PdfObject {
        PdfObject::String(PdfByteString::from(s))
    }
    fn int_obj(v: i32) -> PdfObject {
        PdfObject::Integer(v)
    }

    // --- Leaf node (/Names array) ---

    #[test]
    fn lookup_found_in_leaf() {
        let mut dict = PdfDictionary::new();
        dict.set(
            "Names",
            PdfObject::Array(vec![
                str_obj("alpha"),
                int_obj(1),
                str_obj("beta"),
                int_obj(2),
            ]),
        );
        // Use a minimal doc just for the function signature — not needed for leaf lookup
        let pdf = minimal_pdf();
        let mut doc = Document::from_reader(Cursor::new(pdf)).unwrap();
        let result = NameTree::lookup(&mut doc, &dict, b"beta").unwrap();
        assert_eq!(result, Some(int_obj(2)));
    }

    #[test]
    fn lookup_not_found_in_leaf() {
        let mut dict = PdfDictionary::new();
        dict.set(
            "Names",
            PdfObject::Array(vec![str_obj("alpha"), int_obj(1)]),
        );
        let pdf = minimal_pdf();
        let mut doc = Document::from_reader(Cursor::new(pdf)).unwrap();
        let result = NameTree::lookup(&mut doc, &dict, b"missing").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn lookup_empty_names_array() {
        let mut dict = PdfDictionary::new();
        dict.set("Names", PdfObject::Array(vec![]));
        let pdf = minimal_pdf();
        let mut doc = Document::from_reader(Cursor::new(pdf)).unwrap();
        let result = NameTree::lookup(&mut doc, &dict, b"x").unwrap();
        assert!(result.is_none());
    }

    // --- Kids (nested) lookup ---

    #[test]
    fn lookup_found_in_nested_kids() {
        // Build a PDF with obj 4 = intermediate node, obj 5 = leaf node
        let pdf = pdf_with_name_tree_kids();
        let mut doc = Document::from_reader(Cursor::new(pdf)).unwrap();

        // Root dict: /Kids [5 0 R]  (obj 4 is the root — but we pass it directly)
        let mut root = PdfDictionary::new();
        root.set(
            "Kids",
            PdfObject::Array(vec![PdfObject::Reference(
                crate::fpdfapi::parser::object::ObjectId::new(5, 0),
            )]),
        );
        let result = NameTree::lookup(&mut doc, &root, b"gamma").unwrap();
        assert_eq!(result, Some(int_obj(3)));
    }

    // --- Named dest lookup ---

    #[test]
    #[ignore = "not yet implemented"]
    fn lookup_named_dest_via_names_dests() {
        let pdf = pdf_with_named_dests_tree();
        let mut doc = Document::from_reader(Cursor::new(pdf)).unwrap();
        let dest = NameTree::lookup_named_dest(&mut doc, b"ch1").unwrap();
        assert!(dest.is_some());
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn lookup_named_dest_legacy_dests_dict() {
        let pdf = pdf_with_legacy_dests();
        let mut doc = Document::from_reader(Cursor::new(pdf)).unwrap();
        let dest = NameTree::lookup_named_dest(&mut doc, b"intro").unwrap();
        assert!(dest.is_some());
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn lookup_named_dest_not_found() {
        let pdf = minimal_pdf();
        let mut doc = Document::from_reader(Cursor::new(pdf)).unwrap();
        let dest = NameTree::lookup_named_dest(&mut doc, b"missing").unwrap();
        assert!(dest.is_none());
    }

    // --- Helper PDFs ---

    fn minimal_pdf() -> Vec<u8> {
        let mut pdf = Vec::new();
        pdf.extend_from_slice(b"%PDF-1.4\n");
        let o1 = pdf.len();
        pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
        let o2 = pdf.len();
        pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [] /Count 0 >>\nendobj\n");
        let xref = pdf.len();
        pdf.extend_from_slice(b"xref\n0 3\n0000000000 65535 f \n");
        pdf.extend_from_slice(format!("{o1:010} 00000 n \n").as_bytes());
        pdf.extend_from_slice(format!("{o2:010} 00000 n \n").as_bytes());
        pdf.extend_from_slice(b"trailer\n<< /Size 3 /Root 1 0 R >>\n");
        pdf.extend_from_slice(format!("startxref\n{xref}\n%%EOF\n").as_bytes());
        pdf
    }

    fn pdf_with_name_tree_kids() -> Vec<u8> {
        // obj 5: leaf node with /Names [gamma 3]
        let mut pdf = Vec::new();
        pdf.extend_from_slice(b"%PDF-1.4\n");
        let o1 = pdf.len();
        pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
        let o2 = pdf.len();
        pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [] /Count 0 >>\nendobj\n");
        let o3 = pdf.len();
        pdf.extend_from_slice(b"3 0 obj\n<< >>\nendobj\n");
        let o4 = pdf.len();
        pdf.extend_from_slice(b"4 0 obj\n<< /Kids [5 0 R] >>\nendobj\n");
        let o5 = pdf.len();
        pdf.extend_from_slice(b"5 0 obj\n<< /Names [(gamma) 3] >>\nendobj\n");
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

    fn pdf_with_named_dests_tree() -> Vec<u8> {
        // Catalog has /Names << /Dests 4 0 R >>
        // obj 4: name tree with /Names [(ch1) [2 0 R /XYZ 0 0 0]]
        let mut pdf = Vec::new();
        pdf.extend_from_slice(b"%PDF-1.4\n");
        let o1 = pdf.len();
        pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R /Names 3 0 R >>\nendobj\n");
        let o2 = pdf.len();
        pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [] /Count 0 >>\nendobj\n");
        let o3 = pdf.len();
        pdf.extend_from_slice(b"3 0 obj\n<< /Dests 4 0 R >>\nendobj\n");
        let o4 = pdf.len();
        // Named dest value is an array (destination array)
        pdf.extend_from_slice(b"4 0 obj\n<< /Names [(ch1) [2 0 R /XYZ 0 0 0]] >>\nendobj\n");
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

    fn pdf_with_legacy_dests() -> Vec<u8> {
        // Catalog has /Dests << /intro [2 0 R /Fit] >>  (legacy dict style)
        let mut pdf = Vec::new();
        pdf.extend_from_slice(b"%PDF-1.4\n");
        let o1 = pdf.len();
        pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R /Dests 3 0 R >>\nendobj\n");
        let o2 = pdf.len();
        pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [] /Count 0 >>\nendobj\n");
        let o3 = pdf.len();
        pdf.extend_from_slice(b"3 0 obj\n<< /intro [2 0 R /Fit] >>\nendobj\n");
        let xref = pdf.len();
        pdf.extend_from_slice(b"xref\n0 4\n0000000000 65535 f \n");
        pdf.extend_from_slice(format!("{o1:010} 00000 n \n").as_bytes());
        pdf.extend_from_slice(format!("{o2:010} 00000 n \n").as_bytes());
        pdf.extend_from_slice(format!("{o3:010} 00000 n \n").as_bytes());
        pdf.extend_from_slice(b"trailer\n<< /Size 4 /Root 1 0 R >>\n");
        pdf.extend_from_slice(format!("startxref\n{xref}\n%%EOF\n").as_bytes());
        pdf
    }
}
