/// Font file data extracted from a PDF FontDescriptor.
#[derive(Debug, Clone)]
pub enum FontData {
    /// TrueType font data (from `/FontFile2` stream).
    TrueType(Vec<u8>),
    /// Type1 font data in PFB format (from `/FontFile` stream).
    Type1(Vec<u8>),
    /// OpenType/CFF font data (from `/FontFile3` stream).
    OpenType(Vec<u8>),
}
