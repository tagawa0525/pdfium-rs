use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read, Seek};
use std::path::Path;

use crate::error::Result;
use crate::fpdfapi::parser::cross_ref::{CrossRefTable, XRefEntry};
use crate::fpdfapi::parser::object::{ObjectId, PdfDictionary, PdfObject};
use crate::fpdfapi::parser::syntax::SyntaxParser;

/// Lazy object storage. Objects are parsed on demand.
enum LazyObject {
    Unparsed { offset: u64, gen_num: u16 },
    Parsed(PdfObject),
    CompressedRef { stream_obj_num: u32, index: u32 },
}

/// Document metadata from the Info dictionary.
pub struct DocumentInfo {
    info: Option<PdfDictionary>,
}

impl DocumentInfo {
    pub fn title(&self) -> Option<String> {
        todo!()
    }

    pub fn author(&self) -> Option<String> {
        todo!()
    }

    pub fn subject(&self) -> Option<String> {
        todo!()
    }

    pub fn creator(&self) -> Option<String> {
        todo!()
    }

    pub fn producer(&self) -> Option<String> {
        todo!()
    }
}

/// A parsed PDF document. Owns the object store and provides
/// access to pages, metadata, and the object graph.
pub struct Document<R: Read + Seek> {
    parser: SyntaxParser<R>,
    objects: HashMap<u32, LazyObject>,
    trailer: PdfDictionary,
}

impl Document<BufReader<File>> {
    /// Open a PDF file from a path.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        todo!()
    }
}

impl<R: Read + Seek> Document<R> {
    /// Open a PDF from any readable + seekable stream.
    pub fn from_reader(reader: R) -> Result<Self> {
        todo!()
    }

    /// Number of pages in the document.
    pub fn page_count(&self) -> u32 {
        todo!()
    }

    /// Get document metadata (Info dictionary).
    pub fn info(&mut self) -> DocumentInfo {
        todo!()
    }

    /// Resolve an object by its object number. Parses lazily.
    pub fn object(&mut self, obj_num: u32) -> Result<&PdfObject> {
        todo!()
    }

    /// Get the trailer dictionary.
    pub fn trailer(&self) -> &PdfDictionary {
        todo!()
    }

    /// Get the root (catalog) dictionary.
    pub fn catalog(&mut self) -> Result<&PdfDictionary> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    /// Minimal valid PDF for testing.
    fn minimal_pdf() -> Vec<u8> {
        let mut pdf = Vec::new();
        // Header
        pdf.extend_from_slice(b"%PDF-1.4\n");

        // Object 1: Catalog
        let obj1_offset = pdf.len();
        pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

        // Object 2: Pages
        let obj2_offset = pdf.len();
        pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [] /Count 0 >>\nendobj\n");

        // Object 3: Info
        let obj3_offset = pdf.len();
        pdf.extend_from_slice(
            b"3 0 obj\n<< /Title (Test Document) /Author (Test Author) >>\nendobj\n",
        );

        // Cross-reference table
        let xref_offset = pdf.len();
        pdf.extend_from_slice(b"xref\n");
        pdf.extend_from_slice(b"0 4\n");
        pdf.extend_from_slice(b"0000000000 65535 f \n");
        pdf.extend_from_slice(format!("{:010} 00000 n \n", obj1_offset).as_bytes());
        pdf.extend_from_slice(format!("{:010} 00000 n \n", obj2_offset).as_bytes());
        pdf.extend_from_slice(format!("{:010} 00000 n \n", obj3_offset).as_bytes());

        // Trailer
        pdf.extend_from_slice(b"trailer\n<< /Size 4 /Root 1 0 R /Info 3 0 R >>\n");
        pdf.extend_from_slice(format!("startxref\n{xref_offset}\n%%EOF\n").as_bytes());

        pdf
    }

    #[test]
    fn open_minimal_pdf() {
        let data = minimal_pdf();
        let doc = Document::from_reader(Cursor::new(data));
        assert!(doc.is_ok());
    }

    #[test]
    fn page_count_zero() {
        let data = minimal_pdf();
        let doc = Document::from_reader(Cursor::new(data)).unwrap();
        assert_eq!(doc.page_count(), 0);
    }

    #[test]
    fn get_info_title() {
        let data = minimal_pdf();
        let mut doc = Document::from_reader(Cursor::new(data)).unwrap();
        let info = doc.info();
        assert_eq!(info.title(), Some("Test Document".into()));
    }

    #[test]
    fn get_info_author() {
        let data = minimal_pdf();
        let mut doc = Document::from_reader(Cursor::new(data)).unwrap();
        let info = doc.info();
        assert_eq!(info.author(), Some("Test Author".into()));
    }

    #[test]
    fn trailer_has_root() {
        let data = minimal_pdf();
        let doc = Document::from_reader(Cursor::new(data)).unwrap();
        assert!(doc.trailer().contains_key(b"Root"));
    }

    #[test]
    fn resolve_object() {
        let data = minimal_pdf();
        let mut doc = Document::from_reader(Cursor::new(data)).unwrap();
        let obj = doc.object(1).unwrap();
        let dict = obj.as_dict().unwrap();
        assert_eq!(dict.get_name(b"Type").unwrap().as_bytes(), b"Catalog");
    }

    #[test]
    fn catalog_type() {
        let data = minimal_pdf();
        let mut doc = Document::from_reader(Cursor::new(data)).unwrap();
        let catalog = doc.catalog().unwrap();
        assert_eq!(catalog.get_name(b"Type").unwrap().as_bytes(), b"Catalog");
    }
}
