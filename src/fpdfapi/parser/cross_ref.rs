use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};

use crate::error::{Error, Result};
use crate::fpdfapi::parser::object::{PdfDictionary, PdfObject};

/// Entry in the cross-reference table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XRefEntry {
    /// Object at given byte offset.
    Used { offset: u64, gen_num: u16 },
    /// Free (deleted) object.
    Free { next_free: u32, gen_num: u16 },
    /// Object stored in an object stream (PDF 1.5+).
    Compressed { stream_obj_num: u32, index: u32 },
}

/// Parsed cross-reference table + trailer dictionary.
pub struct CrossRefTable {
    pub entries: HashMap<u32, XRefEntry>,
    pub trailer: PdfDictionary,
}

/// Maximum bytes to read for the xref+trailer section.
/// Prevents excessive memory use on large PDFs; the xref+trailer
/// section is typically a few KB and never exceeds a few MB.
const MAX_XREF_READ: u64 = 2 * 1024 * 1024; // 2 MiB

impl CrossRefTable {
    /// Parse cross-reference table(s) starting from the given xref offset.
    /// Reads up to [`MAX_XREF_READ`] bytes from the xref offset into memory.
    pub fn parse<R: Read + Seek>(reader: &mut R, xref_offset: u64) -> Result<Self> {
        reader.seek(SeekFrom::Start(xref_offset))?;

        // Bound the read to avoid a memory spike on large PDFs.
        let file_end = reader.seek(SeekFrom::End(0))?;
        reader.seek(SeekFrom::Start(xref_offset))?;
        let available = file_end.saturating_sub(xref_offset).min(MAX_XREF_READ);

        let mut data = vec![0u8; available as usize];
        reader.read_exact(&mut data)?;

        // Parse xref table from the in-memory buffer
        Self::parse_from_bytes(&data)
    }

    fn parse_from_bytes(data: &[u8]) -> Result<Self> {
        let mut pos = 0;

        // Expect "xref"
        if !data[pos..].starts_with(b"xref") {
            return Err(Error::InvalidPdf("expected 'xref' keyword".into()));
        }
        pos += 4;

        // Skip whitespace
        while pos < data.len() && data[pos].is_ascii_whitespace() {
            pos += 1;
        }

        let mut entries = HashMap::new();

        // Parse sections until "trailer"
        loop {
            if data[pos..].starts_with(b"trailer") {
                break;
            }

            // Read "first_obj count\n"
            let (first_obj, new_pos) = read_int(&data[pos..])?;
            pos += new_pos;
            skip_ws(data, &mut pos);
            let (count, new_pos) = read_int(&data[pos..])?;
            pos += new_pos;
            skip_ws(data, &mut pos);

            // Parse each 20-byte entry
            for i in 0..count as u32 {
                let obj_num = first_obj as u32 + i;

                let (offset, new_pos) = read_int(&data[pos..])?;
                pos += new_pos;
                skip_ws(data, &mut pos);
                let (gen_num, new_pos) = read_int(&data[pos..])?;
                pos += new_pos;
                skip_ws(data, &mut pos);

                // Read type: 'f' or 'n'
                let entry_type = data.get(pos).copied().unwrap_or(b'n');
                pos += 1;
                skip_ws(data, &mut pos);

                let entry = if entry_type == b'f' {
                    XRefEntry::Free {
                        next_free: offset as u32,
                        gen_num: gen_num as u16,
                    }
                } else {
                    XRefEntry::Used {
                        offset: offset as u64,
                        gen_num: gen_num as u16,
                    }
                };

                entries.entry(obj_num).or_insert(entry);
            }
        }

        // Skip "trailer" keyword
        pos += 7; // "trailer"
        skip_ws(data, &mut pos);

        // Parse trailer dictionary using SyntaxParser
        use crate::fpdfapi::parser::syntax::SyntaxParser;
        use std::io::Cursor;

        let trailer_data = &data[pos..];
        let mut parser = SyntaxParser::new(Cursor::new(trailer_data.to_vec()))?;
        let trailer_obj = parser.read_object()?;
        let trailer = match trailer_obj {
            PdfObject::Dictionary(d) => d,
            _ => return Err(Error::InvalidPdf("trailer must be a dictionary".into())),
        };

        Ok(CrossRefTable { entries, trailer })
    }
}

fn skip_ws(data: &[u8], pos: &mut usize) {
    while *pos < data.len() && data[*pos].is_ascii_whitespace() {
        *pos += 1;
    }
}

fn read_int(data: &[u8]) -> Result<(i64, usize)> {
    let mut i = 0;
    let mut negative = false;

    if i < data.len() && data[i] == b'-' {
        negative = true;
        i += 1;
    } else if i < data.len() && data[i] == b'+' {
        i += 1;
    }

    let start = i;
    while i < data.len() && data[i].is_ascii_digit() {
        i += 1;
    }

    if i == start {
        return Err(Error::InvalidPdf("expected integer".into()));
    }

    let s = std::str::from_utf8(&data[start..i])
        .map_err(|_| Error::InvalidPdf("invalid integer".into()))?;
    let val: i64 = s
        .parse()
        .map_err(|_| Error::InvalidPdf("invalid integer".into()))?;

    Ok((if negative { -val } else { val }, i))
}

/// Find the `startxref` offset from the end of a PDF file.
pub fn find_startxref<R: Read + Seek>(reader: &mut R) -> Result<u64> {
    // Read the last 1024 bytes (or entire file if smaller)
    let file_len = reader.seek(SeekFrom::End(0))?;
    let search_start = file_len.saturating_sub(1024);
    reader.seek(SeekFrom::Start(search_start))?;

    let mut buf = Vec::new();
    reader.read_to_end(&mut buf)?;

    // Search backwards for "startxref"
    let needle = b"startxref";
    let pos = buf
        .windows(needle.len())
        .rposition(|w| w == needle)
        .ok_or_else(|| Error::InvalidPdf("'startxref' not found".into()))?;

    // Parse the first integer token after "startxref"
    let after = &buf[pos + needle.len()..];
    let mut i = 0;
    // Skip whitespace
    while i < after.len() && after[i].is_ascii_whitespace() {
        i += 1;
    }
    // Collect contiguous digits only
    let start = i;
    while i < after.len() && after[i].is_ascii_digit() {
        i += 1;
    }

    if i == start {
        return Err(Error::InvalidPdf("no offset after 'startxref'".into()));
    }

    let num_str = std::str::from_utf8(&after[start..i])
        .map_err(|_| Error::InvalidPdf("invalid startxref offset".into()))?;
    num_str
        .parse()
        .map_err(|_| Error::InvalidPdf("invalid startxref offset".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn find_startxref_simple() {
        let data = b"%PDF-1.4\nstartxref\n123\n%%EOF";
        let mut cursor = Cursor::new(data.to_vec());
        let offset = find_startxref(&mut cursor).unwrap();
        assert_eq!(offset, 123);
    }

    #[test]
    fn find_startxref_with_trailing_whitespace() {
        let data = b"%PDF-1.4\nstartxref\n456\n%%EOF\n";
        let mut cursor = Cursor::new(data.to_vec());
        let offset = find_startxref(&mut cursor).unwrap();
        assert_eq!(offset, 456);
    }

    #[test]
    fn find_startxref_missing() {
        let data = b"this is not a pdf";
        let mut cursor = Cursor::new(data.to_vec());
        let result = find_startxref(&mut cursor);
        assert!(result.is_err());
    }

    #[test]
    fn parse_xref_table() {
        let data = b"xref\n\
            0 3\n\
            0000000000 65535 f \n\
            0000000100 00000 n \n\
            0000000200 00000 n \n\
            trailer\n\
            << /Size 3 /Root 1 0 R >>\n\
            startxref\n\
            0\n\
            %%EOF";
        let mut cursor = Cursor::new(data.to_vec());
        let table = CrossRefTable::parse(&mut cursor, 0).unwrap();

        assert_eq!(table.entries.len(), 3);

        // Object 0 is free
        assert_eq!(
            table.entries[&0],
            XRefEntry::Free {
                next_free: 0,
                gen_num: 65535
            }
        );

        // Object 1 at offset 100
        assert_eq!(
            table.entries[&1],
            XRefEntry::Used {
                offset: 100,
                gen_num: 0
            }
        );

        // Object 2 at offset 200
        assert_eq!(
            table.entries[&2],
            XRefEntry::Used {
                offset: 200,
                gen_num: 0
            }
        );

        // Trailer
        assert_eq!(table.trailer.get_i32(b"Size"), Some(3));
    }

    #[test]
    fn parse_xref_multiple_sections() {
        let data = b"xref\n\
            0 2\n\
            0000000000 65535 f \n\
            0000000100 00000 n \n\
            5 1\n\
            0000000500 00000 n \n\
            trailer\n\
            << /Size 6 >>\n\
            startxref\n\
            0\n\
            %%EOF";
        let mut cursor = Cursor::new(data.to_vec());
        let table = CrossRefTable::parse(&mut cursor, 0).unwrap();

        assert_eq!(table.entries.len(), 3);
        assert!(table.entries.contains_key(&0));
        assert!(table.entries.contains_key(&1));
        assert!(table.entries.contains_key(&5));
    }
}
