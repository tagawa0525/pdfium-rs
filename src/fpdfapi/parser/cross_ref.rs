use std::collections::HashMap;
use std::io::{Read, Seek};

use crate::error::Result;
use crate::fpdfapi::parser::object::PdfDictionary;

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

impl CrossRefTable {
    /// Parse cross-reference table(s) starting from the given xref offset.
    /// Follows `/Prev` links for incremental updates.
    pub fn parse<R: Read + Seek>(reader: &mut R, xref_offset: u64) -> Result<Self> {
        todo!()
    }
}

/// Find the `startxref` offset from the end of a PDF file.
pub fn find_startxref<R: Read + Seek>(reader: &mut R) -> Result<u64> {
    todo!()
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
