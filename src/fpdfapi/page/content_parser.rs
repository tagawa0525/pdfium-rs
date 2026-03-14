use std::collections::HashMap;
use std::io::{Read, Seek};

use crate::fpdfapi::font::pdf_font::PdfFont;
use crate::fpdfapi::page::graphics_state::GraphicsState;
use crate::fpdfapi::page::page_object::{CharEntry, PageObject, TextObject};
use crate::fpdfapi::parser::document::Document;
use crate::fpdfapi::parser::object::{PdfDictionary, PdfObject};
use crate::fxcrt::coordinates::{Matrix, Point};

// ── Token ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
enum Token {
    Number(f64),
    LiteralString(Vec<u8>),
    HexString(Vec<u8>),
    Name(Vec<u8>),
    Array(Vec<Token>),
    Keyword(Vec<u8>),
}

// ── Tokenizer ─────────────────────────────────────────────────────────────────

fn is_whitespace(b: u8) -> bool {
    matches!(b, 0 | 9 | 10 | 12 | 13 | 32)
}

fn is_delimiter(b: u8) -> bool {
    matches!(
        b,
        b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%'
    )
}

fn skip_whitespace_and_comments(data: &[u8], pos: &mut usize) {
    while *pos < data.len() {
        let b = data[*pos];
        if b == b'%' {
            // Skip to end of line
            while *pos < data.len() && data[*pos] != b'\n' && data[*pos] != b'\r' {
                *pos += 1;
            }
        } else if is_whitespace(b) {
            *pos += 1;
        } else {
            break;
        }
    }
}

fn read_literal_string(data: &[u8], pos: &mut usize) -> Vec<u8> {
    // Caller consumed '('
    let mut result = Vec::new();
    let mut depth = 1usize;
    while *pos < data.len() {
        let b = data[*pos];
        *pos += 1;
        match b {
            b'(' => {
                depth += 1;
                result.push(b);
            }
            b')' => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
                result.push(b);
            }
            b'\\' if *pos < data.len() => {
                let escaped = data[*pos];
                *pos += 1;
                match escaped {
                    b'n' => result.push(b'\n'),
                    b'r' => result.push(b'\r'),
                    b't' => result.push(b'\t'),
                    b'b' => result.push(8),
                    b'f' => result.push(12),
                    b'(' => result.push(b'('),
                    b')' => result.push(b')'),
                    b'\\' => result.push(b'\\'),
                    // Line continuation: \r\n (CRLF) must consume both bytes
                    b'\r' => {
                        if *pos < data.len() && data[*pos] == b'\n' {
                            *pos += 1;
                        }
                    }
                    b'\n' => {} // LF line continuation
                    b'0'..=b'7' => {
                        // octal escape
                        let mut val = (escaped - b'0') as u32;
                        for _ in 0..2 {
                            if *pos < data.len() && data[*pos].is_ascii_digit() {
                                let d = data[*pos] - b'0';
                                if d < 8 {
                                    val = val * 8 + d as u32;
                                    *pos += 1;
                                } else {
                                    break;
                                }
                            } else {
                                break;
                            }
                        }
                        result.push(val as u8);
                    }
                    other => result.push(other),
                }
            }
            other => result.push(other),
        }
    }
    result
}

fn hex_digit(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn read_hex_string(data: &[u8], pos: &mut usize) -> Vec<u8> {
    // Caller consumed '<'
    let mut result = Vec::new();
    let mut high: Option<u8> = None;
    while *pos < data.len() {
        let b = data[*pos];
        *pos += 1;
        if b == b'>' {
            break;
        }
        if is_whitespace(b) {
            continue;
        }
        if let Some(d) = hex_digit(b) {
            match high {
                None => high = Some(d),
                Some(h) => {
                    result.push(h << 4 | d);
                    high = None;
                }
            }
        }
    }
    if let Some(h) = high {
        result.push(h << 4); // odd nibble count: pad with 0
    }
    result
}

fn read_name(data: &[u8], pos: &mut usize) -> Vec<u8> {
    // Caller consumed '/'
    let mut name = Vec::new();
    while *pos < data.len() {
        let b = data[*pos];
        if is_whitespace(b) || is_delimiter(b) {
            break;
        }
        *pos += 1;
        if b == b'#' && *pos + 1 < data.len() {
            // Hex escape in name
            if let (Some(h), Some(l)) = (hex_digit(data[*pos]), hex_digit(data[*pos + 1])) {
                name.push(h << 4 | l);
                *pos += 2;
                continue;
            }
        }
        name.push(b);
    }
    name
}

fn read_number_or_keyword(data: &[u8], pos: &mut usize) -> Token {
    let start = *pos;
    while *pos < data.len() && !is_whitespace(data[*pos]) && !is_delimiter(data[*pos]) {
        *pos += 1;
    }
    // Guarantee forward progress: if the current byte is an unhandled delimiter
    // (e.g. `>`, `>>`, stray `]`), consume it to prevent infinite loops.
    if *pos == start && *pos < data.len() {
        *pos += 1;
    }
    let token_bytes = &data[start..*pos];

    // Try to parse as number
    if let Ok(s) = std::str::from_utf8(token_bytes)
        && let Ok(f) = s.parse::<f64>()
    {
        return Token::Number(f);
    }
    Token::Keyword(token_bytes.to_vec())
}

fn read_array(data: &[u8], pos: &mut usize) -> Vec<Token> {
    // Caller consumed '['
    let mut items = Vec::new();
    loop {
        skip_whitespace_and_comments(data, pos);
        if *pos >= data.len() || data[*pos] == b']' {
            if *pos < data.len() {
                *pos += 1; // consume ']'
            }
            break;
        }
        if let Some(tok) = next_token(data, pos) {
            items.push(tok);
        }
    }
    items
}

fn next_token(data: &[u8], pos: &mut usize) -> Option<Token> {
    skip_whitespace_and_comments(data, pos);
    if *pos >= data.len() {
        return None;
    }
    let b = data[*pos];
    *pos += 1;
    match b {
        b'(' => Some(Token::LiteralString(read_literal_string(data, pos))),
        b'<' if *pos < data.len() && data[*pos] != b'<' => {
            Some(Token::HexString(read_hex_string(data, pos)))
        }
        b'/' => Some(Token::Name(read_name(data, pos))),
        b'[' => Some(Token::Array(read_array(data, pos))),
        _ => {
            // Put byte back
            *pos -= 1;
            Some(read_number_or_keyword(data, pos))
        }
    }
}

// ── Parser ────────────────────────────────────────────────────────────────────

struct Parser<'a, R: Read + Seek> {
    data: &'a [u8],
    pos: usize,
    stack: Vec<Token>,
    gs: GraphicsState,
    gs_stack: Vec<GraphicsState>,
    font_cache: HashMap<Vec<u8>, PdfFont>,
    resources: PdfDictionary,
    doc: &'a mut Document<R>,
    objects: Vec<PageObject>,
    /// Whether we are inside a BT/ET block.
    in_bt: bool,
    /// Characters collected for the current text object.
    text_chars: Vec<CharEntry>,
    /// Text matrix at BT entry (recorded once per BT block).
    bt_text_matrix: Matrix,
    /// CTM at BT entry.
    bt_ctm: Matrix,
    /// Font active at BT entry (or font of first Tj within BT).
    bt_font: Option<PdfFont>,
    /// Font size at BT entry.
    bt_font_size: f64,
}

impl<'a, R: Read + Seek> Parser<'a, R> {
    fn new(data: &'a [u8], resources: PdfDictionary, doc: &'a mut Document<R>) -> Self {
        Parser {
            data,
            pos: 0,
            stack: Vec::new(),
            gs: GraphicsState::default(),
            gs_stack: Vec::new(),
            font_cache: HashMap::new(),
            resources,
            doc,
            objects: Vec::new(),
            in_bt: false,
            text_chars: Vec::new(),
            bt_text_matrix: Matrix::default(),
            bt_ctm: Matrix::default(),
            bt_font: None,
            bt_font_size: 0.0,
        }
    }

    fn run(mut self) -> Vec<PageObject> {
        loop {
            let Some(tok) = next_token(self.data, &mut self.pos) else {
                break;
            };
            match &tok {
                Token::Keyword(kw) => {
                    let op = kw.clone();
                    self.dispatch(&op);
                }
                _ => self.stack.push(tok),
            }
        }
        self.objects
    }

    fn dispatch(&mut self, op: &[u8]) {
        match op {
            // ── Graphics state ──────────────────────────────────────────────
            b"q" => {
                self.gs_stack.push(self.gs.clone());
            }
            b"Q" => {
                if let Some(saved) = self.gs_stack.pop() {
                    self.gs = saved;
                }
            }
            b"cm" => {
                // 6 numbers: a b c d e f
                if let (
                    Some(Token::Number(f)),
                    Some(Token::Number(e)),
                    Some(Token::Number(d)),
                    Some(Token::Number(c)),
                    Some(Token::Number(b)),
                    Some(Token::Number(a)),
                ) = (
                    self.stack.pop(),
                    self.stack.pop(),
                    self.stack.pop(),
                    self.stack.pop(),
                    self.stack.pop(),
                    self.stack.pop(),
                ) {
                    let m = Matrix::new(a as f32, b as f32, c as f32, d as f32, e as f32, f as f32);
                    self.gs.ctm.concat(&m);
                }
            }

            // ── Text object ──────────────────────────────────────────────────
            b"BT" => {
                self.in_bt = true;
                self.text_chars.clear();
                // Reset text matrix and position
                self.gs.text_matrix = Matrix::default();
                self.gs.text_pos = Point::default();
                self.gs.text_line_pos = Point::default();
                self.bt_ctm = self.gs.ctm;
                self.bt_text_matrix = self.gs.text_matrix;
                self.bt_font = self.gs.font.clone();
                self.bt_font_size = self.gs.text_state.font_size;
            }
            b"ET" => {
                if self.in_bt && !self.text_chars.is_empty() {
                    let font = self.bt_font.take().unwrap_or(PdfFont::Unsupported {
                        base_font: String::new(),
                    });
                    self.objects.push(PageObject::Text(Box::new(TextObject {
                        char_entries: std::mem::take(&mut self.text_chars),
                        font,
                        font_size: self.bt_font_size,
                        text_matrix: self.bt_text_matrix,
                        ctm: self.bt_ctm,
                    })));
                }
                self.in_bt = false;
                self.text_chars.clear();
            }

            // ── Text state ───────────────────────────────────────────────────
            b"Tf" => {
                // name size Tf
                if let (Some(Token::Number(size)), Some(Token::Name(name))) =
                    (self.stack.pop(), self.stack.pop())
                {
                    self.gs.text_state.font_size = size;
                    if self.in_bt {
                        self.bt_font_size = size;
                    }
                    self.load_font(name);
                }
            }
            b"Tc" => {
                if let Some(Token::Number(v)) = self.stack.pop() {
                    self.gs.text_state.char_space = v;
                }
            }
            b"Tw" => {
                if let Some(Token::Number(v)) = self.stack.pop() {
                    self.gs.text_state.word_space = v;
                }
            }
            b"Tz" => {
                if let Some(Token::Number(v)) = self.stack.pop() {
                    self.gs.text_horz_scale = v / 100.0;
                }
            }
            b"TL" => {
                if let Some(Token::Number(v)) = self.stack.pop() {
                    self.gs.text_leading = v;
                }
            }
            b"Tr" => {
                if let Some(Token::Number(v)) = self.stack.pop() {
                    self.gs.text_state.text_rendering_mode = v as u8;
                }
            }
            b"Ts" => {
                if let Some(Token::Number(v)) = self.stack.pop() {
                    self.gs.text_rise = v;
                }
            }

            // ── Text positioning ─────────────────────────────────────────────
            b"Td" => {
                if let (Some(Token::Number(dy)), Some(Token::Number(dx))) =
                    (self.stack.pop(), self.stack.pop())
                {
                    self.gs.move_text_point(dx, dy);
                }
            }
            b"TD" => {
                // Td + set TL to -ty
                if let (Some(Token::Number(dy)), Some(Token::Number(dx))) =
                    (self.stack.pop(), self.stack.pop())
                {
                    self.gs.text_leading = -dy;
                    self.gs.move_text_point(dx, dy);
                }
            }
            b"Tm" => {
                if let (
                    Some(Token::Number(f)),
                    Some(Token::Number(e)),
                    Some(Token::Number(d)),
                    Some(Token::Number(c)),
                    Some(Token::Number(b)),
                    Some(Token::Number(a)),
                ) = (
                    self.stack.pop(),
                    self.stack.pop(),
                    self.stack.pop(),
                    self.stack.pop(),
                    self.stack.pop(),
                    self.stack.pop(),
                ) {
                    self.gs.set_text_matrix(a, b, c, d, e, f);
                    self.bt_text_matrix = self.gs.text_matrix;
                }
            }
            b"T*" => {
                self.gs.move_to_next_line();
            }

            // ── Text showing ─────────────────────────────────────────────────
            b"Tj" => {
                if let Some(Token::LiteralString(s) | Token::HexString(s)) = self.stack.pop() {
                    self.show_string(&s);
                }
            }
            b"TJ" => {
                if let Some(Token::Array(items)) = self.stack.pop() {
                    for item in items {
                        match item {
                            Token::LiteralString(s) | Token::HexString(s) => {
                                self.show_string(&s);
                            }
                            Token::Number(n) => {
                                let font_size = self.gs.text_state.font_size;
                                let horz = self.gs.text_horz_scale;
                                let dx = -n / 1000.0 * font_size * horz;
                                self.gs.advance_text_position(dx);
                            }
                            _ => {}
                        }
                    }
                }
            }
            b"'" => {
                // move to next line + Tj
                self.gs.move_to_next_line();
                if let Some(Token::LiteralString(s) | Token::HexString(s)) = self.stack.pop() {
                    self.show_string(&s);
                }
            }
            b"\"" => {
                // word_space char_space string "
                if let (
                    Some(Token::LiteralString(s) | Token::HexString(s)),
                    Some(Token::Number(char_space)),
                    Some(Token::Number(word_space)),
                ) = (self.stack.pop(), self.stack.pop(), self.stack.pop())
                {
                    self.gs.text_state.word_space = word_space;
                    self.gs.text_state.char_space = char_space;
                    self.gs.move_to_next_line();
                    self.show_string(&s);
                }
            }

            // ── Inline image — skip until EI ─────────────────────────────────
            b"BI" => {
                self.skip_inline_image();
            }

            // ── Everything else: silently ignore ─────────────────────────────
            _ => {}
        }
        self.stack.clear();
    }

    /// Load a font by resource name (raw bytes) into `gs.font` and update `bt_font`.
    fn load_font(&mut self, name: Vec<u8>) {
        // Check cache first
        if let Some(cached) = self.font_cache.get(&name) {
            let font = cached.clone();
            if self.in_bt {
                self.bt_font = Some(font.clone());
            }
            self.gs.font = Some(font);
            return;
        }

        // Lookup font object in resources (clone to release borrow).
        // Use raw bytes for the key to avoid UTF-8 conversion that could corrupt
        // non-ASCII PDF name bytes.
        let font_obj_opt = self
            .resources
            .get(b"Font")
            .and_then(|o| o.as_dict())
            .and_then(|d| d.get(&name))
            .cloned();

        let Some(font_obj) = font_obj_opt else {
            return;
        };

        // Resolve indirect reference if necessary
        let dict_opt: Option<crate::fpdfapi::parser::object::PdfDictionary> = match font_obj {
            PdfObject::Dictionary(d) => Some(d),
            PdfObject::Reference(id) => self
                .doc
                .object(id.num)
                .ok()
                .and_then(|o| o.as_dict().cloned()),
            _ => None,
        };

        let Some(dict) = dict_opt else {
            return;
        };

        if let Ok(font) = PdfFont::load(&dict, self.doc) {
            self.font_cache.insert(name, font.clone());
            if self.in_bt {
                self.bt_font = Some(font.clone());
            }
            self.gs.font = Some(font);
        }
    }

    /// Render a byte string as CharEntry items, advancing text position.
    fn show_string(&mut self, bytes: &[u8]) {
        if !self.in_bt {
            return;
        }
        let font = match &self.gs.font {
            Some(f) => f.clone(),
            None => return,
        };

        let font_size = self.gs.text_state.font_size;
        let char_space = self.gs.text_state.char_space;
        let word_space = self.gs.text_state.word_space;
        let horz = self.gs.text_horz_scale;

        for &byte in bytes {
            let code = byte as u32;

            // Character origin in user space: CTM × text_matrix × (text_pos + rise)
            let text_pt = Point::new(
                self.gs.text_pos.x,
                self.gs.text_pos.y + self.gs.text_rise as f32,
            );
            let after_tm = self.gs.text_matrix.transform_point(text_pt);
            let origin = self.gs.ctm.transform_point(after_tm);

            let char_width = font.char_width(code);

            self.text_chars.push(CharEntry {
                code,
                origin,
                width: char_width,
            });

            // Advance text position
            let advance = (char_width / 1000.0 * font_size + char_space) * horz;
            let extra = if code == 0x20 { word_space * horz } else { 0.0 };
            self.gs.advance_text_position(advance + extra);
        }
    }

    /// Skip inline image data (BI ... ID <data> EI).
    fn skip_inline_image(&mut self) {
        // Scan for the keyword ID to find start of image data
        loop {
            skip_whitespace_and_comments(self.data, &mut self.pos);
            if self.pos >= self.data.len() {
                break;
            }
            let Some(tok) = next_token(self.data, &mut self.pos) else {
                break;
            };
            if matches!(tok, Token::Keyword(ref k) if k == b"ID") {
                break;
            }
        }
        // Skip the single whitespace byte that separates ID from image data (PDF spec).
        if self.pos < self.data.len() && is_whitespace(self.data[self.pos]) {
            self.pos += 1;
        }
        // Scan for whitespace + EI + (whitespace | delimiter | EOF).
        // Requiring whitespace before EI reduces false positives in binary image data.
        while self.pos < self.data.len() {
            if is_whitespace(self.data[self.pos])
                && self.pos + 2 < self.data.len()
                && self.data[self.pos + 1] == b'E'
                && self.data[self.pos + 2] == b'I'
            {
                let after = self.pos + 3;
                if after >= self.data.len()
                    || is_whitespace(self.data[after])
                    || is_delimiter(self.data[after])
                {
                    self.pos = after;
                    break;
                }
            }
            self.pos += 1;
        }
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Parse a PDF content stream buffer and return the page objects it defines.
///
/// `resources` is the page's /Resources dictionary (used for font lookup).
/// `doc` is the document, needed to resolve indirect font references.
pub fn parse_content_stream<R: Read + Seek>(
    data: &[u8],
    resources: &PdfDictionary,
    doc: &mut Document<R>,
) -> Vec<PageObject> {
    let parser = Parser::new(data, resources.clone(), doc);
    parser.run()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fpdfapi::parser::document::Document;
    use crate::fpdfapi::parser::object::{PdfDictionary, PdfObject};
    use crate::fxcrt::bytestring::PdfByteString;
    use std::io::Cursor;

    fn minimal_pdf() -> Vec<u8> {
        let mut pdf = Vec::new();
        pdf.extend_from_slice(b"%PDF-1.4\n");
        let obj1_off = pdf.len();
        pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
        let obj2_off = pdf.len();
        pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [] /Count 0 >>\nendobj\n");
        let xref_off = pdf.len();
        pdf.extend_from_slice(b"xref\n0 3\n");
        pdf.extend_from_slice(b"0000000000 65535 f \n");
        pdf.extend_from_slice(format!("{:010} 00000 n \n", obj1_off).as_bytes());
        pdf.extend_from_slice(format!("{:010} 00000 n \n", obj2_off).as_bytes());
        pdf.extend_from_slice(b"trailer\n<< /Size 3 /Root 1 0 R >>\n");
        pdf.extend_from_slice(format!("startxref\n{xref_off}\n%%EOF\n").as_bytes());
        pdf
    }

    fn make_doc() -> Document<Cursor<Vec<u8>>> {
        Document::from_reader(Cursor::new(minimal_pdf())).unwrap()
    }

    /// Build minimal /Resources with an inline Type1 font at /F1.
    fn resources_with_type1() -> PdfDictionary {
        let mut font = PdfDictionary::new();
        font.set("Type", PdfObject::Name(PdfByteString::from("Font")));
        font.set("Subtype", PdfObject::Name(PdfByteString::from("Type1")));
        font.set(
            "BaseFont",
            PdfObject::Name(PdfByteString::from("Helvetica")),
        );
        font.set(
            "Encoding",
            PdfObject::Name(PdfByteString::from("WinAnsiEncoding")),
        );
        font.set("FirstChar", PdfObject::Integer(65));
        font.set("LastChar", PdfObject::Integer(66));
        font.set(
            "Widths",
            PdfObject::Array(vec![
                PdfObject::Integer(722), // A
                PdfObject::Integer(667), // B
            ]),
        );

        let mut font_map = PdfDictionary::new();
        font_map.set("F1", PdfObject::Dictionary(font));

        let mut resources = PdfDictionary::new();
        resources.set("Font", PdfObject::Dictionary(font_map));
        resources
    }

    #[test]
    fn empty_stream_returns_empty() {
        let mut doc = make_doc();
        let result = parse_content_stream(b"", &PdfDictionary::new(), &mut doc);
        assert!(result.is_empty());
    }

    #[test]
    fn whitespace_only_stream_returns_empty() {
        let mut doc = make_doc();
        let result = parse_content_stream(b"  \n\r\n  ", &PdfDictionary::new(), &mut doc);
        assert!(result.is_empty());
    }

    #[test]
    fn bt_et_no_text_produces_no_objects() {
        let mut doc = make_doc();
        // BT/ET without any Tj should produce no objects (nothing to render)
        let result = parse_content_stream(b"BT ET", &PdfDictionary::new(), &mut doc);
        assert!(result.is_empty());
    }

    #[test]
    fn tj_creates_char_entries_for_each_byte() {
        let mut doc = make_doc();
        let resources = resources_with_type1();
        // /F1 12 Tf sets font; (AB) Tj renders 2 chars
        let stream = b"BT /F1 12 Tf (AB) Tj ET";
        let result = parse_content_stream(stream, &resources, &mut doc);
        assert_eq!(result.len(), 1);
        if let PageObject::Text(obj) = &result[0] {
            assert_eq!(obj.char_entries.len(), 2);
            assert_eq!(obj.char_entries[0].code, 65); // 'A'
            assert_eq!(obj.char_entries[1].code, 66); // 'B'
        } else {
            panic!("expected PageObject::Text");
        }
    }

    #[test]
    fn char_entries_have_correct_widths() {
        let mut doc = make_doc();
        let resources = resources_with_type1();
        let stream = b"BT /F1 10 Tf (AB) Tj ET";
        let result = parse_content_stream(stream, &resources, &mut doc);
        assert_eq!(result.len(), 1);
        if let PageObject::Text(obj) = &result[0] {
            // A: width=722 font units
            assert!((obj.char_entries[0].width - 722.0).abs() < 1e-6);
            // B: width=667 font units
            assert!((obj.char_entries[1].width - 667.0).abs() < 1e-6);
        } else {
            panic!("expected PageObject::Text");
        }
    }

    #[test]
    fn td_moves_origin_of_subsequent_chars() {
        let mut doc = make_doc();
        let resources = resources_with_type1();
        // After 100 200 Td, text_line_pos = (100, 200)
        // With identity matrices, char origin should be near (100, 200)
        let stream = b"BT /F1 10 Tf 100 200 Td (A) Tj ET";
        let result = parse_content_stream(stream, &resources, &mut doc);
        assert_eq!(result.len(), 1);
        if let PageObject::Text(obj) = &result[0] {
            assert_eq!(obj.char_entries.len(), 1);
            let origin = obj.char_entries[0].origin;
            assert!((origin.x - 100.0).abs() < 1e-3, "x={}", origin.x);
            assert!((origin.y - 200.0).abs() < 1e-3, "y={}", origin.y);
        } else {
            panic!("expected PageObject::Text");
        }
    }

    #[test]
    fn tm_sets_text_matrix_for_char_origin() {
        let mut doc = make_doc();
        let resources = resources_with_type1();
        // 1 0 0 1 50 75 Tm → text matrix translate to (50, 75)
        let stream = b"BT /F1 10 Tf 1 0 0 1 50 75 Tm (A) Tj ET";
        let result = parse_content_stream(stream, &resources, &mut doc);
        assert_eq!(result.len(), 1);
        if let PageObject::Text(obj) = &result[0] {
            let origin = obj.char_entries[0].origin;
            assert!((origin.x - 50.0).abs() < 1e-3, "x={}", origin.x);
            assert!((origin.y - 75.0).abs() < 1e-3, "y={}", origin.y);
        } else {
            panic!("expected PageObject::Text");
        }
    }

    #[test]
    fn q_q_restores_graphics_state() {
        // After q/Q, the graphics state is restored.
        // We can't easily observe this from PageObjects, but we can verify
        // that chars rendered after Q use the pre-q state.
        let mut doc = make_doc();
        let resources = resources_with_type1();
        // Set font, save state, change position, restore, then render
        let stream = b"BT /F1 10 Tf 1 0 0 1 10 20 Tm q 1 0 0 1 999 999 Tm Q (A) Tj ET";
        let result = parse_content_stream(stream, &resources, &mut doc);
        assert_eq!(result.len(), 1);
        if let PageObject::Text(obj) = &result[0] {
            let origin = obj.char_entries[0].origin;
            // After Q restores, text_matrix from before q is back: (10, 20)
            assert!((origin.x - 10.0).abs() < 1e-3, "x={}", origin.x);
            assert!((origin.y - 20.0).abs() < 1e-3, "y={}", origin.y);
        } else {
            panic!("expected PageObject::Text");
        }
    }

    #[test]
    fn tj_array_with_kerning_adjusts_position() {
        let mut doc = make_doc();
        let resources = resources_with_type1();
        // [(A) -1000 (B)] TJ: A at 0, then +10 kerning adjustment (size=10)
        let stream = b"BT /F1 10 Tf [(A) -1000 (B)] TJ ET";
        let result = parse_content_stream(stream, &resources, &mut doc);
        assert_eq!(result.len(), 1);
        if let PageObject::Text(obj) = &result[0] {
            assert_eq!(obj.char_entries.len(), 2);
            let a_x = obj.char_entries[0].origin.x;
            let b_x = obj.char_entries[1].origin.x;
            // A advance: 722/1000*10 = 7.22; TJ -1000: dx = 1000/1000*10 = 10
            // B origin = 7.22 + 10 = 17.22 > a_x (0.0)
            assert!(b_x > a_x, "B should be right of A, a_x={a_x}, b_x={b_x}");
        } else {
            panic!("expected PageObject::Text");
        }
    }

    #[test]
    fn multiple_bt_et_blocks_produce_multiple_objects() {
        let mut doc = make_doc();
        let resources = resources_with_type1();
        let stream = b"BT /F1 10 Tf (A) Tj ET BT /F1 10 Tf (B) Tj ET";
        let result = parse_content_stream(stream, &resources, &mut doc);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn unknown_operators_are_skipped() {
        let mut doc = make_doc();
        let resources = resources_with_type1();
        // Path operators should not crash or produce objects
        let stream = b"1 0 0 RG 0 0 100 100 re f BT /F1 10 Tf (A) Tj ET";
        let result = parse_content_stream(stream, &resources, &mut doc);
        assert_eq!(result.len(), 1);
    }
}
