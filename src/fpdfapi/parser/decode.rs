use crate::error::{Error, Result};
use crate::fpdfapi::parser::object::{PdfDictionary, PdfObject};
use crate::fxcodec::{ascii_hex, ascii85, flate, lzw};

/// Maximum size of a decoded stream. Protects against OOM from malicious or
/// corrupt PDFs with degenerate compression ratios.
/// `wasm32-unknown-unknown` (browser) uses 64 MiB because browser memory is
/// more constrained; all other targets use 256 MiB.
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
const MAX_DECODED_SIZE: usize = 64 * 1024 * 1024;
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
const MAX_DECODED_SIZE: usize = 256 * 1024 * 1024;

/// Decode a raw stream by applying its filter pipeline.
///
/// Reads `/Filter` (name or array) and `/DecodeParms` (dict or array)
/// from the stream dictionary and applies the codec chain in order.
///
/// Filter arrays are applied left-to-right during decoding (outermost first).
pub fn decode_stream(data: &[u8], dict: &PdfDictionary) -> Result<Vec<u8>> {
    let filters = collect_filters(dict)?;
    if filters.is_empty() {
        return Ok(data.to_vec());
    }

    // parms_list is guaranteed to have the same length as filters.
    let parms_list = collect_decode_parms(dict, filters.len());
    debug_assert_eq!(parms_list.len(), filters.len());

    let mut buf = data.to_vec();
    for (filter, parms) in filters.iter().zip(parms_list.iter()) {
        buf = apply_filter(&buf, filter, parms.as_ref())?;
        if buf.len() > MAX_DECODED_SIZE {
            return Err(Error::InvalidPdf(format!(
                "decode_stream: decoded size {} exceeds {MAX_DECODED_SIZE} byte limit",
                buf.len()
            )));
        }
    }
    Ok(buf)
}

/// Collect filter names from /Filter (name or array).
fn collect_filters(dict: &PdfDictionary) -> Result<Vec<Filter>> {
    match dict.get(b"Filter") {
        None => Ok(Vec::new()),
        Some(PdfObject::Name(name)) => {
            let f = Filter::from_name(name.as_bytes()).ok_or_else(|| {
                Error::InvalidPdf(format!(
                    "decode_stream: unsupported filter {:?}",
                    name.as_bytes()
                ))
            })?;
            Ok(vec![f])
        }
        Some(PdfObject::Array(arr)) => arr
            .iter()
            .map(|obj| match obj {
                PdfObject::Name(name) => Filter::from_name(name.as_bytes()).ok_or_else(|| {
                    Error::InvalidPdf(format!(
                        "decode_stream: unsupported filter {:?}",
                        name.as_bytes()
                    ))
                }),
                _ => Err(Error::InvalidPdf(
                    "decode_stream: /Filter array element is not a name".into(),
                )),
            })
            .collect(),
        _ => Err(Error::InvalidPdf(
            "decode_stream: /Filter is not a name or array".into(),
        )),
    }
}

/// Collect /DecodeParms entries corresponding to each filter.
/// Returns a Vec of Option<PdfDictionary>, one per filter.
fn collect_decode_parms(dict: &PdfDictionary, count: usize) -> Vec<Option<PdfDictionary>> {
    match dict.get(b"DecodeParms") {
        None => vec![None; count],
        Some(PdfObject::Dictionary(d)) => {
            let mut result = vec![None; count];
            if !result.is_empty() {
                result[0] = Some(d.clone());
            }
            result
        }
        Some(PdfObject::Array(arr)) => {
            let mut result: Vec<Option<PdfDictionary>> = arr
                .iter()
                .map(|obj| match obj {
                    PdfObject::Dictionary(d) => Some(d.clone()),
                    _ => None, // null or missing entry
                })
                .collect();
            // Ensure parms_list has the same length as the filter list.
            result.resize(count, None);
            result
        }
        _ => vec![None; count],
    }
}

fn apply_filter(data: &[u8], filter: &Filter, parms: Option<&PdfDictionary>) -> Result<Vec<u8>> {
    match filter {
        Filter::ASCIIHexDecode => ascii_hex::decode(data),
        Filter::ASCII85Decode => ascii85::decode(data),
        Filter::FlateDecode => {
            let predictor = parms.and_then(parse_flate_predictor);
            flate::decode(data, predictor)
        }
        Filter::LZWDecode => {
            let early_change = parms
                .and_then(|p| p.get(b"EarlyChange"))
                .and_then(|o| o.as_i32())
                .map(|v| v != 0)
                .unwrap_or(true);
            lzw::decode(data, early_change)
        }
    }
}

/// Parse FlateDecode predictor from /DecodeParms dictionary.
fn parse_flate_predictor(parms: &PdfDictionary) -> Option<flate::Predictor> {
    let predictor = parms.get(b"Predictor")?.as_i32()?;
    match predictor {
        1 => Some(flate::Predictor::None),
        2 => Some(flate::Predictor::Tiff),
        10..=15 => {
            let colors = parms
                .get(b"Colors")
                .and_then(|o| o.as_i32())
                .unwrap_or(1)
                .clamp(1, 255) as u8;
            let bits_per_component = parms
                .get(b"BitsPerComponent")
                .and_then(|o| o.as_i32())
                .unwrap_or(8)
                .clamp(1, 16) as u8;
            let columns = parms
                .get(b"Columns")
                .and_then(|o| o.as_i32())
                .unwrap_or(1)
                .clamp(1, 65535) as u16;
            Some(flate::Predictor::Png {
                colors,
                bits_per_component,
                columns,
            })
        }
        _ => None,
    }
}

/// Known PDF stream filters.
// Variant names mirror PDF spec filter names exactly; the shared "Decode" suffix is intentional.
#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, PartialEq, Eq)]
enum Filter {
    ASCIIHexDecode,
    ASCII85Decode,
    FlateDecode,
    LZWDecode,
}

impl Filter {
    fn from_name(name: &[u8]) -> Option<Self> {
        match name {
            b"ASCIIHexDecode" | b"AHx" => Some(Filter::ASCIIHexDecode),
            b"ASCII85Decode" | b"A85" => Some(Filter::ASCII85Decode),
            b"FlateDecode" | b"Fl" => Some(Filter::FlateDecode),
            b"LZWDecode" | b"LZW" => Some(Filter::LZWDecode),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fpdfapi::parser::object::{PdfDictionary, PdfObject};
    use crate::fxcrt::bytestring::PdfByteString;

    fn dict_with_filter(filter: &[u8]) -> PdfDictionary {
        let mut dict = PdfDictionary::new();
        dict.set(
            PdfByteString::from(b"Filter".as_slice()),
            PdfObject::Name(PdfByteString::from(filter)),
        );
        dict
    }

    fn dict_with_filters(filters: &[&[u8]]) -> PdfDictionary {
        let mut dict = PdfDictionary::new();
        let arr: Vec<PdfObject> = filters
            .iter()
            .map(|&f| PdfObject::Name(PdfByteString::from(f)))
            .collect();
        dict.set(
            PdfByteString::from(b"Filter".as_slice()),
            PdfObject::Array(arr),
        );
        dict
    }

    #[test]
    fn decode_no_filter() {
        let data = b"raw bytes";
        let dict = PdfDictionary::new();
        let result = decode_stream(data, &dict).unwrap();
        assert_eq!(result, data);
    }

    #[test]
    fn decode_ascii_hex() {
        let data = b"48656C6C6F>";
        let dict = dict_with_filter(b"ASCIIHexDecode");
        let result = decode_stream(data, &dict).unwrap();
        assert_eq!(result, b"Hello");
    }

    #[test]
    fn decode_ascii85() {
        let data = b"FCfN8~>";
        let dict = dict_with_filter(b"ASCII85Decode");
        let result = decode_stream(data, &dict).unwrap();
        assert_eq!(result, b"test");
    }

    #[test]
    fn decode_flate() {
        use std::io::Write;
        let mut encoder =
            flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(b"Hello, PDF!").unwrap();
        let compressed = encoder.finish().unwrap();

        let dict = dict_with_filter(b"FlateDecode");
        let result = decode_stream(&compressed, &dict).unwrap();
        assert_eq!(result, b"Hello, PDF!");
    }

    #[test]
    fn decode_lzw() {
        // Encode "ABABAB" as LZW: Clear(256) A(65) B(66) AB(258) AB(258) EOD(257)
        // Using 9-bit MSB-first codes (simple fixed-width for this test)
        let lzw_data = encode_lzw_simple(&[256, 65, 66, 258, 258, 257]);
        let dict = dict_with_filter(b"LZWDecode");
        let result = decode_stream(&lzw_data, &dict).unwrap();
        assert_eq!(result, b"ABABAB");
    }

    #[test]
    fn decode_chained_filters() {
        // PDF filter chain: [ASCII85Decode, FlateDecode]
        // Decoding order: ASCII85 first, then Flate
        // Encoding order (for test setup): Flate first, then ASCII85
        use std::io::Write;
        let original = b"chained";
        let mut encoder =
            flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(original).unwrap();
        let compressed = encoder.finish().unwrap();
        let encoded = ascii85_encode(&compressed);

        let dict = dict_with_filters(&[b"ASCII85Decode", b"FlateDecode"]);
        let result = decode_stream(&encoded, &dict).unwrap();
        assert_eq!(result, original);
    }

    #[test]
    fn decode_unknown_filter_is_error() {
        let data = b"data";
        let dict = dict_with_filter(b"RunLengthDecode");
        let result = decode_stream(data, &dict);
        assert!(result.is_err());
    }

    #[test]
    fn decode_flate_with_png_predictor() {
        use std::io::Write;
        // 2-column, 8-bit grayscale, 2 rows: [10,20] and [30,40]
        // PNG Up predictor (filter type 2) applied before FlateDecode encoding
        let original: Vec<u8> = vec![10, 20, 30, 40];
        // Predicted: row1=[2,10,20] row2=[2,20,20] (Up: row2-row1)
        let predicted: Vec<u8> = vec![2, 10, 20, 2, 20, 20];
        let mut encoder =
            flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(&predicted).unwrap();
        let compressed = encoder.finish().unwrap();

        let mut parms = PdfDictionary::new();
        parms.set(b"Predictor".as_slice(), PdfObject::Integer(12)); // PNG Up
        parms.set(b"Colors".as_slice(), PdfObject::Integer(1));
        parms.set(b"BitsPerComponent".as_slice(), PdfObject::Integer(8));
        parms.set(b"Columns".as_slice(), PdfObject::Integer(2));

        let mut dict = PdfDictionary::new();
        dict.set(
            b"Filter".as_slice(),
            PdfObject::Name(PdfByteString::from(b"FlateDecode".as_slice())),
        );
        dict.set(b"DecodeParms".as_slice(), PdfObject::Dictionary(parms));

        let result = decode_stream(&compressed, &dict).unwrap();
        assert_eq!(result, original);
    }

    /// LZW encoder for test use. Encodes codes using fixed 9-bit MSB-first packing.
    fn encode_lzw_simple(codes: &[u16]) -> Vec<u8> {
        let mut bits = Vec::new();
        for &code in codes {
            for bit_idx in (0u8..9).rev() {
                bits.push(((code >> bit_idx) & 1) as u8);
            }
        }
        let mut bytes = Vec::new();
        for chunk in bits.chunks(8) {
            let mut byte = 0u8;
            for (i, &bit) in chunk.iter().enumerate() {
                byte |= bit << (7 - i);
            }
            bytes.push(byte);
        }
        bytes
    }

    /// Simple ASCII85 encoder for test use only.
    fn ascii85_encode(data: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        for chunk in data.chunks(4) {
            let mut group = [0u8; 4];
            for (i, &b) in chunk.iter().enumerate() {
                group[i] = b;
            }
            let value = u32::from_be_bytes(group);
            if chunk.len() == 4 && value == 0 {
                out.push(b'z');
                continue;
            }
            let mut v = value;
            let mut digits = [0u8; 5];
            for i in (0..5).rev() {
                digits[i] = (v % 85) as u8;
                v /= 85;
            }
            let n = chunk.len() + 1;
            for &d in &digits[..n] {
                out.push(d + b'!');
            }
        }
        out.extend_from_slice(b"~>");
        out
    }
}
