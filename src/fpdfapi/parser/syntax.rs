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

fn hex_val(ch: u8) -> Option<u8> {
    match ch {
        b'0'..=b'9' => Some(ch - b'0'),
        b'a'..=b'f' => Some(ch - b'a' + 10),
        b'A'..=b'F' => Some(ch - b'A' + 10),
        _ => None,
    }
}

/// Low-level PDF syntax tokenizer/parser.
///
/// Reads PDF tokens from a seekable byte stream and constructs
/// `PdfObject` values.
pub struct SyntaxParser<R: Read + Seek> {
    reader: R,
    #[allow(dead_code)]
    file_len: u64,
}

impl<R: Read + Seek> SyntaxParser<R> {
    pub fn new(mut reader: R) -> Result<Self> {
        let file_len = reader.seek(SeekFrom::End(0))?;
        reader.seek(SeekFrom::Start(0))?;
        Ok(SyntaxParser { reader, file_len })
    }

    /// Current position in the stream.
    pub fn pos(&mut self) -> Result<u64> {
        Ok(self.reader.stream_position()?)
    }

    /// Seek to an absolute position.
    pub fn seek(&mut self, pos: u64) -> Result<()> {
        self.reader.seek(SeekFrom::Start(pos))?;
        Ok(())
    }

    /// Read the next PDF object at current position.
    pub fn read_object(&mut self) -> Result<PdfObject> {
        self.skip_whitespace_and_comments()?;

        let ch = self
            .read_byte()?
            .ok_or_else(|| Error::InvalidPdf("unexpected EOF".into()))?;

        match ch {
            // Literal string
            b'(' => {
                let s = self.read_literal_string()?;
                Ok(PdfObject::String(s))
            }
            // Hex string or dictionary
            b'<' => {
                let next = self.peek_byte()?;
                if next == Some(b'<') {
                    self.read_byte()?; // consume second '<'
                    self.read_dict()
                } else {
                    let s = self.read_hex_string()?;
                    Ok(PdfObject::String(s))
                }
            }
            // Name
            b'/' => {
                let name = self.read_name()?;
                Ok(PdfObject::Name(name))
            }
            // Array
            b'[' => {
                let arr = self.read_array()?;
                Ok(PdfObject::Array(arr))
            }
            // Number or reference (N M R)
            _ if is_numeric_start(ch) => {
                let obj = self.read_number(ch)?;
                // Check for reference: integer integer 'R'
                if let PdfObject::Integer(num) = obj {
                    let saved_pos = self.pos()?;
                    if let Ok(obj2) = self.read_object() {
                        if let PdfObject::Integer(gen_num) = obj2 {
                            self.skip_whitespace_and_comments()?;
                            if let Some(b'R') = self.peek_byte()? {
                                self.read_byte()?;
                                return Ok(PdfObject::Reference(ObjectId::new(
                                    num as u32,
                                    gen_num as u16,
                                )));
                            }
                        }
                    }
                    // Not a reference, restore position
                    self.seek(saved_pos)?;
                    return Ok(obj);
                }
                Ok(obj)
            }
            // Keyword: true, false, null, or other
            _ if !is_delimiter(ch) => {
                let mut word = vec![ch];
                loop {
                    match self.peek_byte()? {
                        Some(c) if !is_whitespace(c) && !is_delimiter(c) => {
                            self.read_byte()?;
                            word.push(c);
                        }
                        _ => break,
                    }
                }
                match word.as_slice() {
                    b"true" => Ok(PdfObject::Boolean(true)),
                    b"false" => Ok(PdfObject::Boolean(false)),
                    b"null" => Ok(PdfObject::Null),
                    _ => Err(Error::InvalidPdf(format!(
                        "unexpected keyword: {}",
                        String::from_utf8_lossy(&word)
                    ))),
                }
            }
            _ => Err(Error::InvalidPdf(format!("unexpected byte: 0x{ch:02X}"))),
        }
    }

    /// Read an indirect object definition: `N M obj ... endobj`
    pub fn read_indirect_object(&mut self) -> Result<(ObjectId, PdfObject)> {
        self.skip_whitespace_and_comments()?;

        // Read object number
        let num_obj = self.read_object()?;
        let num = num_obj
            .as_i32()
            .ok_or_else(|| Error::InvalidPdf("expected object number".into()))?
            as u32;

        // Read generation number
        let gen_obj = self.read_object()?;
        let gen_num = gen_obj
            .as_i32()
            .ok_or_else(|| Error::InvalidPdf("expected generation number".into()))?
            as u16;

        // Read "obj" keyword
        self.skip_whitespace_and_comments()?;
        let word = self.read_word()?;
        if word != b"obj" {
            return Err(Error::InvalidPdf(format!(
                "expected 'obj', got '{}'",
                String::from_utf8_lossy(&word)
            )));
        }

        // Read the object body
        let obj = self.read_object()?;

        // Read "endobj" keyword
        self.skip_whitespace_and_comments()?;
        let end_word = self.read_word()?;
        if end_word != b"endobj" {
            return Err(Error::InvalidPdf(format!(
                "expected 'endobj', got '{}'",
                String::from_utf8_lossy(&end_word)
            )));
        }

        Ok((ObjectId::new(num, gen_num), obj))
    }

    /// Read the next non-whitespace token as a keyword/word.
    fn read_word(&mut self) -> Result<Vec<u8>> {
        self.skip_whitespace_and_comments()?;
        let mut word = Vec::new();
        loop {
            match self.peek_byte()? {
                Some(c) if !is_whitespace(c) && !is_delimiter(c) => {
                    self.read_byte()?;
                    word.push(c);
                }
                _ => break,
            }
        }
        if word.is_empty() {
            return Err(Error::InvalidPdf("expected word".into()));
        }
        Ok(word)
    }

    /// Skip whitespace and comments.
    fn skip_whitespace_and_comments(&mut self) -> Result<()> {
        loop {
            match self.peek_byte()? {
                Some(ch) if is_whitespace(ch) => {
                    self.read_byte()?;
                }
                Some(b'%') => {
                    // Skip to end of line
                    loop {
                        match self.read_byte()? {
                            Some(b'\n') | Some(b'\r') | None => break,
                            _ => {}
                        }
                    }
                }
                _ => return Ok(()),
            }
        }
    }

    /// Read a single byte, returning None at EOF.
    fn read_byte(&mut self) -> Result<Option<u8>> {
        let mut buf = [0u8; 1];
        match self.reader.read(&mut buf)? {
            0 => Ok(None),
            _ => Ok(Some(buf[0])),
        }
    }

    /// Peek the next byte without consuming.
    fn peek_byte(&mut self) -> Result<Option<u8>> {
        let byte = self.read_byte()?;
        if byte.is_some() {
            self.unread_byte()?;
        }
        Ok(byte)
    }

    /// Put back (unread) one byte.
    fn unread_byte(&mut self) -> Result<()> {
        self.reader.seek(SeekFrom::Current(-1))?;
        Ok(())
    }

    /// Parse a number (integer or real).
    fn read_number(&mut self, first: u8) -> Result<PdfObject> {
        let mut buf = vec![first];
        let mut has_dot = first == b'.';

        loop {
            match self.peek_byte()? {
                Some(c) if c.is_ascii_digit() => {
                    self.read_byte()?;
                    buf.push(c);
                }
                Some(b'.') if !has_dot => {
                    has_dot = true;
                    self.read_byte()?;
                    buf.push(b'.');
                }
                _ => break,
            }
        }

        let s = String::from_utf8_lossy(&buf);
        if has_dot {
            let val: f64 = s
                .parse()
                .map_err(|_| Error::InvalidPdf(format!("invalid number: {s}")))?;
            Ok(PdfObject::Real(val))
        } else {
            let val: i32 = s
                .parse()
                .map_err(|_| Error::InvalidPdf(format!("invalid integer: {s}")))?;
            Ok(PdfObject::Integer(val))
        }
    }

    /// Parse a literal string `(...)`. Opening `(` already consumed.
    fn read_literal_string(&mut self) -> Result<PdfByteString> {
        let mut result = Vec::new();
        let mut depth = 1u32;

        loop {
            let ch = self
                .read_byte()?
                .ok_or_else(|| Error::InvalidPdf("unterminated string".into()))?;

            match ch {
                b'(' => {
                    depth += 1;
                    result.push(b'(');
                }
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                    result.push(b')');
                }
                b'\\' => {
                    let esc = self
                        .read_byte()?
                        .ok_or_else(|| Error::InvalidPdf("unterminated escape".into()))?;
                    match esc {
                        b'n' => result.push(b'\n'),
                        b'r' => result.push(b'\r'),
                        b't' => result.push(b'\t'),
                        b'b' => result.push(0x08),
                        b'f' => result.push(0x0C),
                        b'(' => result.push(b'('),
                        b')' => result.push(b')'),
                        b'\\' => result.push(b'\\'),
                        b'\r' => {
                            // Line continuation: \CR or \CR\LF
                            if self.peek_byte()? == Some(b'\n') {
                                self.read_byte()?;
                            }
                        }
                        b'\n' => {
                            // Line continuation
                        }
                        b'0'..=b'7' => {
                            // Octal escape (up to 3 digits)
                            let mut val = esc - b'0';
                            for _ in 0..2 {
                                if let Some(c) = self.peek_byte()? {
                                    if (b'0'..=b'7').contains(&c) {
                                        self.read_byte()?;
                                        val = val * 8 + (c - b'0');
                                    } else {
                                        break;
                                    }
                                }
                            }
                            result.push(val);
                        }
                        _ => {
                            // Unknown escape: ignore backslash
                            result.push(esc);
                        }
                    }
                }
                _ => result.push(ch),
            }
        }

        Ok(PdfByteString::from(result))
    }

    /// Parse a hex string `<...>`. Opening `<` already consumed.
    fn read_hex_string(&mut self) -> Result<PdfByteString> {
        let mut hex_chars = Vec::new();

        loop {
            let ch = self
                .read_byte()?
                .ok_or_else(|| Error::InvalidPdf("unterminated hex string".into()))?;

            if ch == b'>' {
                break;
            }
            if is_whitespace(ch) {
                continue;
            }
            if let Some(v) = hex_val(ch) {
                hex_chars.push(v);
            } else {
                return Err(Error::InvalidPdf(format!(
                    "invalid hex character: 0x{ch:02X}"
                )));
            }
        }

        let mut result = Vec::with_capacity((hex_chars.len() + 1) / 2);
        let mut i = 0;
        while i < hex_chars.len() {
            let hi = hex_chars[i];
            let lo = if i + 1 < hex_chars.len() {
                hex_chars[i + 1]
            } else {
                0
            };
            result.push((hi << 4) | lo);
            i += 2;
        }

        Ok(PdfByteString::from(result))
    }

    /// Parse a name object `/Name`. Leading `/` already consumed.
    fn read_name(&mut self) -> Result<PdfByteString> {
        let mut name = Vec::new();

        loop {
            match self.peek_byte()? {
                Some(ch) if is_whitespace(ch) || is_delimiter(ch) => break,
                None => break,
                Some(b'#') => {
                    // Hex-encoded character in name
                    self.read_byte()?; // consume '#'
                    let h1 = self
                        .read_byte()?
                        .and_then(hex_val)
                        .ok_or_else(|| Error::InvalidPdf("invalid hex in name".into()))?;
                    let h2 = self
                        .read_byte()?
                        .and_then(hex_val)
                        .ok_or_else(|| Error::InvalidPdf("invalid hex in name".into()))?;
                    name.push((h1 << 4) | h2);
                }
                Some(ch) => {
                    self.read_byte()?;
                    name.push(ch);
                }
            }
        }

        Ok(PdfByteString::from(name))
    }

    /// Parse an array `[...]`. Opening `[` already consumed.
    fn read_array(&mut self) -> Result<Vec<PdfObject>> {
        let mut items = Vec::new();

        loop {
            self.skip_whitespace_and_comments()?;
            match self.peek_byte()? {
                Some(b']') => {
                    self.read_byte()?;
                    return Ok(items);
                }
                None => return Err(Error::InvalidPdf("unterminated array".into())),
                _ => {
                    let obj = self.read_object()?;
                    items.push(obj);
                }
            }
        }
    }

    /// Parse a dictionary `<< ... >>`. Opening `<<` already consumed.
    /// If followed by `stream`, reads stream data too.
    fn read_dict(&mut self) -> Result<PdfObject> {
        let mut dict = PdfDictionary::new();

        loop {
            self.skip_whitespace_and_comments()?;
            match self.peek_byte()? {
                Some(b'>') => {
                    self.read_byte()?;
                    // Consume the second '>'
                    let next = self.read_byte()?;
                    if next != Some(b'>') {
                        return Err(Error::InvalidPdf("expected '>>'".into()));
                    }
                    break;
                }
                None => return Err(Error::InvalidPdf("unterminated dictionary".into())),
                _ => {
                    // Read key (must be a name)
                    let key_obj = self.read_object()?;
                    let key = match key_obj {
                        PdfObject::Name(n) => n,
                        _ => return Err(Error::InvalidPdf("dictionary key must be a name".into())),
                    };
                    // Read value
                    let value = self.read_object()?;
                    dict.set(key, value);
                }
            }
        }

        // Check for stream
        let saved_pos = self.pos()?;
        self.skip_whitespace_and_comments()?;
        let mut word_buf = Vec::new();
        for _ in 0..6 {
            match self.peek_byte()? {
                Some(c) if !is_whitespace(c) && !is_delimiter(c) => {
                    self.read_byte()?;
                    word_buf.push(c);
                }
                _ => break,
            }
        }

        if word_buf == b"stream" {
            // Skip the line ending after "stream" keyword
            match self.read_byte()? {
                Some(b'\r') => {
                    if self.peek_byte()? == Some(b'\n') {
                        self.read_byte()?;
                    }
                }
                Some(b'\n') => {}
                _ => {}
            }

            // Read stream data based on /Length.
            // /Length must be a direct integer; indirect references are not
            // supported at this stage (would require two-pass parsing).
            let length = dict
                .get_i32(b"Length")
                .ok_or_else(|| Error::InvalidPdf("stream missing integer /Length".into()))?
                as usize;
            let mut data = vec![0u8; length];
            self.reader.read_exact(&mut data)?;

            // Consume optional EOL then the required "endstream" keyword.
            match self.peek_byte()? {
                Some(b'\r') => {
                    self.read_byte()?;
                    if self.peek_byte()? == Some(b'\n') {
                        self.read_byte()?;
                    }
                }
                Some(b'\n') => {
                    self.read_byte()?;
                }
                _ => {}
            }
            let endstream = self.read_word()?;
            if endstream != b"endstream" {
                return Err(Error::InvalidPdf("expected 'endstream'".into()));
            }

            return Ok(PdfObject::Stream(PdfStream { dict, data }));
        }

        // Not a stream, restore position
        self.seek(saved_pos)?;
        Ok(PdfObject::Dictionary(dict))
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

    // --- Streams ---

    #[test]
    fn parse_stream_basic() {
        let input = b"<< /Length 5 >>\nstream\nhello\nendstream";
        let obj = parse(input);
        let stream = obj.as_stream().unwrap();
        assert_eq!(stream.data, b"hello");
    }

    #[test]
    fn parse_stream_missing_length_is_error() {
        let result = parse_err(b"<< >>\nstream\nhello\nendstream");
        assert!(matches!(result, Error::InvalidPdf(_)));
    }

    // --- Error cases ---

    #[test]
    fn parse_empty_is_error() {
        let result = parse_err(b"");
        assert!(matches!(result, Error::InvalidPdf(_)));
    }
}
