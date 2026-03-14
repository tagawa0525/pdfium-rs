use crate::fxcrt::coordinates::Rect;

use super::page_object::PageObject;

/// A parsed PDF page.
pub struct Page {
    /// Visible media area in default user space (points).
    pub media_box: Rect,
    /// Clip box, if different from `media_box`.
    pub crop_box: Option<Rect>,
    /// Rotation in degrees clockwise (0, 90, 180, 270).
    pub rotation: u16,
    /// Parsed content objects in rendering order.
    pub objects: Vec<PageObject>,
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use crate::fpdfapi::parser::document::Document;

    /// Minimal single-page PDF with an inherited MediaBox, empty content stream.
    fn single_page_pdf() -> Vec<u8> {
        let mut pdf = Vec::new();
        pdf.extend_from_slice(b"%PDF-1.4\n");

        // Object 1: Catalog
        let obj1_off = pdf.len();
        pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

        // Object 2: Pages (MediaBox inherited by all kids)
        let obj2_off = pdf.len();
        pdf.extend_from_slice(
            b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 612 792] >>\nendobj\n",
        );

        // Object 3: Page (inherits MediaBox from parent)
        let obj3_off = pdf.len();
        pdf.extend_from_slice(
            b"3 0 obj\n<< /Type /Page /Parent 2 0 R /Contents 4 0 R /Resources << >> >>\nendobj\n",
        );

        // Object 4: Content stream (empty)
        let obj4_off = pdf.len();
        pdf.extend_from_slice(b"4 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n");

        // Xref
        let xref_off = pdf.len();
        pdf.extend_from_slice(b"xref\n0 5\n");
        pdf.extend_from_slice(b"0000000000 65535 f \n");
        pdf.extend_from_slice(format!("{:010} 00000 n \n", obj1_off).as_bytes());
        pdf.extend_from_slice(format!("{:010} 00000 n \n", obj2_off).as_bytes());
        pdf.extend_from_slice(format!("{:010} 00000 n \n", obj3_off).as_bytes());
        pdf.extend_from_slice(format!("{:010} 00000 n \n", obj4_off).as_bytes());

        // Trailer
        pdf.extend_from_slice(b"trailer\n<< /Size 5 /Root 1 0 R >>\n");
        pdf.extend_from_slice(format!("startxref\n{xref_off}\n%%EOF\n").as_bytes());

        pdf
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn page_media_box_inherited_from_pages_node() {
        let data = single_page_pdf();
        let mut doc = Document::from_reader(Cursor::new(data)).unwrap();
        let page = doc.page(0).unwrap();
        assert_eq!(page.media_box.left, 0.0);
        assert_eq!(page.media_box.bottom, 0.0);
        assert_eq!(page.media_box.right, 612.0);
        assert_eq!(page.media_box.top, 792.0);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn page_out_of_bounds_is_error() {
        let data = single_page_pdf();
        let mut doc = Document::from_reader(Cursor::new(data)).unwrap();
        assert!(doc.page(1).is_err());
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn page_empty_content_has_no_objects() {
        let data = single_page_pdf();
        let mut doc = Document::from_reader(Cursor::new(data)).unwrap();
        let page = doc.page(0).unwrap();
        assert!(page.objects.is_empty());
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn page_rotation_defaults_to_zero() {
        let data = single_page_pdf();
        let mut doc = Document::from_reader(Cursor::new(data)).unwrap();
        let page = doc.page(0).unwrap();
        assert_eq!(page.rotation, 0);
    }
}
