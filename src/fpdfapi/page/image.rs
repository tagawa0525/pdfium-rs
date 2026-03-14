use std::io::{Read, Seek};

use crate::error::Result;
use crate::fpdfapi::page::page_object::ImageObject;
use crate::fpdfapi::parser::document::Document;
use crate::fpdfapi::parser::object::PdfDictionary;
use crate::fxcrt::coordinates::Matrix;

/// Decode an image XObject stream into an `ImageObject` with RGBA pixel data.
///
/// Reads `/Width`, `/Height`, `/ColorSpace`, `/BitsPerComponent` from `dict`,
/// applies the filter pipeline (via `decode_stream`), then converts raw pixels
/// to RGBA-8888.
///
/// `ctm` is the current transformation matrix from the content stream context
/// (`Do` operator) and is stored verbatim in the returned `ImageObject`.
pub fn decode_image_xobject<R: Read + Seek>(
    _stream_data: &[u8],
    _dict: &PdfDictionary,
    _ctm: Matrix,
    _doc: &mut Document<R>,
) -> Result<ImageObject> {
    unimplemented!("image XObject decode not yet implemented")
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

    /// Build an image XObject dictionary for a 2×2 RGB image.
    fn rgb_2x2_dict() -> PdfDictionary {
        let mut dict = PdfDictionary::new();
        dict.set("Width", PdfObject::Integer(2));
        dict.set("Height", PdfObject::Integer(2));
        dict.set(
            "ColorSpace",
            PdfObject::Name(PdfByteString::from("DeviceRGB")),
        );
        dict.set("BitsPerComponent", PdfObject::Integer(8));
        dict
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn decode_rgb_2x2_image_xobject() {
        let mut doc = make_doc();
        // 2×2 RGB raw pixel data: red, green, blue, white
        let pixels: &[u8] = &[
            255, 0, 0, // red
            0, 255, 0, // green
            0, 0, 255, // blue
            255, 255, 255, // white
        ];
        let dict = rgb_2x2_dict();
        let obj = decode_image_xobject(pixels, &dict, Matrix::default(), &mut doc).unwrap();
        assert_eq!(obj.width, 2);
        assert_eq!(obj.height, 2);
        // RGBA output: 4 bytes per pixel × 4 pixels = 16 bytes
        assert_eq!(obj.data.len(), 16);
        // First pixel (red): R=255, G=0, B=0, A=255
        assert_eq!(&obj.data[0..4], &[255, 0, 0, 255]);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn decode_gray_1x1_image_xobject() {
        let mut doc = make_doc();
        // 1×1 gray, value 128
        let pixels: &[u8] = &[128];
        let mut dict = PdfDictionary::new();
        dict.set("Width", PdfObject::Integer(1));
        dict.set("Height", PdfObject::Integer(1));
        dict.set(
            "ColorSpace",
            PdfObject::Name(PdfByteString::from("DeviceGray")),
        );
        dict.set("BitsPerComponent", PdfObject::Integer(8));
        let obj = decode_image_xobject(pixels, &dict, Matrix::default(), &mut doc).unwrap();
        assert_eq!(obj.width, 1);
        assert_eq!(obj.height, 1);
        // Gray → RGBA: [128, 128, 128, 255]
        assert_eq!(&obj.data[0..4], &[128, 128, 128, 255]);
    }
}
