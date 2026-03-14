//! Integration tests for the `fpdfdoc` module.
//!
//! Each test builds a minimal but valid PDF in-memory and exercises the
//! full `Document::from_reader` → fpdfdoc-API pipeline, covering cross-module
//! interactions between bookmarks, actions, annotations, links, and forms.

use pdfium_rs::Document;
use pdfium_rs::fpdfdoc::{
    ActionType, AnnotSubtype, AnnotationsExt, BookmarksExt, FormExt, FormFieldType, LinksExt,
};
use std::io::Cursor;

// ── PDF builders ──────────────────────────────────────────────────────────────

/// PDF with a two-level bookmark tree and a URI-action bookmark.
///
/// obj 3: Outlines /First 4 /Last 6
/// obj 4: "Chapter 1"  /Count 1  /First 5 /Next 6
/// obj 5: "Section 1.1" (child of 4)
/// obj 6: "External"  /A << /S /URI /URI (https://example.com) >>
fn pdf_with_bookmark_tree() -> Vec<u8> {
    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.4\n");
    let o1 = pdf.len();
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R /Outlines 3 0 R >>\nendobj\n");
    let o2 = pdf.len();
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [] /Count 0 >>\nendobj\n");
    let o3 = pdf.len();
    pdf.extend_from_slice(b"3 0 obj\n<< /Type /Outlines /First 4 0 R /Last 6 0 R >>\nendobj\n");
    let o4 = pdf.len();
    pdf.extend_from_slice(
        b"4 0 obj\n<< /Title (Chapter 1) /Count 1 /First 5 0 R /Last 5 0 R /Next 6 0 R >>\nendobj\n",
    );
    let o5 = pdf.len();
    pdf.extend_from_slice(b"5 0 obj\n<< /Title (Section 1.1) /Count 0 >>\nendobj\n");
    let o6 = pdf.len();
    pdf.extend_from_slice(
        b"6 0 obj\n<< /Title (External) /Count 0 /A << /S /URI /URI (https://example.com) >> >>\nendobj\n",
    );
    let xref = pdf.len();
    pdf.extend_from_slice(b"xref\n0 7\n0000000000 65535 f \n");
    for o in [o1, o2, o3, o4, o5, o6] {
        pdf.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(b"trailer\n<< /Size 7 /Root 1 0 R >>\n");
    pdf.extend_from_slice(format!("startxref\n{xref}\n%%EOF\n").as_bytes());
    pdf
}

/// PDF with two annotations on page 0.
///
/// obj 4: /Subtype /Text  (sticky note)
/// obj 5: /Subtype /Link  with URI action
fn pdf_with_annotations() -> Vec<u8> {
    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.4\n");
    let o1 = pdf.len();
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    let o2 = pdf.len();
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    let o3 = pdf.len();
    pdf.extend_from_slice(
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Annots [4 0 R 5 0 R] >>\nendobj\n",
    );
    let o4 = pdf.len();
    pdf.extend_from_slice(
        b"4 0 obj\n<< /Type /Annot /Subtype /Text /Rect [100 100 200 200] /Contents (A note) >>\nendobj\n",
    );
    let o5 = pdf.len();
    pdf.extend_from_slice(
        b"5 0 obj\n<< /Type /Annot /Subtype /Link /Rect [300 300 400 400] /A << /S /URI /URI (https://rust-lang.org) >> >>\nendobj\n",
    );
    let xref = pdf.len();
    pdf.extend_from_slice(b"xref\n0 6\n0000000000 65535 f \n");
    for o in [o1, o2, o3, o4, o5] {
        pdf.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(b"trailer\n<< /Size 6 /Root 1 0 R >>\n");
    pdf.extend_from_slice(format!("startxref\n{xref}\n%%EOF\n").as_bytes());
    pdf
}

/// PDF with two form fields: a text field and a checkbox.
///
/// obj 3: AcroForm /Fields [4 5]
/// obj 4: /Tx  "Name"   /V (Alice)
/// obj 5: /Btn "Accept" /Ff 0  (CheckBox)
fn pdf_with_form() -> Vec<u8> {
    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.4\n");
    let o1 = pdf.len();
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R /AcroForm 3 0 R >>\nendobj\n");
    let o2 = pdf.len();
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [] /Count 0 >>\nendobj\n");
    let o3 = pdf.len();
    pdf.extend_from_slice(b"3 0 obj\n<< /Fields [4 0 R 5 0 R] >>\nendobj\n");
    let o4 = pdf.len();
    pdf.extend_from_slice(b"4 0 obj\n<< /FT /Tx /T (Name) /V (Alice) >>\nendobj\n");
    let o5 = pdf.len();
    pdf.extend_from_slice(b"5 0 obj\n<< /FT /Btn /T (Accept) /Ff 0 >>\nendobj\n");
    let xref = pdf.len();
    pdf.extend_from_slice(b"xref\n0 6\n0000000000 65535 f \n");
    for o in [o1, o2, o3, o4, o5] {
        pdf.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(b"trailer\n<< /Size 6 /Root 1 0 R >>\n");
    pdf.extend_from_slice(format!("startxref\n{xref}\n%%EOF\n").as_bytes());
    pdf
}

// ── Bookmark tests ────────────────────────────────────────────────────────────

#[test]
fn bookmarks_multi_level_tree() {
    let mut doc = Document::from_reader(Cursor::new(pdf_with_bookmark_tree())).unwrap();
    let bookmarks = doc.bookmarks().unwrap();
    assert_eq!(bookmarks.len(), 2, "two top-level bookmarks");
    assert_eq!(bookmarks[0].title, "Chapter 1");
    assert_eq!(bookmarks[0].children.len(), 1);
    assert_eq!(bookmarks[0].children[0].title, "Section 1.1");
    assert_eq!(bookmarks[1].title, "External");
    assert!(bookmarks[1].children.is_empty());
}

#[test]
fn bookmarks_uri_action() {
    let mut doc = Document::from_reader(Cursor::new(pdf_with_bookmark_tree())).unwrap();
    let bookmarks = doc.bookmarks().unwrap();
    assert_eq!(bookmarks.len(), 2, "expected two top-level bookmarks");
    let action = bookmarks[1].action.as_ref().unwrap();
    assert_eq!(action.action_type(), ActionType::Uri);
    assert_eq!(action.uri(), Some("https://example.com".to_string()));
}

// ── Annotation tests ──────────────────────────────────────────────────────────

#[test]
fn annotations_two_subtypes_on_page() {
    let mut doc = Document::from_reader(Cursor::new(pdf_with_annotations())).unwrap();
    let annots = doc.page_annotations(0).unwrap();
    assert_eq!(annots.len(), 2);
    assert_eq!(annots[0].subtype, AnnotSubtype::Text);
    assert_eq!(annots[1].subtype, AnnotSubtype::Link);
}

#[test]
fn annotations_contents_extracted() {
    let mut doc = Document::from_reader(Cursor::new(pdf_with_annotations())).unwrap();
    let annots = doc.page_annotations(0).unwrap();
    assert!(!annots.is_empty(), "expected at least one annotation");
    assert_eq!(annots[0].contents, Some("A note".to_string()));
}

// ── Link tests ────────────────────────────────────────────────────────────────

#[test]
fn links_uri_on_page() {
    let mut doc = Document::from_reader(Cursor::new(pdf_with_annotations())).unwrap();
    let links = doc.page_links(0).unwrap();
    assert_eq!(links.len(), 1, "one Link annotation on the page");
    let action = links[0].action.as_ref().unwrap();
    assert_eq!(action.action_type(), ActionType::Uri);
    assert_eq!(action.uri(), Some("https://rust-lang.org".to_string()));
}

// ── Form tests ────────────────────────────────────────────────────────────────

#[test]
fn form_mixed_field_types() {
    let mut doc = Document::from_reader(Cursor::new(pdf_with_form())).unwrap();
    let form = doc.form().unwrap().unwrap();
    assert_eq!(form.fields.len(), 2);

    let name = form.fields.iter().find(|f| f.full_name == "Name").unwrap();
    assert_eq!(name.field_type, FormFieldType::Text);
    assert_eq!(name.value, Some("Alice".to_string()));

    let accept = form
        .fields
        .iter()
        .find(|f| f.full_name == "Accept")
        .unwrap();
    assert_eq!(accept.field_type, FormFieldType::CheckBox);
}

#[test]
fn form_no_acroform_returns_none() {
    let pdf = pdf_with_bookmark_tree(); // no /AcroForm
    let mut doc = Document::from_reader(Cursor::new(pdf)).unwrap();
    assert!(doc.form().unwrap().is_none());
}
