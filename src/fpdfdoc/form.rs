use std::io::{Read, Seek};

use crate::error::{Error, Result};
use crate::fpdfapi::parser::document::Document;
use crate::fpdfapi::parser::object::PdfObject;

/// The type of an interactive form field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormFieldType {
    Unknown,
    PushButton,
    CheckBox,
    RadioButton,
    Text,
    RichText,
    File,
    ListBox,
    ComboBox,
    Signature,
}

/// A single option entry in a list box or combo box field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormOption {
    /// The value exported when this option is selected (first element of the sub-array).
    pub value: String,
    /// The display label shown to the user (second element, or same as `value` for plain strings).
    pub label: String,
}

/// A single interactive form field extracted from `/AcroForm`.
#[derive(Debug, Clone)]
pub struct FormField {
    /// Full dot-separated field name (e.g., `"Address.Street"`).
    pub full_name: String,
    pub field_type: FormFieldType,
    /// Current value (`/V`).
    pub value: Option<String>,
    /// Default value (`/DV`).
    pub default_value: Option<String>,
    /// Raw field flags (`/Ff`).
    pub flags: u32,
    /// Choice options (`/Opt`), non-empty for list boxes and combo boxes.
    pub options: Vec<FormOption>,
    /// Selected indices (`/I`) for list boxes.
    pub selected_indices: Vec<i32>,
    /// Maximum character length (`/MaxLen`) for text fields.
    pub max_len: Option<i32>,
    /// Alternate (tooltip) name (`/TU`).
    pub alternate_name: Option<String>,
    /// True when bit 1 of `/Ff` is set.
    pub read_only: bool,
    /// True when bit 2 of `/Ff` is set.
    pub required: bool,
}

/// The interactive form extracted from a PDF document.
#[derive(Debug, Clone)]
pub struct InteractiveForm {
    pub fields: Vec<FormField>,
}

/// Extension trait providing form access on `Document`.
pub trait FormExt {
    /// Extract the interactive form from the document's `/AcroForm`.
    ///
    /// Returns `Ok(None)` when the document has no `/AcroForm`.
    fn form(&mut self) -> Result<Option<InteractiveForm>>;
}

impl<R: Read + Seek> FormExt for Document<R> {
    fn form(&mut self) -> Result<Option<InteractiveForm>> {
        let catalog = self.catalog()?.clone();

        let acroform_obj = match catalog.get(b"AcroForm").cloned() {
            Some(obj) => obj,
            None => return Ok(None),
        };

        let acroform = match acroform_obj {
            PdfObject::Dictionary(d) => d,
            PdfObject::Reference(id) => self
                .object(id.num)?
                .as_dict()
                .ok_or_else(|| Error::InvalidPdf("/AcroForm is not a dictionary".into()))?
                .clone(),
            _ => return Err(Error::InvalidPdf("/AcroForm is not a dictionary".into())),
        };

        let field_nums: Vec<u32> = acroform
            .get_array(b"Fields")
            .map(|arr| {
                arr.iter()
                    .filter_map(|o| o.as_reference().map(|id| id.num))
                    .collect()
            })
            .unwrap_or_default();

        let mut fields = Vec::new();
        collect_fields(self, field_nums, None, None, 0, &mut fields)?;

        Ok(Some(InteractiveForm { fields }))
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn collect_fields<R: Read + Seek>(
    doc: &mut Document<R>,
    field_nums: Vec<u32>,
    parent_name: Option<String>,
    inh_ft: Option<Vec<u8>>,
    inh_ff: u32,
    result: &mut Vec<FormField>,
) -> Result<()> {
    for num in field_nums {
        let dict = doc
            .object(num)?
            .as_dict()
            .ok_or_else(|| {
                Error::InvalidPdf(format!("form field object {num} is not a dictionary"))
            })?
            .clone();

        let partial = dict
            .get_string(b"T")
            .map(|s| decode_pdf_text_string(s.as_bytes()));

        let ft_bytes: Option<Vec<u8>> = dict
            .get_name(b"FT")
            .map(|n| n.as_bytes().to_vec())
            .or_else(|| inh_ft.clone());

        let ff = dict.get_i32(b"Ff").map(|v| v as u32).unwrap_or(inh_ff);

        let full_name = match (&parent_name, &partial) {
            (Some(p), Some(t)) => Some(format!("{p}.{t}")),
            (None, Some(t)) => Some(t.clone()),
            (Some(p), None) => Some(p.clone()),
            (None, None) => None,
        };

        let kid_nums: Vec<u32> = dict
            .get_array(b"Kids")
            .map(|arr| {
                arr.iter()
                    .filter_map(|o| o.as_reference().map(|id| id.num))
                    .collect()
            })
            .unwrap_or_default();

        // Separate kids into sub-fields (have /T) vs. widget annotations (no /T)
        let mut subfield_kids = Vec::new();
        for &kid_num in &kid_nums {
            let has_t = doc
                .object(kid_num)
                .ok()
                .and_then(|o| o.as_dict())
                .map(|d| d.get(b"T").is_some())
                .unwrap_or(false);
            if has_t {
                subfield_kids.push(kid_num);
            }
        }

        if !subfield_kids.is_empty() {
            // Intermediate node — recurse into sub-fields
            collect_fields(doc, subfield_kids, full_name, ft_bytes, ff, result)?;
        } else if let Some(name) = full_name {
            // Terminal field — emit
            let field_type = determine_field_type(ft_bytes.as_deref(), ff);
            let value = dict.get(b"V").and_then(pdf_obj_to_string);
            let default_value = dict.get(b"DV").and_then(pdf_obj_to_string);
            let options = parse_opt_array(dict.get_array(b"Opt").unwrap_or(&[]));
            let selected_indices = dict
                .get_array(b"I")
                .map(|arr| arr.iter().filter_map(|o| o.as_i32()).collect())
                .unwrap_or_default();
            let max_len = dict.get_i32(b"MaxLen");
            let alternate_name = dict
                .get_string(b"TU")
                .map(|s| decode_pdf_text_string(s.as_bytes()));

            result.push(FormField {
                full_name: name,
                field_type,
                value,
                default_value,
                flags: ff,
                options,
                selected_indices,
                max_len,
                alternate_name,
                read_only: ff & 1 != 0,
                required: ff & 2 != 0,
            });
        }
    }
    Ok(())
}

fn determine_field_type(ft: Option<&[u8]>, ff: u32) -> FormFieldType {
    match ft {
        Some(b"Tx") => {
            if ff & (1 << 25) != 0 {
                FormFieldType::RichText
            } else if ff & (1 << 20) != 0 {
                FormFieldType::File
            } else {
                FormFieldType::Text
            }
        }
        Some(b"Ch") => {
            if ff & (1 << 16) != 0 {
                FormFieldType::ComboBox
            } else {
                FormFieldType::ListBox
            }
        }
        Some(b"Btn") => {
            if ff & (1 << 15) != 0 {
                FormFieldType::PushButton
            } else if ff & (1 << 14) != 0 {
                FormFieldType::RadioButton
            } else {
                FormFieldType::CheckBox
            }
        }
        Some(b"Sig") => FormFieldType::Signature,
        _ => FormFieldType::Unknown,
    }
}

fn parse_opt_array(arr: &[PdfObject]) -> Vec<FormOption> {
    arr.iter()
        .filter_map(|item| match item {
            PdfObject::String(s) => {
                let text = decode_pdf_text_string(s.as_bytes());
                Some(FormOption {
                    value: text.clone(),
                    label: text,
                })
            }
            PdfObject::Array(sub) if sub.len() >= 2 => {
                let value = sub[0]
                    .as_str()
                    .map(|s| decode_pdf_text_string(s.as_bytes()))
                    .unwrap_or_default();
                let label = sub[1]
                    .as_str()
                    .map(|s| decode_pdf_text_string(s.as_bytes()))
                    .unwrap_or_default();
                Some(FormOption { value, label })
            }
            _ => None,
        })
        .collect()
}

fn pdf_obj_to_string(obj: &PdfObject) -> Option<String> {
    match obj {
        PdfObject::String(s) => Some(decode_pdf_text_string(s.as_bytes())),
        PdfObject::Name(n) => Some(String::from_utf8_lossy(n.as_bytes()).into_owned()),
        PdfObject::Integer(v) => Some(v.to_string()),
        _ => None,
    }
}

fn decode_pdf_text_string(bytes: &[u8]) -> String {
    let raw: String = if bytes.starts_with(b"\xfe\xff") {
        let pairs = bytes[2..].chunks_exact(2);
        pairs
            .filter_map(|p| {
                let cp = u16::from_be_bytes([p[0], p[1]]);
                char::from_u32(cp as u32)
            })
            .collect()
    } else {
        bytes.iter().map(|&b| pdf_doc_encoding_char(b)).collect()
    };

    raw.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect()
}

fn pdf_doc_encoding_char(b: u8) -> char {
    match b {
        0x80 => '\u{2022}',
        0x81 => '\u{2020}',
        0x82 => '\u{2021}',
        0x83 => '\u{2026}',
        0x84 => '\u{2014}',
        0x85 => '\u{2013}',
        0x86 => '\u{0192}',
        0x87 => '\u{2044}',
        0x88 => '\u{2039}',
        0x89 => '\u{203A}',
        0x8A => '\u{2212}',
        0x8B => '\u{2030}',
        0x8C => '\u{201E}',
        0x8D => '\u{201C}',
        0x8E => '\u{201D}',
        0x8F => '\u{2018}',
        0x90 => '\u{2019}',
        0x91 => '\u{201A}',
        0x92 => '\u{2122}',
        0x93 => '\u{FB01}',
        0x94 => '\u{FB02}',
        0x95 => '\u{0141}',
        0x96 => '\u{0152}',
        0x97 => '\u{0160}',
        0x98 => '\u{0178}',
        0x99 => '\u{017D}',
        0x9A => '\u{0131}',
        0x9B => '\u{0142}',
        0x9C => '\u{0153}',
        0x9D => '\u{0161}',
        0x9E => '\u{017E}',
        _ => b as char,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    // --- form() tests ---

    #[test]
    fn form_returns_none_without_acroform() {
        let pdf = minimal_pdf();
        let mut doc = Document::from_reader(Cursor::new(pdf)).unwrap();
        assert!(doc.form().unwrap().is_none());
    }

    #[test]
    fn form_text_field() {
        let pdf = pdf_with_text_field();
        let mut doc = Document::from_reader(Cursor::new(pdf)).unwrap();
        let form = doc.form().unwrap().unwrap();
        assert_eq!(form.fields.len(), 1);
        let f = &form.fields[0];
        assert_eq!(f.full_name, "FirstName");
        assert_eq!(f.field_type, FormFieldType::Text);
        assert_eq!(f.value, Some("Alice".to_string()));
        assert_eq!(f.max_len, Some(50));
    }

    #[test]
    fn form_button_types() {
        let pdf = pdf_with_button_fields();
        let mut doc = Document::from_reader(Cursor::new(pdf)).unwrap();
        let form = doc.form().unwrap().unwrap();
        assert_eq!(form.fields.len(), 3);

        let push = form
            .fields
            .iter()
            .find(|f| f.full_name == "Submit")
            .unwrap();
        assert_eq!(push.field_type, FormFieldType::PushButton);

        let check = form
            .fields
            .iter()
            .find(|f| f.full_name == "Accept")
            .unwrap();
        assert_eq!(check.field_type, FormFieldType::CheckBox);

        let radio = form.fields.iter().find(|f| f.full_name == "Color").unwrap();
        assert_eq!(radio.field_type, FormFieldType::RadioButton);
    }

    #[test]
    fn form_field_full_name_from_hierarchy() {
        let pdf = pdf_with_nested_fields();
        let mut doc = Document::from_reader(Cursor::new(pdf)).unwrap();
        let form = doc.form().unwrap().unwrap();
        assert_eq!(form.fields.len(), 1);
        assert_eq!(form.fields[0].full_name, "Address.Street");
        assert_eq!(form.fields[0].field_type, FormFieldType::Text);
    }

    #[test]
    fn form_listbox_with_options() {
        let pdf = pdf_with_listbox();
        let mut doc = Document::from_reader(Cursor::new(pdf)).unwrap();
        let form = doc.form().unwrap().unwrap();
        assert_eq!(form.fields.len(), 1);
        let f = &form.fields[0];
        assert_eq!(f.field_type, FormFieldType::ListBox);
        assert_eq!(f.options.len(), 2);
        assert_eq!(f.options[0].value, "red");
        assert_eq!(f.options[0].label, "Red");
        assert_eq!(f.options[1].value, "blue");
        assert_eq!(f.options[1].label, "Blue");
    }

    #[test]
    fn form_read_only_required_flags() {
        let pdf = pdf_with_flagged_field();
        let mut doc = Document::from_reader(Cursor::new(pdf)).unwrap();
        let form = doc.form().unwrap().unwrap();
        assert_eq!(form.fields.len(), 1);
        let f = &form.fields[0];
        assert!(f.read_only);
        assert!(f.required);
        assert_eq!(f.flags, 3); // bits 1 and 2
    }

    // --- Helper PDFs ---

    fn minimal_pdf() -> Vec<u8> {
        let mut pdf = Vec::new();
        pdf.extend_from_slice(b"%PDF-1.4\n");
        let o1 = pdf.len();
        pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
        let o2 = pdf.len();
        pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [] /Count 0 >>\nendobj\n");
        let xref = pdf.len();
        pdf.extend_from_slice(b"xref\n0 3\n0000000000 65535 f \n");
        pdf.extend_from_slice(format!("{o1:010} 00000 n \n").as_bytes());
        pdf.extend_from_slice(format!("{o2:010} 00000 n \n").as_bytes());
        pdf.extend_from_slice(b"trailer\n<< /Size 3 /Root 1 0 R >>\n");
        pdf.extend_from_slice(format!("startxref\n{xref}\n%%EOF\n").as_bytes());
        pdf
    }

    fn pdf_with_text_field() -> Vec<u8> {
        // obj 3: AcroForm, obj 4: text field
        let mut pdf = Vec::new();
        pdf.extend_from_slice(b"%PDF-1.4\n");
        let o1 = pdf.len();
        pdf.extend_from_slice(
            b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R /AcroForm 3 0 R >>\nendobj\n",
        );
        let o2 = pdf.len();
        pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [] /Count 0 >>\nendobj\n");
        let o3 = pdf.len();
        pdf.extend_from_slice(b"3 0 obj\n<< /Fields [4 0 R] >>\nendobj\n");
        let o4 = pdf.len();
        pdf.extend_from_slice(
            b"4 0 obj\n<< /FT /Tx /T (FirstName) /V (Alice) /MaxLen 50 >>\nendobj\n",
        );
        let xref = pdf.len();
        pdf.extend_from_slice(b"xref\n0 5\n0000000000 65535 f \n");
        pdf.extend_from_slice(format!("{o1:010} 00000 n \n").as_bytes());
        pdf.extend_from_slice(format!("{o2:010} 00000 n \n").as_bytes());
        pdf.extend_from_slice(format!("{o3:010} 00000 n \n").as_bytes());
        pdf.extend_from_slice(format!("{o4:010} 00000 n \n").as_bytes());
        pdf.extend_from_slice(b"trailer\n<< /Size 5 /Root 1 0 R >>\n");
        pdf.extend_from_slice(format!("startxref\n{xref}\n%%EOF\n").as_bytes());
        pdf
    }

    fn pdf_with_button_fields() -> Vec<u8> {
        // Three button fields:
        //   obj 4: PushButton  (/Btn, Ff=32768 = bit 16)
        //   obj 5: CheckBox    (/Btn, Ff=0)
        //   obj 6: RadioButton (/Btn, Ff=16384 = bit 15)
        let mut pdf = Vec::new();
        pdf.extend_from_slice(b"%PDF-1.4\n");
        let o1 = pdf.len();
        pdf.extend_from_slice(
            b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R /AcroForm 3 0 R >>\nendobj\n",
        );
        let o2 = pdf.len();
        pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [] /Count 0 >>\nendobj\n");
        let o3 = pdf.len();
        pdf.extend_from_slice(b"3 0 obj\n<< /Fields [4 0 R 5 0 R 6 0 R] >>\nendobj\n");
        let o4 = pdf.len();
        pdf.extend_from_slice(b"4 0 obj\n<< /FT /Btn /T (Submit) /Ff 32768 >>\nendobj\n");
        let o5 = pdf.len();
        pdf.extend_from_slice(b"5 0 obj\n<< /FT /Btn /T (Accept) /Ff 0 >>\nendobj\n");
        let o6 = pdf.len();
        pdf.extend_from_slice(b"6 0 obj\n<< /FT /Btn /T (Color) /Ff 16384 >>\nendobj\n");
        let xref = pdf.len();
        pdf.extend_from_slice(b"xref\n0 7\n0000000000 65535 f \n");
        pdf.extend_from_slice(format!("{o1:010} 00000 n \n").as_bytes());
        pdf.extend_from_slice(format!("{o2:010} 00000 n \n").as_bytes());
        pdf.extend_from_slice(format!("{o3:010} 00000 n \n").as_bytes());
        pdf.extend_from_slice(format!("{o4:010} 00000 n \n").as_bytes());
        pdf.extend_from_slice(format!("{o5:010} 00000 n \n").as_bytes());
        pdf.extend_from_slice(format!("{o6:010} 00000 n \n").as_bytes());
        pdf.extend_from_slice(b"trailer\n<< /Size 7 /Root 1 0 R >>\n");
        pdf.extend_from_slice(format!("startxref\n{xref}\n%%EOF\n").as_bytes());
        pdf
    }

    fn pdf_with_nested_fields() -> Vec<u8> {
        // obj 3: AcroForm /Fields [4 0 R]
        // obj 4: /T (Address), /Kids [5 0 R]  -- intermediate node
        // obj 5: /T (Street), /FT /Tx          -- leaf
        let mut pdf = Vec::new();
        pdf.extend_from_slice(b"%PDF-1.4\n");
        let o1 = pdf.len();
        pdf.extend_from_slice(
            b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R /AcroForm 3 0 R >>\nendobj\n",
        );
        let o2 = pdf.len();
        pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [] /Count 0 >>\nendobj\n");
        let o3 = pdf.len();
        pdf.extend_from_slice(b"3 0 obj\n<< /Fields [4 0 R] >>\nendobj\n");
        let o4 = pdf.len();
        pdf.extend_from_slice(b"4 0 obj\n<< /T (Address) /Kids [5 0 R] >>\nendobj\n");
        let o5 = pdf.len();
        pdf.extend_from_slice(b"5 0 obj\n<< /T (Street) /FT /Tx >>\nendobj\n");
        let xref = pdf.len();
        pdf.extend_from_slice(b"xref\n0 6\n0000000000 65535 f \n");
        pdf.extend_from_slice(format!("{o1:010} 00000 n \n").as_bytes());
        pdf.extend_from_slice(format!("{o2:010} 00000 n \n").as_bytes());
        pdf.extend_from_slice(format!("{o3:010} 00000 n \n").as_bytes());
        pdf.extend_from_slice(format!("{o4:010} 00000 n \n").as_bytes());
        pdf.extend_from_slice(format!("{o5:010} 00000 n \n").as_bytes());
        pdf.extend_from_slice(b"trailer\n<< /Size 6 /Root 1 0 R >>\n");
        pdf.extend_from_slice(format!("startxref\n{xref}\n%%EOF\n").as_bytes());
        pdf
    }

    fn pdf_with_listbox() -> Vec<u8> {
        // obj 4: list box with /Opt [[red Red] [blue Blue]]
        let mut pdf = Vec::new();
        pdf.extend_from_slice(b"%PDF-1.4\n");
        let o1 = pdf.len();
        pdf.extend_from_slice(
            b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R /AcroForm 3 0 R >>\nendobj\n",
        );
        let o2 = pdf.len();
        pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [] /Count 0 >>\nendobj\n");
        let o3 = pdf.len();
        pdf.extend_from_slice(b"3 0 obj\n<< /Fields [4 0 R] >>\nendobj\n");
        let o4 = pdf.len();
        pdf.extend_from_slice(
            b"4 0 obj\n<< /FT /Ch /T (Colour) /Opt [[(red) (Red)] [(blue) (Blue)]] >>\nendobj\n",
        );
        let xref = pdf.len();
        pdf.extend_from_slice(b"xref\n0 5\n0000000000 65535 f \n");
        pdf.extend_from_slice(format!("{o1:010} 00000 n \n").as_bytes());
        pdf.extend_from_slice(format!("{o2:010} 00000 n \n").as_bytes());
        pdf.extend_from_slice(format!("{o3:010} 00000 n \n").as_bytes());
        pdf.extend_from_slice(format!("{o4:010} 00000 n \n").as_bytes());
        pdf.extend_from_slice(b"trailer\n<< /Size 5 /Root 1 0 R >>\n");
        pdf.extend_from_slice(format!("startxref\n{xref}\n%%EOF\n").as_bytes());
        pdf
    }

    fn pdf_with_flagged_field() -> Vec<u8> {
        // obj 4: text field with /Ff 3 (ReadOnly=bit1 + Required=bit2)
        let mut pdf = Vec::new();
        pdf.extend_from_slice(b"%PDF-1.4\n");
        let o1 = pdf.len();
        pdf.extend_from_slice(
            b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R /AcroForm 3 0 R >>\nendobj\n",
        );
        let o2 = pdf.len();
        pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [] /Count 0 >>\nendobj\n");
        let o3 = pdf.len();
        pdf.extend_from_slice(b"3 0 obj\n<< /Fields [4 0 R] >>\nendobj\n");
        let o4 = pdf.len();
        pdf.extend_from_slice(b"4 0 obj\n<< /FT /Tx /T (Email) /Ff 3 >>\nendobj\n");
        let xref = pdf.len();
        pdf.extend_from_slice(b"xref\n0 5\n0000000000 65535 f \n");
        pdf.extend_from_slice(format!("{o1:010} 00000 n \n").as_bytes());
        pdf.extend_from_slice(format!("{o2:010} 00000 n \n").as_bytes());
        pdf.extend_from_slice(format!("{o3:010} 00000 n \n").as_bytes());
        pdf.extend_from_slice(format!("{o4:010} 00000 n \n").as_bytes());
        pdf.extend_from_slice(b"trailer\n<< /Size 5 /Root 1 0 R >>\n");
        pdf.extend_from_slice(format!("startxref\n{xref}\n%%EOF\n").as_bytes());
        pdf
    }
}
