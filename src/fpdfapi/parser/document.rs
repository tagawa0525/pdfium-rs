use std::borrow::Cow;
use std::collections::HashMap;
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
use std::fs::File;
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
use std::io::BufReader;
use std::io::{Read, Seek};
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
use std::path::Path;

use crate::error::{Error, Result};
use crate::fpdfapi::parser::cross_ref::{CrossRefTable, XRefEntry, find_startxref};
use crate::fpdfapi::parser::decode;
use crate::fpdfapi::parser::encrypt_dict;
use crate::fpdfapi::parser::object::{PdfDictionary, PdfObject, PdfStream};
use crate::fpdfapi::parser::security::SecurityHandler;
use crate::fpdfapi::parser::syntax::SyntaxParser;
use crate::fxcrt::coordinates::Rect;

/// Lazy object storage. Objects are parsed on demand.
enum LazyObject {
    Unparsed { offset: u64 },
    Parsed(PdfObject),
}

/// Inherited page-tree attributes, propagated from `/Pages` nodes to leaf `/Page` dicts.
#[derive(Default, Clone)]
struct PageInherit {
    media_box: Option<Rect>,
    crop_box: Option<Rect>,
    /// Clockwise rotation in degrees. Normalised to a multiple of 90.
    rotation: u16,
    resources: Option<PdfDictionary>,
}

impl PageInherit {
    /// Return a new `PageInherit` updated with any attributes present in `dict`.
    /// Child-level values shadow parent-level ones, matching the PDF spec.
    fn merge(mut self, dict: &PdfDictionary) -> Self {
        if let Some(arr) = dict.get_array(b"MediaBox")
            && let Some(r) = array_to_rect(arr)
        {
            self.media_box = Some(r);
        }
        if let Some(arr) = dict.get_array(b"CropBox")
            && let Some(r) = array_to_rect(arr)
        {
            self.crop_box = Some(r);
        }
        if let Some(rot) = dict.get_i32(b"Rotate") {
            // PDF spec requires /Rotate to be a multiple of 90. Normalise with
            // integer division so non-conformant values are floored to the
            // nearest valid step (e.g. 45 → 0, 100 → 90).
            let normalised = rot.rem_euclid(360) / 90 * 90;
            self.rotation = normalised as u16;
        }
        if let Some(res) = dict.get_dict(b"Resources") {
            self.resources = Some(res.clone());
        }
        self
    }
}

/// Parse a 4-element PDF array `[left bottom right top]` into a `Rect`.
fn array_to_rect(arr: &[PdfObject]) -> Option<Rect> {
    if arr.len() < 4 {
        return None;
    }
    let l = arr[0].as_f64()? as f32;
    let b = arr[1].as_f64()? as f32;
    let r = arr[2].as_f64()? as f32;
    let t = arr[3].as_f64()? as f32;
    Some(Rect::new(l, b, r, t))
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
    security: Option<SecurityHandler>,
}

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
impl Document<BufReader<File>> {
    /// Open a PDF file from a path.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        Self::from_reader(reader)
    }

    /// Open a password-protected PDF file from a path.
    pub fn open_with_password(path: impl AsRef<Path>, password: &[u8]) -> Result<Self> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        Self::from_reader_with_password(reader, password)
    }
}

impl<R: Read + Seek> Document<R> {
    /// Open a PDF from any readable + seekable stream.
    ///
    /// Returns an error if the PDF is encrypted. Use
    /// [`from_reader_with_password`](Self::from_reader_with_password) instead.
    pub fn from_reader(mut reader: R) -> Result<Self> {
        // Find startxref
        let xref_offset = find_startxref(&mut reader)?;

        // Parse cross-reference table
        let xref = CrossRefTable::parse(&mut reader, xref_offset)?;

        // Reject encrypted PDFs opened without a password
        if xref.trailer.contains_key(b"Encrypt") {
            return Err(Error::InvalidPdf(
                "document is encrypted; use open_with_password or from_reader_with_password".into(),
            ));
        }

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
            security: None,
        })
    }

    /// Open an encrypted PDF from any readable + seekable stream.
    ///
    /// If the PDF is not encrypted, the password is ignored and the
    /// document opens normally.
    pub fn from_reader_with_password(mut reader: R, password: &[u8]) -> Result<Self> {
        let xref_offset = find_startxref(&mut reader)?;
        let xref = CrossRefTable::parse(&mut reader, xref_offset)?;

        let mut objects = HashMap::new();
        for (&obj_num, entry) in &xref.entries {
            if let XRefEntry::Used { offset, .. } = entry {
                objects.insert(obj_num, LazyObject::Unparsed { offset: *offset });
            }
        }

        // Check for /Encrypt in trailer — resolve before building the main parser
        let security = if xref.trailer.contains_key(b"Encrypt") {
            let encrypt_dict_obj = match xref.trailer.get(b"Encrypt") {
                Some(PdfObject::Reference(id)) => {
                    if let Some(LazyObject::Unparsed { offset }) = objects.get(&id.num) {
                        let mut tmp_parser = SyntaxParser::new(&mut reader)?;
                        tmp_parser.seek(*offset)?;
                        let (parsed_id, obj) = tmp_parser.read_indirect_object()?;
                        if parsed_id.num != id.num {
                            return Err(Error::InvalidPdf(format!(
                                "xref for /Encrypt object {} points to object {} at offset {}",
                                id.num, parsed_id.num, offset
                            )));
                        }
                        obj
                    } else {
                        return Err(Error::InvalidPdf(
                            "/Encrypt reference not found in xref".into(),
                        ));
                    }
                }
                Some(obj) => obj.clone(),
                None => unreachable!(),
            };

            let encrypt_pdf_dict = encrypt_dict_obj
                .as_dict()
                .ok_or_else(|| Error::InvalidPdf("/Encrypt is not a dictionary".into()))?;

            let ed = encrypt_dict::parse_encrypt_dict(encrypt_pdf_dict)?;
            let file_id = encrypt_dict::extract_file_id(&xref.trailer);
            if file_id.is_empty() {
                return Err(Error::InvalidPdf(
                    "encrypted PDF requires /ID in trailer".into(),
                ));
            }
            let handler = SecurityHandler::new(&ed, &file_id, password)?;
            Some(handler)
        } else {
            None
        };

        let parser = SyntaxParser::new(reader)?;

        Ok(Document {
            parser,
            objects,
            trailer: xref.trailer,
            security,
        })
    }

    /// Whether this document is encrypted.
    pub fn is_encrypted(&self) -> bool {
        self.security.is_some()
    }

    /// Decode a stream: decrypt (if encrypted) then apply filter pipeline.
    ///
    /// `obj_num` and `gen_num` identify the indirect object that owns
    /// the stream (needed for per-object key derivation).
    pub fn decode_stream(&self, stream: &PdfStream, obj_num: u32, gen_num: u16) -> Result<Vec<u8>> {
        let raw = if let Some(ref handler) = self.security {
            Cow::Owned(handler.decrypt_bytes(obj_num, gen_num, &stream.data)?)
        } else {
            Cow::Borrowed(&stream.data)
        };
        decode::decode_stream(&raw, &stream.dict)
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

    /// Resolve a `PdfObject`: if it is a `Reference`, follow it to the stored object;
    /// otherwise clone and return the object itself.
    ///
    /// Returns an error if the referenced object does not exist.
    ///
    /// # Note on generation numbers
    ///
    /// This method (like [`Document::object`]) resolves references by object number only.
    /// Generation numbers (`ObjectId::gen_num`) are currently ignored because the internal
    /// object store is keyed solely on `obj_num`. This is safe for the vast majority of
    /// PDFs, which use generation number 0 throughout. Non-zero generation numbers
    /// (used only after in-place object replacement, a rare feature) are not validated
    /// against the cross-reference table.
    pub fn resolve(&mut self, obj: &PdfObject) -> Result<PdfObject> {
        match obj {
            PdfObject::Reference(id) => self.object(id.num).cloned(),
            other => Ok(other.clone()),
        }
    }

    /// Get a page by zero-based index.
    ///
    /// Traverses the page tree, collecting inherited attributes
    /// (MediaBox, CropBox, Rotation, Resources), decodes the content stream(s),
    /// and returns a fully parsed [`Page`].
    pub fn page(&mut self, n: u32) -> Result<crate::fpdfapi::page::pdf_page::Page> {
        // Resolve /Root → /Pages
        let pages_num = {
            let root_id = self
                .trailer
                .get(b"Root")
                .and_then(|o| o.as_reference())
                .ok_or_else(|| Error::InvalidPdf("trailer missing /Root".into()))?;
            let root_dict = self
                .object(root_id.num)?
                .as_dict()
                .ok_or_else(|| Error::InvalidPdf("/Root is not a dictionary".into()))?
                .clone();
            root_dict
                .get_reference(b"Pages")
                .ok_or_else(|| Error::InvalidPdf("/Root missing /Pages".into()))?
                .num
        };

        let mut idx = 0u32;
        self.find_page_in_tree(pages_num, n, &mut idx, PageInherit::default())?
            .ok_or_else(|| Error::InvalidPdf(format!("page index {n} out of range")))
    }

    /// Recursive page-tree traversal. Returns `Some(Page)` when the target
    /// zero-based index is found, `None` otherwise.
    fn find_page_in_tree(
        &mut self,
        node_id: u32,
        target: u32,
        idx: &mut u32,
        inherit: PageInherit,
    ) -> Result<Option<crate::fpdfapi::page::pdf_page::Page>> {
        // Clone the dict to release the immutable borrow before any further &mut self calls.
        let dict = self
            .object(node_id)?
            .as_dict()
            .ok_or_else(|| Error::InvalidPdf(format!("page tree node {node_id} is not a dict")))?
            .clone();

        let node_type = dict.get_name(b"Type").map(|n| n.as_bytes().to_vec());
        let mut merged = inherit.merge(&dict);

        // /Resources may be an indirect reference (common in real PDFs).
        // `PageInherit::merge` only handles direct dictionaries via `get_dict`,
        // so resolve indirect references here.  Child-level /Resources must
        // always shadow parent-level, hence no `is_none()` guard.
        if let Some(res_ref) = dict.get_reference(b"Resources")
            && let Ok(obj) = self.object(res_ref.num)
            && let Some(d) = obj.as_dict()
        {
            merged.resources = Some(d.clone());
        }

        match node_type.as_deref() {
            Some(b"Page") => {
                if *idx == target {
                    Ok(Some(self.build_page(dict, merged)?))
                } else {
                    *idx += 1;
                    Ok(None)
                }
            }
            _ => {
                // Pages intermediate node (or missing /Type — treat as Pages).
                let kids: Vec<u32> = dict
                    .get_array(b"Kids")
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|o| o.as_reference().map(|id| id.num))
                            .collect()
                    })
                    .unwrap_or_default();

                for kid_id in kids {
                    if let Some(page) =
                        self.find_page_in_tree(kid_id, target, idx, merged.clone())?
                    {
                        return Ok(Some(page));
                    }
                }
                Ok(None)
            }
        }
    }

    /// Build a `Page` from a leaf page dictionary and its inherited attributes.
    fn build_page(
        &mut self,
        page_dict: PdfDictionary,
        inherit: PageInherit,
    ) -> Result<crate::fpdfapi::page::pdf_page::Page> {
        use crate::fpdfapi::page::content_parser::parse_content_stream;

        let media_box = inherit
            .media_box
            .ok_or_else(|| Error::InvalidPdf("page missing /MediaBox".into()))?;
        let resources = inherit.resources.unwrap_or_default();

        let content_data = self.collect_page_contents(&page_dict)?;
        let objects = parse_content_stream(&content_data, &resources, self);

        Ok(crate::fpdfapi::page::pdf_page::Page {
            media_box,
            crop_box: inherit.crop_box,
            rotation: inherit.rotation,
            objects,
        })
    }

    /// Concatenate all content streams referenced by `/Contents`.
    fn collect_page_contents(&mut self, page_dict: &PdfDictionary) -> Result<Vec<u8>> {
        let contents_obj = match page_dict.get(b"Contents") {
            Some(obj) => obj.clone(),
            None => return Ok(Vec::new()),
        };

        // A /Contents reference may point to a single stream *or* to an array of
        // stream references (valid per PDF spec §7.7.3.3). Resolve first, then dispatch.
        let stream_ids: Vec<u32> = match contents_obj {
            PdfObject::Reference(id) => {
                let resolved = self.object(id.num)?.clone();
                match resolved {
                    PdfObject::Stream(_) => vec![id.num],
                    PdfObject::Array(arr) => arr
                        .iter()
                        .filter_map(|o| o.as_reference().map(|r| r.num))
                        .collect(),
                    _ => {
                        return Err(Error::InvalidPdf(
                            "/Contents reference is not a stream or array".into(),
                        ));
                    }
                }
            }
            PdfObject::Array(arr) => arr
                .iter()
                .filter_map(|o| o.as_reference().map(|id| id.num))
                .collect(),
            _ => {
                return Err(Error::InvalidPdf(
                    "/Contents must be a stream reference or array".into(),
                ));
            }
        };

        let mut data = Vec::new();
        for num in stream_ids {
            let stream = self
                .object(num)?
                .as_stream()
                .ok_or_else(|| {
                    Error::InvalidPdf(format!("Contents element {num} is not a stream"))
                })?
                .clone();
            let decoded = self.decode_stream(&stream, num, 0)?;
            data.extend_from_slice(&decoded);
            // Ensure operators from adjacent streams are separated by whitespace.
            if !decoded.is_empty() && decoded.last() != Some(&b'\n') {
                data.push(b'\n');
            }
        }
        Ok(data)
    }

    /// Return the document outline (bookmarks) tree.
    ///
    /// Returns an empty `Vec` if the document has no `/Outlines` entry.
    pub fn bookmarks(&mut self) -> Result<Vec<crate::fpdfdoc::bookmark::Bookmark>> {
        todo!()
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
    use crate::fxcrt::bytestring::PdfByteString;
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

    // --- Document::resolve ---

    #[test]
    fn resolve_direct_object_returns_itself() {
        let data = minimal_pdf();
        let mut doc = Document::from_reader(Cursor::new(data)).unwrap();
        let obj = PdfObject::Integer(42);
        let resolved = doc.resolve(&obj).unwrap();
        assert_eq!(resolved, PdfObject::Integer(42));
    }

    #[test]
    fn resolve_reference_follows_to_stored_object() {
        let data = minimal_pdf();
        let mut doc = Document::from_reader(Cursor::new(data)).unwrap();
        // Object 1 is the Catalog dictionary
        let obj_ref = PdfObject::Reference(crate::fpdfapi::parser::object::ObjectId::new(1, 0));
        let resolved = doc.resolve(&obj_ref).unwrap();
        assert!(resolved.as_dict().is_some());
        assert_eq!(
            resolved
                .as_dict()
                .unwrap()
                .get_name(b"Type")
                .unwrap()
                .as_bytes(),
            b"Catalog"
        );
    }

    #[test]
    fn resolve_reference_to_missing_object_is_error() {
        let data = minimal_pdf();
        let mut doc = Document::from_reader(Cursor::new(data)).unwrap();
        let obj_ref = PdfObject::Reference(crate::fpdfapi::parser::object::ObjectId::new(999, 0));
        assert!(doc.resolve(&obj_ref).is_err());
    }

    // --- Encryption integration tests ---

    /// Build a minimal RC4-encrypted PDF in memory.
    ///
    /// Uses revision 2, 40-bit key, empty user password.
    /// The /Encrypt dict is object 4, referenced from the trailer.
    fn encrypted_rc4_pdf() -> Vec<u8> {
        use crate::fdrm::{md5, rc4};
        use crate::fpdfapi::parser::security;

        let file_id = b"0123456789abcdef"; // 16-byte file ID
        let password = b""; // empty user password

        // --- Compute encryption parameters ---
        // We need /O and /U values, plus the encryption key.
        // For revision 2, key_length = 5 (40-bit).

        // Compute /O: encrypt padded owner password with MD5(padded_owner)
        let owner_padded = security::tests::pad_password_helper(b"");
        let owner_key = md5::digest(&owner_padded);
        let o_value = rc4::crypt(&owner_key[..5], &owner_padded).expect("rc4 crypt");

        // Compute encryption key: Algorithm 2
        let p_value: i32 = -4; // all permissions
        let encrypt_key = security::tests::calc_encrypt_key_helper(
            password, &o_value, p_value, file_id, 5, 2, true,
        );

        // Compute /U: Algorithm 4 (R2) — encrypt padding with key
        let u_value = security::tests::compute_u_r2_helper(&encrypt_key);

        // --- Build PDF bytes ---
        let mut pdf = Vec::new();
        pdf.extend_from_slice(b"%PDF-1.4\n");

        // Object 1: Catalog (not encrypted — /Type values are names, not strings)
        let obj1_offset = pdf.len();
        pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

        // Object 2: Pages
        let obj2_offset = pdf.len();
        pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [] /Count 0 >>\nendobj\n");

        // Object 3: Info with encrypted string
        // We encrypt the string "Secret Title" using the per-object key for obj 3 gen 0
        let title_plain = b"Secret Title";
        let obj3_key =
            security::tests::derive_object_key_helper(&encrypt_key, 3, 0, security::Cipher::Rc4);
        let title_encrypted = rc4::crypt(&obj3_key, title_plain).expect("rc4 crypt");
        let title_hex: String = title_encrypted.iter().map(|b| format!("{b:02X}")).collect();

        let obj3_offset = pdf.len();
        pdf.extend_from_slice(format!("3 0 obj\n<< /Title <{title_hex}> >>\nendobj\n").as_bytes());

        // Object 4: Encrypt dictionary
        let o_hex: String = o_value.iter().map(|b| format!("{b:02X}")).collect();
        let u_hex: String = u_value.iter().map(|b| format!("{b:02X}")).collect();
        let obj4_offset = pdf.len();
        pdf.extend_from_slice(
            format!(
                "4 0 obj\n<< /Filter /Standard /V 1 /R 2 /Length 40 /P {p_value} /O <{o_hex}> /U <{u_hex}> >>\nendobj\n"
            )
            .as_bytes(),
        );

        // File ID as hex string
        let file_id_hex: String = file_id.iter().map(|b| format!("{b:02X}")).collect();

        // Cross-reference table
        let xref_offset = pdf.len();
        pdf.extend_from_slice(b"xref\n");
        pdf.extend_from_slice(b"0 5\n");
        pdf.extend_from_slice(b"0000000000 65535 f \n");
        pdf.extend_from_slice(format!("{:010} 00000 n \n", obj1_offset).as_bytes());
        pdf.extend_from_slice(format!("{:010} 00000 n \n", obj2_offset).as_bytes());
        pdf.extend_from_slice(format!("{:010} 00000 n \n", obj3_offset).as_bytes());
        pdf.extend_from_slice(format!("{:010} 00000 n \n", obj4_offset).as_bytes());

        // Trailer with /Encrypt and /ID
        pdf.extend_from_slice(
            format!(
                "trailer\n<< /Size 5 /Root 1 0 R /Info 3 0 R /Encrypt 4 0 R /ID [<{file_id_hex}> <{file_id_hex}>] >>\n"
            )
            .as_bytes(),
        );
        pdf.extend_from_slice(format!("startxref\n{xref_offset}\n%%EOF\n").as_bytes());

        pdf
    }

    #[test]
    fn unencrypted_pdf_is_not_encrypted() {
        let data = minimal_pdf();
        let doc = Document::from_reader(Cursor::new(data)).unwrap();
        assert!(!doc.is_encrypted());
    }

    #[test]
    fn open_encrypted_pdf_with_correct_password() {
        let data = encrypted_rc4_pdf();
        let doc = Document::from_reader_with_password(Cursor::new(data), b"").unwrap();
        assert!(doc.is_encrypted());
    }

    #[test]
    fn open_encrypted_pdf_wrong_password_is_error() {
        let data = encrypted_rc4_pdf();
        let result = Document::from_reader_with_password(Cursor::new(data), b"wrong");
        assert!(result.is_err());
    }

    #[test]
    fn encrypted_pdf_unencrypted_open_detects_encryption() {
        // Opening an encrypted PDF without a password via from_reader
        // should detect /Encrypt in trailer and return an error.
        let data = encrypted_rc4_pdf();
        let result = Document::from_reader(Cursor::new(data));
        assert!(result.is_err());
    }

    #[test]
    fn open_with_password_on_unencrypted_pdf_succeeds() {
        // Passing a password to an unencrypted PDF should just work.
        let data = minimal_pdf();
        let doc = Document::from_reader_with_password(Cursor::new(data), b"any").unwrap();
        assert!(!doc.is_encrypted());
    }

    #[test]
    fn decode_stream_unencrypted() {
        // Build a PDF with a FlateDecode stream and verify decode_stream works
        use std::io::Write;
        let original = b"Hello, decoded stream!";
        let mut encoder =
            flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(original).unwrap();
        let compressed = encoder.finish().unwrap();

        let stream = PdfStream {
            dict: {
                let mut d = PdfDictionary::new();
                d.set(
                    "Filter",
                    PdfObject::Name(PdfByteString::from("FlateDecode")),
                );
                d.set("Length", PdfObject::Integer(compressed.len() as i32));
                d
            },
            data: compressed,
        };

        let data = minimal_pdf();
        let doc = Document::from_reader(Cursor::new(data)).unwrap();
        let decoded = doc.decode_stream(&stream, 1, 0).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn decode_stream_encrypted_rc4() {
        // Build encrypted PDF, open with password, decrypt + decode a stream
        use crate::fdrm::rc4;
        use crate::fpdfapi::parser::security;
        use std::io::Write;

        let file_id = b"0123456789abcdef";
        let password = b"";
        let p_value: i32 = -4;

        // Encryption setup (same as encrypted_rc4_pdf)
        let owner_padded = security::tests::pad_password_helper(b"");
        let owner_key = crate::fdrm::md5::digest(&owner_padded);
        let o_value = rc4::crypt(&owner_key[..5], &owner_padded).expect("rc4");
        let encrypt_key = security::tests::calc_encrypt_key_helper(
            password, &o_value, p_value, file_id, 5, 2, true,
        );

        // Object 5 will be a stream: FlateDecode compressed, then RC4 encrypted
        let original = b"Stream content here";
        let mut encoder =
            flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(original).unwrap();
        let compressed = encoder.finish().unwrap();

        // Encrypt compressed data with per-object key for obj 5 gen 0
        let obj5_key =
            security::tests::derive_object_key_helper(&encrypt_key, 5, 0, security::Cipher::Rc4);
        let encrypted_stream = rc4::crypt(&obj5_key, &compressed).expect("rc4");

        // Build stream object
        let stream = PdfStream {
            dict: {
                let mut d = PdfDictionary::new();
                d.set(
                    "Filter",
                    PdfObject::Name(PdfByteString::from("FlateDecode")),
                );
                d.set("Length", PdfObject::Integer(encrypted_stream.len() as i32));
                d
            },
            data: encrypted_stream,
        };

        // The test verifies: decrypt(RC4) → decompress(Flate) = original
        let data = encrypted_rc4_pdf();
        let doc = Document::from_reader_with_password(Cursor::new(data), b"").unwrap();
        let decoded = doc.decode_stream(&stream, 5, 0).unwrap();
        assert_eq!(decoded, original);
    }
}
