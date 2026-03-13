use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read, Seek};
use std::path::Path;

use crate::error::{Error, Result};
use crate::fpdfapi::parser::cross_ref::{CrossRefTable, XRefEntry, find_startxref};
use crate::fpdfapi::parser::object::{PdfDictionary, PdfObject};
use crate::fpdfapi::parser::syntax::SyntaxParser;

/// Lazy object storage. Objects are parsed on demand.
enum LazyObject {
    Unparsed { offset: u64 },
    Parsed(PdfObject),
}

/// Document metadata from the Info dictionary.
pub struct DocumentInfo {
    info: Option<PdfDictionary>,
}

impl DocumentInfo {
    fn get_field(&self, key: &[u8]) -> Option<String> {
        self.info
            .as_ref()?
            .get_string(key)
            .and_then(|s| s.as_str().map(|s| s.to_string()))
    }

    pub fn title(&self) -> Option<String> {
        self.get_field(b"Title")
    }

    pub fn author(&self) -> Option<String> {
        self.get_field(b"Author")
    }

    pub fn subject(&self) -> Option<String> {
        self.get_field(b"Subject")
    }

    pub fn creator(&self) -> Option<String> {
        self.get_field(b"Creator")
    }

    pub fn producer(&self) -> Option<String> {
        self.get_field(b"Producer")
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
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        Self::from_reader(reader)
    }
}

impl<R: Read + Seek> Document<R> {
    /// Open a PDF from any readable + seekable stream.
    pub fn from_reader(mut reader: R) -> Result<Self> {
        // Find startxref
        let xref_offset = find_startxref(&mut reader)?;

        // Parse cross-reference table
        let xref = CrossRefTable::parse(&mut reader, xref_offset)?;

        // Build lazy object store from xref entries
        let mut objects = HashMap::new();
        for (&obj_num, entry) in &xref.entries {
            if let XRefEntry::Used { offset, .. } = entry {
                objects.insert(obj_num, LazyObject::Unparsed { offset: *offset });
            }
        }

        let parser = SyntaxParser::new(reader)?;

        Ok(Document {
            parser,
            objects,
            trailer: xref.trailer,
        })
    }

    /// Number of pages in the document.
    /// Resolves `/Root → /Pages → /Count` from the trailer.
    pub fn page_count(&mut self) -> Result<u32> {
        let root_ref = self
            .trailer
            .get(b"Root")
            .and_then(|o| o.as_reference())
            .ok_or_else(|| Error::InvalidPdf("trailer missing /Root".into()))?;

        let pages_ref = self
            .object(root_ref.num)?
            .as_dict()
            .ok_or_else(|| Error::InvalidPdf("/Root is not a dictionary".into()))?
            .get(b"Pages")
            .and_then(|o| o.as_reference())
            .ok_or_else(|| Error::InvalidPdf("/Root missing /Pages".into()))?;

        self.object(pages_ref.num)?
            .as_dict()
            .ok_or_else(|| Error::InvalidPdf("/Pages is not a dictionary".into()))?
            .get_i32(b"Count")
            .map(|c| c as u32)
            .ok_or_else(|| Error::InvalidPdf("/Pages missing /Count".into()))
    }

    /// Get document metadata (Info dictionary).
    pub fn info(&mut self) -> DocumentInfo {
        // Get /Info reference from trailer
        let info_ref = self.trailer.get(b"Info").and_then(|o| o.as_reference());

        let info = info_ref.and_then(|id| {
            self.object(id.num)
                .ok()
                .and_then(|obj| obj.as_dict().cloned())
        });

        DocumentInfo { info }
    }

    /// Resolve an object by its object number. Parses lazily.
    pub fn object(&mut self, obj_num: u32) -> Result<&PdfObject> {
        // Check if we need to parse
        if let Some(LazyObject::Unparsed { offset }) = self.objects.get(&obj_num) {
            let offset = *offset;
            self.parser.seek(offset)?;
            let (id, obj) = self.parser.read_indirect_object()?;
            if id.num != obj_num {
                return Err(Error::InvalidPdf(format!(
                    "xref for object {obj_num} points to object {} at offset {offset}",
                    id.num
                )));
            }
            self.objects.insert(obj_num, LazyObject::Parsed(obj));
        }

        match self.objects.get(&obj_num) {
            Some(LazyObject::Parsed(obj)) => Ok(obj),
            _ => Err(Error::InvalidPdf(format!("object {obj_num} not found"))),
        }
    }

    /// Get the trailer dictionary.
    pub fn trailer(&self) -> &PdfDictionary {
        &self.trailer
    }

    /// Get the root (catalog) dictionary.
    pub fn catalog(&mut self) -> Result<&PdfDictionary> {
        let root_ref = self
            .trailer
            .get(b"Root")
            .and_then(|o| o.as_reference())
            .ok_or_else(|| Error::InvalidPdf("trailer missing /Root".into()))?;

        let obj = self.object(root_ref.num)?;
        obj.as_dict()
            .ok_or_else(|| Error::InvalidPdf("/Root is not a dictionary".into()))
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
        let mut doc = Document::from_reader(Cursor::new(data)).unwrap();
        assert_eq!(doc.page_count().unwrap(), 0);
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
