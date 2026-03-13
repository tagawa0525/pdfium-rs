use crate::error::{Error, Result};
use flate2::read::ZlibDecoder;
use std::io::Read;

/// Decode FlateDecode (zlib/deflate) compressed data.
///
/// PDF uses zlib-wrapped deflate (RFC 1950). The optional `predictor`
/// parameter enables PNG/TIFF row filtering for image data.
pub fn decode(input: &[u8], predictor: Option<Predictor>) -> Result<Vec<u8>> {
    let mut decoder = ZlibDecoder::new(input);
    let mut decompressed = Vec::new();
    decoder
        .read_to_end(&mut decompressed)
        .map_err(|e| Error::InvalidPdf(format!("FlateDecode: {e}")))?;

    match predictor {
        None | Some(Predictor::None) => Ok(decompressed),
        Some(Predictor::Tiff) => Err(Error::InvalidPdf(
            "FlateDecode: TIFF predictor not yet supported".into(),
        )),
        Some(Predictor::Png {
            colors,
            bits_per_component,
            columns,
        }) => reverse_png_predictor(&decompressed, colors, bits_per_component, columns),
    }
}

/// Predictor algorithm as specified in PDF's DecodeParms /Predictor entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Predictor {
    /// No prediction (default, Predictor = 1).
    None,
    /// TIFF Predictor 2: horizontal differencing.
    Tiff,
    /// PNG prediction (Predictor 10-15): one of None/Sub/Up/Average/Paeth or Optimum.
    Png {
        colors: u8,
        bits_per_component: u8,
        columns: u16,
    },
}

/// Reverse PNG row prediction. Each row starts with a 1-byte filter type.
fn reverse_png_predictor(
    data: &[u8],
    colors: u8,
    bits_per_component: u8,
    columns: u16,
) -> Result<Vec<u8>> {
    if colors == 0 || bits_per_component == 0 || columns == 0 {
        return Err(Error::InvalidPdf(
            "FlateDecode PNG predictor: colors, bits_per_component, and columns must be non-zero"
                .into(),
        ));
    }

    // bytes_per_pixel: for Sub/Average/Paeth left-neighbor distance
    let bytes_per_pixel = (colors as usize * bits_per_component as usize).div_ceil(8);
    // row_stride: actual byte width of one row (accounts for bit-packed samples)
    let row_stride = (columns as usize * colors as usize * bits_per_component as usize).div_ceil(8);
    // Each stored row: 1 filter byte + row_stride data bytes
    let stored_row_len = 1 + row_stride;

    if !data.len().is_multiple_of(stored_row_len) {
        return Err(Error::InvalidPdf(format!(
            "FlateDecode PNG predictor: data length {} not divisible by row size {}",
            data.len(),
            stored_row_len
        )));
    }

    let mut output = Vec::with_capacity(data.len() / stored_row_len * row_stride);
    let mut prev_row = vec![0u8; row_stride];

    for chunk in data.chunks(stored_row_len) {
        let filter_type = chunk[0];
        let raw = &chunk[1..];
        let mut row = vec![0u8; row_stride];

        match filter_type {
            0 => {
                // None
                row.copy_from_slice(raw);
            }
            1 => {
                // Sub
                for i in 0..row_stride {
                    let left = if i < bytes_per_pixel {
                        0
                    } else {
                        row[i - bytes_per_pixel]
                    };
                    row[i] = raw[i].wrapping_add(left);
                }
            }
            2 => {
                // Up
                for i in 0..row_stride {
                    row[i] = raw[i].wrapping_add(prev_row[i]);
                }
            }
            3 => {
                // Average
                for i in 0..row_stride {
                    let left = if i < bytes_per_pixel {
                        0
                    } else {
                        row[i - bytes_per_pixel]
                    };
                    let up = prev_row[i];
                    row[i] = raw[i].wrapping_add(((left as u16 + up as u16) / 2) as u8);
                }
            }
            4 => {
                // Paeth
                for i in 0..row_stride {
                    let left = if i < bytes_per_pixel {
                        0
                    } else {
                        row[i - bytes_per_pixel]
                    };
                    let up = prev_row[i];
                    let up_left = if i < bytes_per_pixel {
                        0
                    } else {
                        prev_row[i - bytes_per_pixel]
                    };
                    row[i] = raw[i].wrapping_add(paeth_predictor(left, up, up_left));
                }
            }
            t => {
                return Err(Error::InvalidPdf(format!(
                    "FlateDecode PNG predictor: unknown filter type {t}"
                )));
            }
        }

        output.extend_from_slice(&row);
        prev_row = row;
    }

    Ok(output)
}

fn paeth_predictor(a: u8, b: u8, c: u8) -> u8 {
    let a = a as i16;
    let b = b as i16;
    let c = c as i16;
    let p = a + b - c;
    let pa = (p - a).abs();
    let pb = (p - b).abs();
    let pc = (p - c).abs();
    if pa <= pb && pa <= pc {
        a as u8
    } else if pb <= pc {
        b as u8
    } else {
        c as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zlib_compress(data: &[u8]) -> Vec<u8> {
        use std::io::Write;
        let mut encoder =
            flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(data).unwrap();
        encoder.finish().unwrap()
    }

    #[test]
    fn decode_simple() {
        let original = b"Hello, PDF World!";
        let compressed = zlib_compress(original);
        let result = decode(&compressed, None).unwrap();
        assert_eq!(result, original);
    }

    #[test]
    fn decode_empty() {
        let compressed = zlib_compress(b"");
        let result = decode(&compressed, None).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn decode_large_data() {
        let original: Vec<u8> = (0u8..=255).cycle().take(10_000).collect();
        let compressed = zlib_compress(&original);
        let result = decode(&compressed, None).unwrap();
        assert_eq!(result, original);
    }

    #[test]
    fn decode_invalid_data() {
        let result = decode(b"not zlib data", None);
        assert!(result.is_err());
    }

    #[test]
    fn decode_with_png_predictor_sub() {
        // 3-column, 1-byte-per-component, 3-color (RGB) image row
        // Row filter type 1 (Sub): each byte is XOR'd with the byte to its left
        let original: Vec<u8> = vec![10, 20, 30, 40, 50, 60]; // 2 RGB pixels
        let predicted = apply_png_predictor_sub(&original, 3, 1, 3);
        let compressed = zlib_compress(&predicted);
        let result = decode(
            &compressed,
            Some(Predictor::Png {
                colors: 3,
                bits_per_component: 8,
                columns: 2,
            }),
        )
        .unwrap();
        assert_eq!(result, original);
    }

    #[test]
    fn decode_with_png_predictor_up() {
        // 2-row, 3-column grayscale image
        let original: Vec<u8> = vec![10, 20, 30, 40, 50, 60]; // row1 + row2
        let predicted = apply_png_predictor_up(&original, 1, 1, 3);
        let compressed = zlib_compress(&predicted);
        let result = decode(
            &compressed,
            Some(Predictor::Png {
                colors: 1,
                bits_per_component: 8,
                columns: 3,
            }),
        )
        .unwrap();
        assert_eq!(result, original);
    }

    /// Helper: apply PNG Sub predictor (filter type 1) to image data.
    /// Prepends filter-type byte (1) to each row.
    fn apply_png_predictor_sub(
        data: &[u8],
        colors: usize,
        _bits: usize,
        columns: usize,
    ) -> Vec<u8> {
        let row_len = columns * colors;
        let mut out = Vec::new();
        for row in data.chunks(row_len) {
            out.push(1); // Sub filter type
            for (i, &b) in row.iter().enumerate() {
                let prev = if i < colors { 0 } else { row[i - colors] };
                out.push(b.wrapping_sub(prev));
            }
        }
        out
    }

    /// Helper: apply PNG Up predictor (filter type 2) to image data.
    /// Prepends filter-type byte (2) to each row.
    fn apply_png_predictor_up(
        data: &[u8],
        _colors: usize,
        _bits: usize,
        columns: usize,
    ) -> Vec<u8> {
        let row_len = columns;
        let mut out = Vec::new();
        let mut prev_row = vec![0u8; row_len];
        for row in data.chunks(row_len) {
            out.push(2); // Up filter type
            for (i, &b) in row.iter().enumerate() {
                out.push(b.wrapping_sub(prev_row[i]));
            }
            prev_row = row.to_vec();
        }
        out
    }
}
