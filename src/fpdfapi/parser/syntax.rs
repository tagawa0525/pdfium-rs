use std::io::{Read, Seek, SeekFrom};

use crate::error::{Error, Result};
use crate::fpdfapi::parser::object::{ObjectId, PdfDictionary, PdfObject, PdfStream};
use crate::fxcrt::bytestring::PdfByteString;

/// PDF character classification.
fn is_whitespace(ch: u8) -> bool {
    matches!(ch, 0x00 | 0x09 | 0x0A | 0x0C | 0x0D | 0x20)
}

fn is_delimiter(ch: u8) -> bool {
    matches!(
        ch,
        b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%'
    )
}

fn is_numeric_start(ch: u8) -> bool {
    ch.is_ascii_digit() || ch == b'+' || ch == b'-' || ch == b'.'
}

/// Low-level PDF syntax tokenizer/parser.
///
/// Reads PDF tokens from a seekable byte stream and constructs
/// `PdfObject` values.
pub struct SyntaxParser<R: Read + Seek> {
    reader: R,
    buf: Vec<u8>,
    file_len: u64,
}

impl<R: Read + Seek> SyntaxParser<R> {
    pub fn new(reader: R) -> Result<Self> {
        todo!()
    }

    /// Current position in the stream.
    pub fn pos(&mut self) -> Result<u64> {
        todo!()
    }

    /// Seek to an absolute position.
    pub fn seek(&mut self, pos: u64) -> Result<()> {
        todo!()
    }

    /// Read the next PDF object at current position.
    pub fn read_object(&mut self) -> Result<PdfObject> {
        todo!()
    }

    /// Read an indirect object definition: `N M obj ... endobj`
    pub fn read_indirect_object(&mut self) -> Result<(ObjectId, PdfObject)> {
        todo!()
    }

    /// Read the next non-whitespace token as a keyword/word.
    fn read_word(&mut self) -> Result<Vec<u8>> {
        todo!()
    }

    /// Skip whitespace and comments.
    fn skip_whitespace_and_comments(&mut self) -> Result<()> {
        todo!()
    }

    /// Read a single byte, returning None at EOF.
    fn read_byte(&mut self) -> Result<Option<u8>> {
        todo!()
    }

    /// Peek the next byte without consuming.
    fn peek_byte(&mut self) -> Result<Option<u8>> {
        todo!()
    }

    /// Put back (unread) one byte.
    fn unread_byte(&mut self) -> Result<()> {
        todo!()
    }

    /// Parse a number (integer or real).
    fn read_number(&mut self, first: u8) -> Result<PdfObject> {
        todo!()
    }

    /// Parse a literal string `(...)`.
    fn read_literal_string(&mut self) -> Result<PdfByteString> {
        todo!()
    }

    /// Parse a hex string `<...>`.
    fn read_hex_string(&mut self) -> Result<PdfByteString> {
        todo!()
    }

    /// Parse a name object `/Name`.
    fn read_name(&mut self) -> Result<PdfByteString> {
        todo!()
    }

    /// Parse an array `[...]`.
    fn read_array(&mut self) -> Result<Vec<PdfObject>> {
        todo!()
    }

    /// Parse a dictionary `<< ... >>` and optionally a following stream.
    fn read_dict(&mut self) -> Result<PdfObject> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn parse(input: &[u8]) -> PdfObject {
        let mut parser = SyntaxParser::new(Cursor::new(input.to_vec())).unwrap();
        parser.read_object().unwrap()
    }

    fn parse_err(input: &[u8]) -> Error {
        let mut parser = SyntaxParser::new(Cursor::new(input.to_vec())).unwrap();
        parser.read_object().unwrap_err()
    }

    // --- Whitespace and comments ---

    #[test]
    fn skip_whitespace() {
        let obj = parse(b"   42  ");
        assert_eq!(obj.as_i32(), Some(42));
    }

    #[test]
    fn skip_comments() {
        let obj = parse(b"% this is a comment\n42");
        assert_eq!(obj.as_i32(), Some(42));
    }

    // --- Booleans ---

    #[test]
    fn parse_true() {
        assert_eq!(parse(b"true").as_bool(), Some(true));
    }

    #[test]
    fn parse_false() {
        assert_eq!(parse(b"false").as_bool(), Some(false));
    }

    // --- Null ---

    #[test]
    fn parse_null() {
        assert!(parse(b"null").is_null());
    }

    // --- Numbers ---

    #[test]
    fn parse_integer() {
        assert_eq!(parse(b"42").as_i32(), Some(42));
    }

    #[test]
    fn parse_negative_integer() {
        assert_eq!(parse(b"-17").as_i32(), Some(-17));
    }

    #[test]
    fn parse_positive_integer() {
        assert_eq!(parse(b"+5").as_i32(), Some(5));
    }

    #[test]
    fn parse_real() {
        let val = parse(b"3.14").as_f64().unwrap();
        assert!((val - 3.14).abs() < 1e-10);
    }

    #[test]
    fn parse_negative_real() {
        let val = parse(b"-2.5").as_f64().unwrap();
        assert!((val - (-2.5)).abs() < 1e-10);
    }

    #[test]
    fn parse_real_no_leading_zero() {
        let val = parse(b".5").as_f64().unwrap();
        assert!((val - 0.5).abs() < 1e-10);
    }

    #[test]
    fn parse_zero() {
        assert_eq!(parse(b"0").as_i32(), Some(0));
    }

    // --- Strings ---

    #[test]
    fn parse_literal_string() {
        let obj = parse(b"(hello world)");
        assert_eq!(obj.as_str().unwrap().as_bytes(), b"hello world");
    }

    #[test]
    fn parse_literal_string_escaped() {
        let obj = parse(b"(hello\\nworld)");
        assert_eq!(obj.as_str().unwrap().as_bytes(), b"hello\nworld");
    }

    #[test]
    fn parse_literal_string_nested_parens() {
        let obj = parse(b"(hello (world))");
        assert_eq!(obj.as_str().unwrap().as_bytes(), b"hello (world)");
    }

    #[test]
    fn parse_literal_string_octal() {
        let obj = parse(b"(\\101)"); // \101 = 'A'
        assert_eq!(obj.as_str().unwrap().as_bytes(), b"A");
    }

    #[test]
    fn parse_hex_string() {
        let obj = parse(b"<48656C6C6F>"); // "Hello"
        assert_eq!(obj.as_str().unwrap().as_bytes(), b"Hello");
    }

    #[test]
    fn parse_hex_string_lowercase() {
        let obj = parse(b"<48656c6c6f>");
        assert_eq!(obj.as_str().unwrap().as_bytes(), b"Hello");
    }

    #[test]
    fn parse_hex_string_with_spaces() {
        let obj = parse(b"<48 65 6C 6C 6F>");
        assert_eq!(obj.as_str().unwrap().as_bytes(), b"Hello");
    }

    #[test]
    fn parse_hex_string_odd() {
        // Odd number of hex digits: last nibble padded with 0
        let obj = parse(b"<ABC>");
        assert_eq!(obj.as_str().unwrap().as_bytes(), &[0xAB, 0xC0]);
    }

    #[test]
    fn parse_empty_hex_string() {
        let obj = parse(b"<>");
        assert_eq!(obj.as_str().unwrap().as_bytes(), b"");
    }

    // --- Names ---

    #[test]
    fn parse_name() {
        let obj = parse(b"/Type");
        assert_eq!(obj.as_name().unwrap().as_bytes(), b"Type");
    }

    #[test]
    fn parse_name_with_hex() {
        // /Type#20Name -> "Type Name"
        let obj = parse(b"/Type#20Name");
        assert_eq!(obj.as_name().unwrap().as_bytes(), b"Type Name");
    }

    #[test]
    fn parse_empty_name() {
        let obj = parse(b"/ ");
        assert_eq!(obj.as_name().unwrap().as_bytes(), b"");
    }

    // --- Arrays ---

    #[test]
    fn parse_array() {
        let obj = parse(b"[1 2 3]");
        let arr = obj.as_array().unwrap();
        assert_eq!(arr.len(), 3);
        assert_eq!(arr[0].as_i32(), Some(1));
        assert_eq!(arr[1].as_i32(), Some(2));
        assert_eq!(arr[2].as_i32(), Some(3));
    }

    #[test]
    fn parse_array_mixed() {
        let obj = parse(b"[1 (hello) /Name true null]");
        let arr = obj.as_array().unwrap();
        assert_eq!(arr.len(), 5);
        assert_eq!(arr[0].as_i32(), Some(1));
        assert_eq!(arr[1].as_str().unwrap().as_bytes(), b"hello");
        assert_eq!(arr[2].as_name().unwrap().as_bytes(), b"Name");
        assert_eq!(arr[3].as_bool(), Some(true));
        assert!(arr[4].is_null());
    }

    #[test]
    fn parse_nested_array() {
        let obj = parse(b"[[1 2] [3 4]]");
        let arr = obj.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0].as_array().unwrap().len(), 2);
    }

    #[test]
    fn parse_empty_array() {
        let obj = parse(b"[]");
        assert_eq!(obj.as_array().unwrap().len(), 0);
    }

    // --- Dictionaries ---

    #[test]
    fn parse_dictionary() {
        let obj = parse(b"<< /Type /Catalog /Pages 3 0 R >>");
        let dict = obj.as_dict().unwrap();
        assert_eq!(dict.get_name(b"Type").unwrap().as_bytes(), b"Catalog");
    }

    #[test]
    fn parse_dictionary_with_reference() {
        let obj = parse(b"<< /Pages 3 0 R >>");
        let dict = obj.as_dict().unwrap();
        let pages = dict.get(b"Pages").unwrap();
        assert_eq!(pages.as_reference(), Some(ObjectId::new(3, 0)));
    }

    #[test]
    fn parse_nested_dictionary() {
        let obj = parse(b"<< /Info << /Title (Test) >> >>");
        let dict = obj.as_dict().unwrap();
        let info = dict.get_dict(b"Info").unwrap();
        assert_eq!(info.get_string(b"Title").unwrap().as_bytes(), b"Test");
    }

    #[test]
    fn parse_empty_dictionary() {
        let obj = parse(b"<< >>");
        assert!(obj.as_dict().unwrap().is_empty());
    }

    // --- References ---

    #[test]
    fn parse_reference() {
        let obj = parse(b"10 0 R");
        assert_eq!(obj.as_reference(), Some(ObjectId::new(10, 0)));
    }

    // --- Indirect objects ---

    #[test]
    fn parse_indirect_object() {
        let input = b"1 0 obj\n42\nendobj";
        let mut parser = SyntaxParser::new(Cursor::new(input.to_vec())).unwrap();
        let (id, obj) = parser.read_indirect_object().unwrap();
        assert_eq!(id, ObjectId::new(1, 0));
        assert_eq!(obj.as_i32(), Some(42));
    }

    #[test]
    fn parse_indirect_dict() {
        let input = b"2 0 obj\n<< /Type /Page >>\nendobj";
        let mut parser = SyntaxParser::new(Cursor::new(input.to_vec())).unwrap();
        let (id, obj) = parser.read_indirect_object().unwrap();
        assert_eq!(id, ObjectId::new(2, 0));
        assert_eq!(
            obj.as_dict().unwrap().get_name(b"Type").unwrap().as_bytes(),
            b"Page"
        );
    }

    // --- Error cases ---

    #[test]
    fn parse_empty_is_error() {
        let result = parse_err(b"");
        assert!(matches!(result, Error::InvalidPdf(_)));
    }
}
