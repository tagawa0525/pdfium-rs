//! Integration tests for text extraction and search, ported from
//! `fpdf_text_embeddertest.cpp` in the C++ PDFium reference.
//!
//! These tests use real PDF fixtures from `reference/pdfium/testing/resources/`
//! to exercise the full pipeline: PDF parse → page tree → content stream →
//! font decoding → text extraction → text search.

use std::fs::File;
use std::io::BufReader;

use pdfium_rs::fpdftext::TextPage;
use pdfium_rs::{Document, FindOptions, TextFind};

/// Open a reference PDF by filename.
fn open_ref_pdf(name: &str) -> Document<BufReader<File>> {
    let path = format!("reference/pdfium/testing/resources/{name}");
    Document::open(&path).unwrap_or_else(|e| panic!("failed to open {path}: {e}"))
}

// ─── Text extraction (ported from TEST_F(FPDFTextEmbedderTest, Text)) ────────

/// hello_world.pdf: "Hello, world!\r\nGoodbye, world!"
/// Two text objects — first in Times-Roman 12pt, second in Helvetica 16pt.
#[test]
#[ignore = "not yet implemented"]
fn hello_world_extract_text() {
    let mut doc = open_ref_pdf("hello_world.pdf");
    let page = doc.page(0).unwrap();
    let text = page.extract_text();

    // The C++ reference emits "Hello, world!\r\nGoodbye, world!" (30 chars).
    // Our implementation may use \n instead of \r\n; accept either.
    assert!(
        text.contains("Hello, world!"),
        "expected 'Hello, world!' in extracted text, got: {text:?}"
    );
    assert!(
        text.contains("Goodbye, world!"),
        "expected 'Goodbye, world!' in extracted text, got: {text:?}"
    );
}

/// Verify per-character info is available for hello_world.pdf.
#[test]
#[ignore = "not yet implemented"]
fn hello_world_char_count() {
    let mut doc = open_ref_pdf("hello_world.pdf");
    let page = doc.page(0).unwrap();
    let tp = TextPage::build(&page);

    // "Hello, world!" = 13 chars, separator(s), "Goodbye, world!" = 15 chars.
    // Exact count depends on whether synthetic \r\n or \n is inserted.
    assert!(
        tp.char_count() >= 28,
        "expected >= 28 chars, got {}",
        tp.char_count()
    );
}

/// Font sizes: first 13 chars at 12pt, then "Goodbye, world!" at 16pt.
#[test]
#[ignore = "not yet implemented"]
fn hello_world_font_sizes() {
    let mut doc = open_ref_pdf("hello_world.pdf");
    let page = doc.page(0).unwrap();
    let tp = TextPage::build(&page);

    // First char 'H' should be 12pt (Times-Roman)
    let first = tp.char_info(0).expect("char_info(0)");
    assert_eq!(first.unicode, 'H');
    assert!(
        (first.font_size - 12.0).abs() < 0.1,
        "expected 12pt, got {}",
        first.font_size
    );

    // Find the first 'G' of "Goodbye" — should be 16pt (Helvetica)
    let g_idx = (0..tp.char_count())
        .find(|&i| tp.char_info(i).map(|ci| ci.unicode) == Some('G'))
        .expect("'G' not found");
    let g_info = tp.char_info(g_idx).unwrap();
    assert!(
        (g_info.font_size - 16.0).abs() < 0.1,
        "expected 16pt for 'G', got {}",
        g_info.font_size
    );
}

// ─── Text search (ported from TEST_F(FPDFTextEmbedderTest, TextSearch)) ──────

/// Case-insensitive search (default) finds "world" twice.
#[test]
#[ignore = "not yet implemented"]
fn hello_world_search_case_insensitive() {
    let mut doc = open_ref_pdf("hello_world.pdf");
    let page = doc.page(0).unwrap();
    let tp = TextPage::build(&page);

    let opts = FindOptions::default(); // case_sensitive=false
    let matches = TextFind::find_all(&tp, "world", &opts);
    assert_eq!(
        matches.len(),
        2,
        "expected 2 occurrences of 'world', got {}",
        matches.len()
    );
}

/// Case-insensitive: "WORLD" matches "world".
#[test]
#[ignore = "not yet implemented"]
fn hello_world_search_caps_matches() {
    let mut doc = open_ref_pdf("hello_world.pdf");
    let page = doc.page(0).unwrap();
    let tp = TextPage::build(&page);

    let opts = FindOptions::default();
    let matches = TextFind::find_all(&tp, "WORLD", &opts);
    assert!(
        !matches.is_empty(),
        "case-insensitive search for 'WORLD' should find matches"
    );
}

/// Case-sensitive: "WORLD" does not match "world".
#[test]
#[ignore = "not yet implemented"]
fn hello_world_search_case_sensitive_no_match() {
    let mut doc = open_ref_pdf("hello_world.pdf");
    let page = doc.page(0).unwrap();
    let tp = TextPage::build(&page);

    let opts = FindOptions {
        case_sensitive: true,
        whole_word: false,
    };
    let matches = TextFind::find_all(&tp, "WORLD", &opts);
    assert!(
        matches.is_empty(),
        "case-sensitive search for 'WORLD' should find nothing"
    );
}

/// Substring "orld" matches by default (not whole-word).
#[test]
#[ignore = "not yet implemented"]
fn hello_world_search_substring_matches() {
    let mut doc = open_ref_pdf("hello_world.pdf");
    let page = doc.page(0).unwrap();
    let tp = TextPage::build(&page);

    let opts = FindOptions::default();
    let matches = TextFind::find_all(&tp, "orld", &opts);
    assert!(!matches.is_empty(), "'orld' should match as substring");
}

/// Whole-word: "orld" does NOT match (it's not a standalone word).
#[test]
#[ignore = "not yet implemented"]
fn hello_world_search_whole_word_rejects_substring() {
    let mut doc = open_ref_pdf("hello_world.pdf");
    let page = doc.page(0).unwrap();
    let tp = TextPage::build(&page);

    let opts = FindOptions {
        case_sensitive: false,
        whole_word: true,
    };
    let matches = TextFind::find_all(&tp, "orld", &opts);
    assert!(
        matches.is_empty(),
        "whole-word search for 'orld' should find nothing"
    );
}

/// "nope" is not in hello_world.pdf.
#[test]
#[ignore = "not yet implemented"]
fn hello_world_search_no_match() {
    let mut doc = open_ref_pdf("hello_world.pdf");
    let page = doc.page(0).unwrap();
    let tp = TextPage::build(&page);

    let opts = FindOptions::default();
    let matches = TextFind::find_all(&tp, "nope", &opts);
    assert!(matches.is_empty());
}

// ─── Bug642: simple "ABCD" text extraction ───────────────────────────────────

/// bug_642.pdf contains exactly "ABCD".
#[test]
#[ignore = "not yet implemented"]
fn bug_642_text_abcd() {
    let mut doc = open_ref_pdf("bug_642.pdf");
    let page = doc.page(0).unwrap();
    let tp = TextPage::build(&page);

    assert_eq!(tp.char_count(), 4);
    let text = tp.text();
    assert_eq!(text, "ABCD");
}

// ─── Page::find_text convenience method ──────────────────────────────────────

/// Page::find_text() wraps TextFind::find_all with default options.
#[test]
#[ignore = "not yet implemented"]
fn hello_world_page_find_text() {
    let mut doc = open_ref_pdf("hello_world.pdf");
    let page = doc.page(0).unwrap();
    let matches = page.find_text("world");
    assert_eq!(matches.len(), 2);
}
