use std::io::{Read, Seek};

use crate::error::{Error, Result};
use crate::fpdfapi::page::color_space::ColorSpace;
use crate::fpdfapi::page::page_object::ImageObject;
use crate::fpdfapi::parser::decode::decode_stream;
use crate::fpdfapi::parser::document::Document;
use crate::fpdfapi::parser::object::PdfDictionary;
use crate::fxcrt::coordinates::Matrix;

/// Decode an image XObject stream into an `ImageObject` with RGBA-8888 pixel data.
///
/// Reads `/Width`, `/Height`, `/ColorSpace`, `/BitsPerComponent` from `dict`,
/// applies the filter pipeline (via `decode_stream`), then converts raw pixels
/// to RGBA-8888. `/SMask` and multi-component color spaces beyond DeviceGray/RGB/CMYK
/// are not yet supported.
///
/// `ctm` is the current transformation matrix from the content stream context
/// (`Do` operator) and is stored verbatim in the returned `ImageObject`.
pub fn decode_image_xobject<R: Read + Seek>(
    stream_data: &[u8],
    dict: &PdfDictionary,
    ctm: Matrix,
    _doc: &mut Document<R>,
) -> Result<ImageObject> {
    let width_i = dict
        .get(b"Width")
        .and_then(|o| o.as_i32())
        .ok_or_else(|| Error::InvalidPdf("image XObject missing /Width".into()))?;
    if width_i <= 0 {
        return Err(Error::InvalidPdf(format!(
            "image XObject: invalid /Width {width_i}"
        )));
    }
    let width = width_i as u32;

    let height_i = dict
        .get(b"Height")
        .and_then(|o| o.as_i32())
        .ok_or_else(|| Error::InvalidPdf("image XObject missing /Height".into()))?;
    if height_i <= 0 {
        return Err(Error::InvalidPdf(format!(
            "image XObject: invalid /Height {height_i}"
        )));
    }
    let height = height_i as u32;
    let bpc = dict
        .get(b"BitsPerComponent")
        .and_then(|o| o.as_i32())
        .unwrap_or(8) as u32;

    let color_space = dict
        .get(b"ColorSpace")
        .and_then(|o| o.as_name())
        .map(|name| name_to_color_space(name.as_bytes()))
        .unwrap_or(Some(ColorSpace::DeviceRGB))
        .unwrap_or(ColorSpace::DeviceRGB);

    // Decode filter pipeline (DCTDecode, FlateDecode, etc.)
    let raw = decode_stream(stream_data, dict)?;

    // Convert raw pixel data to RGBA-8888
    let rgba = raw_to_rgba(&raw, width, height, bpc, color_space)?;

    Ok(ImageObject {
        data: rgba,
        width,
        height,
        ctm,
    })
}

/// Convert raw image bytes to RGBA-8888.
fn raw_to_rgba(
    raw: &[u8],
    width: u32,
    height: u32,
    bpc: u32,
    color_space: ColorSpace,
) -> Result<Vec<u8>> {
    let n_pixels = (width * height) as usize;
    let components = color_space.num_components();
    let mut rgba = Vec::with_capacity(n_pixels * 4);

    match bpc {
        8 => {
            // Each component is 1 byte; stride = components per pixel.
            let expected = n_pixels * components;
            if raw.len() < expected {
                return Err(Error::InvalidPdf(format!(
                    "image XObject: expected {expected} bytes, got {}",
                    raw.len()
                )));
            }
            for chunk in raw[..expected].chunks(components) {
                let (r, g, b) = pixel_to_rgb(chunk, color_space);
                rgba.push(r);
                rgba.push(g);
                rgba.push(b);
                rgba.push(255); // fully opaque
            }
        }
        1 => {
            // 1-bit monochrome: 8 pixels packed per byte, MSB first.
            for y in 0..height {
                let row_bytes = width.div_ceil(8);
                let row_start = (y * row_bytes) as usize;
                for x in 0..width {
                    let byte_idx = row_start + (x / 8) as usize;
                    let bit = if byte_idx < raw.len() {
                        (raw[byte_idx] >> (7 - (x % 8))) & 1
                    } else {
                        0
                    };
                    let v = if bit == 1 { 255u8 } else { 0u8 };
                    rgba.push(v);
                    rgba.push(v);
                    rgba.push(v);
                    rgba.push(255);
                }
            }
        }
        _ => {
            return Err(Error::InvalidPdf(format!(
                "image XObject: unsupported BitsPerComponent {bpc}"
            )));
        }
    }

    Ok(rgba)
}

/// Convert a pixel's component bytes to (R, G, B) u8 values.
fn pixel_to_rgb(components: &[u8], color_space: ColorSpace) -> (u8, u8, u8) {
    match color_space {
        ColorSpace::DeviceGray => {
            let v = components[0];
            (v, v, v)
        }
        ColorSpace::DeviceRGB => {
            let r = components.first().copied().unwrap_or(0);
            let g = components.get(1).copied().unwrap_or(0);
            let b = components.get(2).copied().unwrap_or(0);
            (r, g, b)
        }
        ColorSpace::DeviceCMYK => {
            // Simple CMYK → RGB approximation
            let c = components.first().copied().unwrap_or(0) as f32 / 255.0;
            let m = components.get(1).copied().unwrap_or(0) as f32 / 255.0;
            let y = components.get(2).copied().unwrap_or(0) as f32 / 255.0;
            let k = components.get(3).copied().unwrap_or(0) as f32 / 255.0;
            let r = ((1.0 - c) * (1.0 - k) * 255.0).round().clamp(0.0, 255.0) as u8;
            let g = ((1.0 - m) * (1.0 - k) * 255.0).round().clamp(0.0, 255.0) as u8;
            let b = ((1.0 - y) * (1.0 - k) * 255.0).round().clamp(0.0, 255.0) as u8;
            (r, g, b)
        }
    }
}

/// Map a color space name to a `ColorSpace` enum value.
fn name_to_color_space(name: &[u8]) -> Option<ColorSpace> {
    match name {
        b"DeviceGray" | b"G" => Some(ColorSpace::DeviceGray),
        b"DeviceRGB" | b"RGB" => Some(ColorSpace::DeviceRGB),
        b"DeviceCMYK" | b"CMYK" => Some(ColorSpace::DeviceCMYK),
        _ => None,
    }
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
