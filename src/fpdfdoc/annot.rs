use std::io::{Read, Seek};

use crate::error::{Error, Result};
use crate::fpdfapi::parser::document::Document;
use crate::fpdfapi::parser::object::{PdfDictionary, PdfObject};
use crate::fpdfdoc::action::Action;
use crate::fpdfdoc::util::decode_pdf_text_string;
use crate::fxcrt::coordinates::Rect;

/// Annotation subtype, derived from the `/Subtype` entry.
///
/// Corresponds to the annotation types enumerated in PDF spec §12.5.6.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnnotSubtype {
    Unknown,
    Text,
    Link,
    FreeText,
    Line,
    Square,
    Circle,
    Polygon,
    Polyline,
    Highlight,
    Underline,
    Squiggly,
    Strikeout,
    Stamp,
    Caret,
    Ink,
    Popup,
    FileAttachment,
    Sound,
    Movie,
    Widget,
    Screen,
    PrinterMark,
    TrapNet,
    Watermark,
    ThreeD,
    RichMedia,
    Redact,
}

impl AnnotSubtype {
    fn from_name(name: &[u8]) -> Self {
        match name {
            b"Text" => AnnotSubtype::Text,
            b"Link" => AnnotSubtype::Link,
            b"FreeText" => AnnotSubtype::FreeText,
            b"Line" => AnnotSubtype::Line,
            b"Square" => AnnotSubtype::Square,
            b"Circle" => AnnotSubtype::Circle,
            b"Polygon" => AnnotSubtype::Polygon,
            b"PolyLine" => AnnotSubtype::Polyline,
            b"Highlight" => AnnotSubtype::Highlight,
            b"Underline" => AnnotSubtype::Underline,
            b"Squiggly" => AnnotSubtype::Squiggly,
            b"StrikeOut" => AnnotSubtype::Strikeout,
            b"Stamp" => AnnotSubtype::Stamp,
            b"Caret" => AnnotSubtype::Caret,
            b"Ink" => AnnotSubtype::Ink,
            b"Popup" => AnnotSubtype::Popup,
            b"FileAttachment" => AnnotSubtype::FileAttachment,
            b"Sound" => AnnotSubtype::Sound,
            b"Movie" => AnnotSubtype::Movie,
            b"Widget" => AnnotSubtype::Widget,
            b"Screen" => AnnotSubtype::Screen,
            b"PrinterMark" => AnnotSubtype::PrinterMark,
            b"TrapNet" => AnnotSubtype::TrapNet,
            b"Watermark" => AnnotSubtype::Watermark,
            b"3D" => AnnotSubtype::ThreeD,
            b"RichMedia" => AnnotSubtype::RichMedia,
            b"Redact" => AnnotSubtype::Redact,
            _ => AnnotSubtype::Unknown,
        }
    }
}

/// Annotation flags (bitfield from `/F` entry).
///
/// Corresponds to C++ `CPDF_Annot` flag constants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AnnotFlags(pub u32);

impl AnnotFlags {
    pub fn invisible(&self) -> bool {
        self.0 & 1 != 0
    }
    pub fn hidden(&self) -> bool {
        self.0 & 2 != 0
    }
    pub fn print(&self) -> bool {
        self.0 & 4 != 0
    }
    pub fn no_zoom(&self) -> bool {
        self.0 & 8 != 0
    }
    pub fn no_rotate(&self) -> bool {
        self.0 & 16 != 0
    }
    pub fn no_view(&self) -> bool {
        self.0 & 32 != 0
    }
    pub fn read_only(&self) -> bool {
        self.0 & 64 != 0
    }
    pub fn locked(&self) -> bool {
        self.0 & 128 != 0
    }
    pub fn toggle_no_view(&self) -> bool {
        self.0 & 256 != 0
    }
    pub fn locked_contents(&self) -> bool {
        self.0 & 512 != 0
    }
}

/// A resolved PDF annotation.
///
/// Corresponds to a single entry in a page's `/Annots` array.
#[derive(Debug, Clone)]
pub struct Annotation {
    pub subtype: AnnotSubtype,
    pub rect: Rect,
    pub flags: AnnotFlags,
    pub contents: Option<String>,
    pub name: Option<String>,
    pub modified: Option<String>,
    pub action: Option<Action>,
    pub dict: PdfDictionary,
}

impl Annotation {
    fn from_dict(dict: PdfDictionary) -> Self {
        let subtype = dict
            .get_name(b"Subtype")
            .map(|n| AnnotSubtype::from_name(n.as_bytes()))
            .unwrap_or(AnnotSubtype::Unknown);

        let rect = dict
            .get_array(b"Rect")
            .and_then(|arr| {
                if arr.len() < 4 {
                    return None;
                }
                let l = arr[0].as_f64()? as f32;
                let b = arr[1].as_f64()? as f32;
                let r = arr[2].as_f64()? as f32;
                let t = arr[3].as_f64()? as f32;
                Some(Rect::new(l, b, r, t))
            })
            .unwrap_or_else(|| Rect::new(0.0, 0.0, 0.0, 0.0));

        let flags = AnnotFlags(dict.get_i32(b"F").unwrap_or(0).max(0) as u32);

        let contents = dict
            .get_string(b"Contents")
            .map(|s| decode_pdf_text_string(s.as_bytes()));

        let name = dict
            .get_string(b"NM")
            .map(|s| decode_pdf_text_string(s.as_bytes()));

        let modified = dict
            .get_string(b"M")
            .map(|s| decode_pdf_text_string(s.as_bytes()));

        // /A action — direct dict only; indirect refs are resolved by the caller
        let action = dict.get_dict(b"A").map(|d| Action::from_dict(d.clone()));

        Annotation {
            subtype,
            rect,
            flags,
            contents,
            name,
            modified,
            action,
            dict,
        }
    }
}

/// Extension trait that adds per-page annotation access to `Document`.
pub trait AnnotationsExt {
    /// Return all annotations on the given zero-based page index.
    ///
    /// Returns `Ok(vec![])` if the page has no `/Annots` entry.
    fn page_annotations(&mut self, page_index: u32) -> Result<Vec<Annotation>>;
}

impl<R: Read + Seek> AnnotationsExt for Document<R> {
    fn page_annotations(&mut self, page_index: u32) -> Result<Vec<Annotation>> {
        let page_dict = self.page_dict(page_index)?;

        // /Annots may be absent (no annotations) or an empty array
        let annots_obj = match page_dict.get(b"Annots") {
            Some(obj) => obj.clone(),
            None => return Ok(vec![]),
        };

        // Resolve indirect reference to the array if needed
        let annot_refs: Vec<u32> = match annots_obj {
            PdfObject::Array(arr) => arr
                .iter()
                .filter_map(|o| o.as_reference().map(|id| id.num))
                .collect(),
            PdfObject::Reference(id) => {
                // /Annots itself is an indirect reference to the array
                let arr = self
                    .object(id.num)?
                    .as_array()
                    .ok_or_else(|| {
                        Error::InvalidPdf(format!(
                            "/Annots reference {} does not resolve to an array",
                            id.num
                        ))
                    })?
                    .to_vec();
                arr.iter()
                    .filter_map(|o| o.as_reference().map(|r| r.num))
                    .collect()
            }
            _ => {
                return Err(Error::InvalidPdf(
                    "/Annots is not an array or reference".into(),
                ));
            }
        };

        let mut result = Vec::with_capacity(annot_refs.len());
        for ref_num in annot_refs {
            let dict = self
                .object(ref_num)?
                .as_dict()
                .ok_or_else(|| {
                    Error::InvalidPdf(format!("annotation object {ref_num} is not a dictionary"))
                })?
                .clone();

            // Resolve /A if it is an indirect reference
            let mut annot = Annotation::from_dict(dict.clone());
            if annot.action.is_none()
                && let Some(PdfObject::Reference(a_id)) = dict.get(b"A")
            {
                annot.action = self
                    .object(a_id.num)?
                    .as_dict()
                    .map(|d| Action::from_dict(d.clone()));
            }

            result.push(annot);
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_annot(subtype: &str) -> Annotation {
        let mut dict = PdfDictionary::new();
        use crate::fpdfapi::parser::object::PdfObject;
        use crate::fxcrt::bytestring::PdfByteString;
        dict.set("Subtype", PdfObject::Name(PdfByteString::from(subtype)));
        dict.set(
            "Rect",
            PdfObject::Array(vec![
                PdfObject::Real(0.0),
                PdfObject::Real(0.0),
                PdfObject::Real(100.0),
                PdfObject::Real(50.0),
            ]),
        );
        Annotation::from_dict(dict)
    }

    // --- AnnotSubtype from name ---

    #[test]
    fn subtype_text() {
        assert_eq!(make_annot("Text").subtype, AnnotSubtype::Text);
    }
    #[test]
    fn subtype_link() {
        assert_eq!(make_annot("Link").subtype, AnnotSubtype::Link);
    }
    #[test]
    fn subtype_free_text() {
        assert_eq!(make_annot("FreeText").subtype, AnnotSubtype::FreeText);
    }
    #[test]
    fn subtype_line() {
        assert_eq!(make_annot("Line").subtype, AnnotSubtype::Line);
    }
    #[test]
    fn subtype_square() {
        assert_eq!(make_annot("Square").subtype, AnnotSubtype::Square);
    }
    #[test]
    fn subtype_circle() {
        assert_eq!(make_annot("Circle").subtype, AnnotSubtype::Circle);
    }
    #[test]
    fn subtype_polygon() {
        assert_eq!(make_annot("Polygon").subtype, AnnotSubtype::Polygon);
    }
    #[test]
    fn subtype_polyline() {
        assert_eq!(make_annot("PolyLine").subtype, AnnotSubtype::Polyline);
    }
    #[test]
    fn subtype_highlight() {
        assert_eq!(make_annot("Highlight").subtype, AnnotSubtype::Highlight);
    }
    #[test]
    fn subtype_underline() {
        assert_eq!(make_annot("Underline").subtype, AnnotSubtype::Underline);
    }
    #[test]
    fn subtype_squiggly() {
        assert_eq!(make_annot("Squiggly").subtype, AnnotSubtype::Squiggly);
    }
    #[test]
    fn subtype_strikeout() {
        assert_eq!(make_annot("StrikeOut").subtype, AnnotSubtype::Strikeout);
    }
    #[test]
    fn subtype_stamp() {
        assert_eq!(make_annot("Stamp").subtype, AnnotSubtype::Stamp);
    }
    #[test]
    fn subtype_caret() {
        assert_eq!(make_annot("Caret").subtype, AnnotSubtype::Caret);
    }
    #[test]
    fn subtype_ink() {
        assert_eq!(make_annot("Ink").subtype, AnnotSubtype::Ink);
    }
    #[test]
    fn subtype_popup() {
        assert_eq!(make_annot("Popup").subtype, AnnotSubtype::Popup);
    }
    #[test]
    fn subtype_file_attachment() {
        assert_eq!(
            make_annot("FileAttachment").subtype,
            AnnotSubtype::FileAttachment
        );
    }
    #[test]
    fn subtype_sound() {
        assert_eq!(make_annot("Sound").subtype, AnnotSubtype::Sound);
    }
    #[test]
    fn subtype_movie() {
        assert_eq!(make_annot("Movie").subtype, AnnotSubtype::Movie);
    }
    #[test]
    fn subtype_widget() {
        assert_eq!(make_annot("Widget").subtype, AnnotSubtype::Widget);
    }
    #[test]
    fn subtype_screen() {
        assert_eq!(make_annot("Screen").subtype, AnnotSubtype::Screen);
    }
    #[test]
    fn subtype_printer_mark() {
        assert_eq!(make_annot("PrinterMark").subtype, AnnotSubtype::PrinterMark);
    }
    #[test]
    fn subtype_trap_net() {
        assert_eq!(make_annot("TrapNet").subtype, AnnotSubtype::TrapNet);
    }
    #[test]
    fn subtype_watermark() {
        assert_eq!(make_annot("Watermark").subtype, AnnotSubtype::Watermark);
    }
    #[test]
    fn subtype_three_d() {
        assert_eq!(make_annot("3D").subtype, AnnotSubtype::ThreeD);
    }
    #[test]
    fn subtype_rich_media() {
        assert_eq!(make_annot("RichMedia").subtype, AnnotSubtype::RichMedia);
    }
    #[test]
    fn subtype_redact() {
        assert_eq!(make_annot("Redact").subtype, AnnotSubtype::Redact);
    }
    #[test]
    fn subtype_unknown() {
        assert_eq!(make_annot("Bogus").subtype, AnnotSubtype::Unknown);
    }

    // --- AnnotFlags bit extraction ---

    #[test]
    fn flags_invisible_bit1() {
        let f = AnnotFlags(1);
        assert!(f.invisible());
        assert!(!f.hidden());
    }
    #[test]
    fn flags_hidden_bit2() {
        let f = AnnotFlags(2);
        assert!(f.hidden());
        assert!(!f.invisible());
    }
    #[test]
    fn flags_print_bit3() {
        let f = AnnotFlags(4);
        assert!(f.print());
    }
    #[test]
    fn flags_no_zoom_bit4() {
        assert!(AnnotFlags(8).no_zoom());
    }
    #[test]
    fn flags_no_rotate_bit5() {
        assert!(AnnotFlags(16).no_rotate());
    }
    #[test]
    fn flags_no_view_bit6() {
        assert!(AnnotFlags(32).no_view());
    }
    #[test]
    fn flags_read_only_bit7() {
        assert!(AnnotFlags(64).read_only());
    }
    #[test]
    fn flags_locked_bit8() {
        assert!(AnnotFlags(128).locked());
    }
    #[test]
    fn flags_toggle_no_view_bit9() {
        assert!(AnnotFlags(256).toggle_no_view());
    }
    #[test]
    fn flags_locked_contents_bit10() {
        assert!(AnnotFlags(512).locked_contents());
    }
    #[test]
    fn flags_combined() {
        let f = AnnotFlags(0b111);
        assert!(f.invisible());
        assert!(f.hidden());
        assert!(f.print());
        assert!(!f.no_zoom());
    }
    #[test]
    fn flags_zero_all_false() {
        let f = AnnotFlags(0);
        assert!(!f.invisible());
        assert!(!f.hidden());
        assert!(!f.print());
    }

    // --- Rect parsing ---

    #[test]
    fn rect_from_annot_dict() {
        let annot = make_annot("Text");
        assert!((annot.rect.right - 100.0f32).abs() < 1e-4);
        assert!((annot.rect.top - 50.0f32).abs() < 1e-4);
    }

    // --- page_annotations (integration, requires Document) ---

    #[test]

    fn page_annotations_empty_annots_array() {
        use crate::fpdfapi::parser::document::Document;
        use std::io::Cursor;

        let pdf = page_with_empty_annots();
        let mut doc = Document::from_reader(Cursor::new(pdf)).unwrap();
        let annots = doc.page_annotations(0).unwrap();
        assert!(annots.is_empty());
    }

    #[test]

    fn page_annotations_text_annot() {
        use crate::fpdfapi::parser::document::Document;
        use std::io::Cursor;

        let pdf = page_with_text_annot();
        let mut doc = Document::from_reader(Cursor::new(pdf)).unwrap();
        let annots = doc.page_annotations(0).unwrap();
        assert_eq!(annots.len(), 1);
        assert_eq!(annots[0].subtype, AnnotSubtype::Text);
        assert_eq!(annots[0].contents.as_deref(), Some("Hello"));
    }

    fn page_with_empty_annots() -> Vec<u8> {
        let mut pdf = Vec::new();
        pdf.extend_from_slice(b"%PDF-1.4\n");
        let obj1_off = pdf.len();
        pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
        let obj2_off = pdf.len();
        pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
        let obj3_off = pdf.len();
        pdf.extend_from_slice(
            b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Annots [] >>\nendobj\n",
        );
        let xref_off = pdf.len();
        pdf.extend_from_slice(b"xref\n0 4\n0000000000 65535 f \n");
        pdf.extend_from_slice(format!("{obj1_off:010} 00000 n \n").as_bytes());
        pdf.extend_from_slice(format!("{obj2_off:010} 00000 n \n").as_bytes());
        pdf.extend_from_slice(format!("{obj3_off:010} 00000 n \n").as_bytes());
        pdf.extend_from_slice(b"trailer\n<< /Size 4 /Root 1 0 R >>\n");
        pdf.extend_from_slice(format!("startxref\n{xref_off}\n%%EOF\n").as_bytes());
        pdf
    }

    fn page_with_text_annot() -> Vec<u8> {
        let mut pdf = Vec::new();
        pdf.extend_from_slice(b"%PDF-1.4\n");
        let obj1_off = pdf.len();
        pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
        let obj2_off = pdf.len();
        pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
        let obj3_off = pdf.len();
        pdf.extend_from_slice(
            b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Annots [4 0 R] >>\nendobj\n",
        );
        let obj4_off = pdf.len();
        pdf.extend_from_slice(
            b"4 0 obj\n<< /Type /Annot /Subtype /Text /Rect [10 20 110 70] /Contents (Hello) /F 4 >>\nendobj\n",
        );
        let xref_off = pdf.len();
        pdf.extend_from_slice(b"xref\n0 5\n0000000000 65535 f \n");
        pdf.extend_from_slice(format!("{obj1_off:010} 00000 n \n").as_bytes());
        pdf.extend_from_slice(format!("{obj2_off:010} 00000 n \n").as_bytes());
        pdf.extend_from_slice(format!("{obj3_off:010} 00000 n \n").as_bytes());
        pdf.extend_from_slice(format!("{obj4_off:010} 00000 n \n").as_bytes());
        pdf.extend_from_slice(b"trailer\n<< /Size 5 /Root 1 0 R >>\n");
        pdf.extend_from_slice(format!("startxref\n{xref_off}\n%%EOF\n").as_bytes());
        pdf
    }

    /// PDF where /Annots is an indirect reference to the array (obj 5).
    fn page_with_indirect_annots() -> Vec<u8> {
        let mut pdf = Vec::new();
        pdf.extend_from_slice(b"%PDF-1.4\n");
        let o1 = pdf.len();
        pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
        let o2 = pdf.len();
        pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
        let o3 = pdf.len();
        // /Annots is an indirect reference to obj 5
        pdf.extend_from_slice(
            b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Annots 5 0 R >>\nendobj\n",
        );
        let o4 = pdf.len();
        pdf.extend_from_slice(
            b"4 0 obj\n<< /Type /Annot /Subtype /Highlight /Rect [0 0 50 50] >>\nendobj\n",
        );
        let o5 = pdf.len();
        pdf.extend_from_slice(b"5 0 obj\n[4 0 R]\nendobj\n");
        let xref = pdf.len();
        pdf.extend_from_slice(b"xref\n0 6\n0000000000 65535 f \n");
        for o in [o1, o2, o3, o4, o5] {
            pdf.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
        }
        pdf.extend_from_slice(b"trailer\n<< /Size 6 /Root 1 0 R >>\n");
        pdf.extend_from_slice(format!("startxref\n{xref}\n%%EOF\n").as_bytes());
        pdf
    }

    #[test]
    fn page_annotations_indirect_annots_ref() {
        use crate::fpdfapi::parser::document::Document;
        use std::io::Cursor;

        let pdf = page_with_indirect_annots();
        let mut doc = Document::from_reader(Cursor::new(pdf)).unwrap();
        let annots = doc.page_annotations(0).unwrap();
        assert_eq!(annots.len(), 1);
        assert_eq!(annots[0].subtype, AnnotSubtype::Highlight);
    }

    #[test]
    fn page_annotations_indirect_action_resolved() {
        use crate::fpdfapi::parser::document::Document;
        use std::io::Cursor;

        // /A is an indirect reference to obj 5 (a URI action)
        let mut pdf = Vec::new();
        pdf.extend_from_slice(b"%PDF-1.4\n");
        let o1 = pdf.len();
        pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
        let o2 = pdf.len();
        pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
        let o3 = pdf.len();
        pdf.extend_from_slice(b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Annots [4 0 R] >>\nendobj\n");
        let o4 = pdf.len();
        pdf.extend_from_slice(
            b"4 0 obj\n<< /Type /Annot /Subtype /Link /Rect [0 0 100 100] /A 5 0 R >>\nendobj\n",
        );
        let o5 = pdf.len();
        pdf.extend_from_slice(b"5 0 obj\n<< /S /URI /URI (https://example.com) >>\nendobj\n");
        let xref = pdf.len();
        pdf.extend_from_slice(b"xref\n0 6\n0000000000 65535 f \n");
        for o in [o1, o2, o3, o4, o5] {
            pdf.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
        }
        pdf.extend_from_slice(b"trailer\n<< /Size 6 /Root 1 0 R >>\n");
        pdf.extend_from_slice(format!("startxref\n{xref}\n%%EOF\n").as_bytes());

        let mut doc = Document::from_reader(Cursor::new(pdf)).unwrap();
        let annots = doc.page_annotations(0).unwrap();
        assert_eq!(annots.len(), 1);
        let action = annots[0]
            .action
            .as_ref()
            .expect("action should be resolved");
        use crate::fpdfdoc::action::ActionType;
        assert_eq!(action.action_type(), ActionType::Uri);
        assert_eq!(action.uri(), Some("https://example.com".to_string()));
    }

    #[test]
    fn page_annotations_invalid_annots_type() {
        use crate::fpdfapi::parser::document::Document;
        use std::io::Cursor;

        // /Annots is an integer (invalid)
        let mut pdf = Vec::new();
        pdf.extend_from_slice(b"%PDF-1.4\n");
        let o1 = pdf.len();
        pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
        let o2 = pdf.len();
        pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
        let o3 = pdf.len();
        pdf.extend_from_slice(b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Annots 42 >>\nendobj\n");
        let xref = pdf.len();
        pdf.extend_from_slice(b"xref\n0 4\n0000000000 65535 f \n");
        for o in [o1, o2, o3] {
            pdf.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
        }
        pdf.extend_from_slice(b"trailer\n<< /Size 4 /Root 1 0 R >>\n");
        pdf.extend_from_slice(format!("startxref\n{xref}\n%%EOF\n").as_bytes());

        let mut doc = Document::from_reader(Cursor::new(pdf)).unwrap();
        let result = doc.page_annotations(0);
        assert!(result.is_err(), "/Annots integer should cause InvalidPdf");
    }
}
