/// Integration tests ported from PDFium C++:
///   core/fpdfapi/parser/fpdf_parser_decode_unittest.cpp
///
/// Tests the stream decode pipeline (filter validation and chaining).
/// These test the end-to-end decode path through Document::decode_stream.
use std::io::Cursor;

use pdfium_rs::Document;

/// Ported from: ValidateDecoderPipeline (subset)
/// Verify that single-filter decode works end-to-end via Document.
#[test]
#[ignore = "not yet implemented"]
fn decode_flate_via_document() {
    use std::io::Write;
    let original = b"Hello from PDFium integration test!";
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(original).unwrap();
    let compressed = encoder.finish().unwrap();

    // Build a minimal PDF with a FlateDecode stream as object 3
    let pdf = build_pdf_with_stream(b"FlateDecode", &compressed);
    let mut doc = Document::from_reader(Cursor::new(pdf)).unwrap();

    // Get the stream object and decode it
    let obj = doc.object(3).unwrap().clone();
    let stream = obj.as_stream().unwrap();
    let decoded = doc.decode_stream(stream, 3, 0).unwrap();
    assert_eq!(decoded, original);
}

/// Ported from: ValidateDecoderPipeline (chained filters)
/// ASCII85 → Flate chain through Document.
#[test]
#[ignore = "not yet implemented"]
fn decode_chained_ascii85_flate_via_document() {
    use std::io::Write;
    let original = b"chained filter test from PDFium";
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(original).unwrap();
    let compressed = encoder.finish().unwrap();
    let ascii85 = ascii85_encode(&compressed);

    // Build a PDF with chained [/ASCII85Decode /FlateDecode] stream
    let pdf = build_pdf_with_chained_stream(&[b"ASCII85Decode", b"FlateDecode"], &ascii85);
    let mut doc = Document::from_reader(Cursor::new(pdf)).unwrap();

    let obj = doc.object(3).unwrap().clone();
    let stream = obj.as_stream().unwrap();
    let decoded = doc.decode_stream(stream, 3, 0).unwrap();
    assert_eq!(decoded, original);
}

/// Ported from: A85Decode test vectors
/// "FCfN8~>" → "test"
#[test]
#[ignore = "not yet implemented"]
fn decode_ascii85_stream_via_document() {
    let encoded = b"FCfN8~>";
    let pdf = build_pdf_with_stream(b"ASCII85Decode", encoded);
    let mut doc = Document::from_reader(Cursor::new(pdf)).unwrap();

    let obj = doc.object(3).unwrap().clone();
    let stream = obj.as_stream().unwrap();
    let decoded = doc.decode_stream(stream, 3, 0).unwrap();
    assert_eq!(decoded, b"test");
}

/// Ported from: HexDecode test vectors
/// "48656C6C6F>" → "Hello"
#[test]
#[ignore = "not yet implemented"]
fn decode_hex_stream_via_document() {
    let encoded = b"48656C6C6F>";
    let pdf = build_pdf_with_stream(b"ASCIIHexDecode", encoded);
    let mut doc = Document::from_reader(Cursor::new(pdf)).unwrap();

    let obj = doc.object(3).unwrap().clone();
    let stream = obj.as_stream().unwrap();
    let decoded = doc.decode_stream(stream, 3, 0).unwrap();
    assert_eq!(decoded, b"Hello");
}

/// Ported from: HexDecode - whitespace handling
/// "12 Ac\t02\r\nBF>" → [0x12, 0xAC, 0x02, 0xBF]
#[test]
#[ignore = "not yet implemented"]
fn decode_hex_stream_with_whitespace() {
    let encoded = b"12 Ac\t02\r\nBF>";
    let pdf = build_pdf_with_stream(b"ASCIIHexDecode", encoded);
    let mut doc = Document::from_reader(Cursor::new(pdf)).unwrap();

    let obj = doc.object(3).unwrap().clone();
    let stream = obj.as_stream().unwrap();
    let decoded = doc.decode_stream(stream, 3, 0).unwrap();
    assert_eq!(decoded, &[0x12, 0xAC, 0x02, 0xBF]);
}

/// No filter — raw data passthrough.
#[test]
#[ignore = "not yet implemented"]
fn decode_no_filter_via_document() {
    let raw = b"raw data no filter";
    let pdf = build_pdf_with_raw_stream(raw);
    let mut doc = Document::from_reader(Cursor::new(pdf)).unwrap();

    let obj = doc.object(3).unwrap().clone();
    let stream = obj.as_stream().unwrap();
    let decoded = doc.decode_stream(stream, 3, 0).unwrap();
    assert_eq!(decoded, raw.as_slice());
}

// --- Helpers ---

/// Build a minimal PDF with a single-filter stream as object 3.
fn build_pdf_with_stream(filter_name: &[u8], data: &[u8]) -> Vec<u8> {
    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.4\n");

    let obj1_offset = pdf.len();
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    let obj2_offset = pdf.len();
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [] /Count 0 >>\nendobj\n");

    let obj3_offset = pdf.len();
    pdf.extend_from_slice(
        format!(
            "3 0 obj\n<< /Filter /{} /Length {} >>\nstream\n",
            String::from_utf8_lossy(filter_name),
            data.len()
        )
        .as_bytes(),
    );
    pdf.extend_from_slice(data);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");

    let xref_offset = pdf.len();
    pdf.extend_from_slice(b"xref\n0 4\n");
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    pdf.extend_from_slice(format!("{:010} 00000 n \n", obj1_offset).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", obj2_offset).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", obj3_offset).as_bytes());
    pdf.extend_from_slice(b"trailer\n<< /Size 4 /Root 1 0 R >>\n");
    pdf.extend_from_slice(format!("startxref\n{xref_offset}\n%%EOF\n").as_bytes());

    pdf
}

/// Build a minimal PDF with chained filter stream as object 3.
fn build_pdf_with_chained_stream(filters: &[&[u8]], data: &[u8]) -> Vec<u8> {
    let filter_array: String = filters
        .iter()
        .map(|f| format!("/{}", String::from_utf8_lossy(f)))
        .collect::<Vec<_>>()
        .join(" ");

    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.4\n");

    let obj1_offset = pdf.len();
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    let obj2_offset = pdf.len();
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [] /Count 0 >>\nendobj\n");

    let obj3_offset = pdf.len();
    pdf.extend_from_slice(
        format!(
            "3 0 obj\n<< /Filter [{filter_array}] /Length {} >>\nstream\n",
            data.len()
        )
        .as_bytes(),
    );
    pdf.extend_from_slice(data);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");

    let xref_offset = pdf.len();
    pdf.extend_from_slice(b"xref\n0 4\n");
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    pdf.extend_from_slice(format!("{:010} 00000 n \n", obj1_offset).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", obj2_offset).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", obj3_offset).as_bytes());
    pdf.extend_from_slice(b"trailer\n<< /Size 4 /Root 1 0 R >>\n");
    pdf.extend_from_slice(format!("startxref\n{xref_offset}\n%%EOF\n").as_bytes());

    pdf
}

/// Build a minimal PDF with a raw (no filter) stream as object 3.
fn build_pdf_with_raw_stream(data: &[u8]) -> Vec<u8> {
    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.4\n");

    let obj1_offset = pdf.len();
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    let obj2_offset = pdf.len();
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [] /Count 0 >>\nendobj\n");

    let obj3_offset = pdf.len();
    pdf.extend_from_slice(format!("3 0 obj\n<< /Length {} >>\nstream\n", data.len()).as_bytes());
    pdf.extend_from_slice(data);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");

    let xref_offset = pdf.len();
    pdf.extend_from_slice(b"xref\n0 4\n");
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    pdf.extend_from_slice(format!("{:010} 00000 n \n", obj1_offset).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", obj2_offset).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", obj3_offset).as_bytes());
    pdf.extend_from_slice(b"trailer\n<< /Size 4 /Root 1 0 R >>\n");
    pdf.extend_from_slice(format!("startxref\n{xref_offset}\n%%EOF\n").as_bytes());

    pdf
}

/// Simple ASCII85 encoder for test use.
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
