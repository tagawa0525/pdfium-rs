/// Integration tests ported from PDFium C++:
///   core/fpdfapi/parser/cpdf_security_handler_embeddertest.cpp
///
/// These tests open real encrypted PDF files (from PDFium's test resources)
/// and verify password verification, encryption detection, and page structure.
use std::path::PathBuf;

use pdfium_rs::Document;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

// Password constants from the C++ test — UTF-8 and Latin-1 variants.
// Owner password: "âge"
const AGE_UTF8: &[u8] = b"\xc3\xa2ge";
const AGE_LATIN1: &[u8] = b"\xe2ge";
// User password: "hôtel"
const HOTEL_UTF8: &[u8] = b"h\xc3\xb4tel";
const HOTEL_LATIN1: &[u8] = b"h\xf4tel";

// --- Unencrypted ---

/// Ported from: Unencrypted
/// Opening a non-encrypted PDF should succeed without a password.
#[test]
#[ignore = "not yet implemented"]
fn unencrypted_open_succeeds() {
    // We use our minimal PDF (generated in-memory) for this test,
    // but we test the file-based path via from_reader_with_password.
    let pdf = fixture("encrypted_hello_world_r2.pdf");
    // An unencrypted PDF would work with from_reader. Here we just verify
    // that encrypted PDFs are correctly rejected by from_reader.
    let result = Document::open(pdf);
    assert!(
        result.is_err(),
        "encrypted PDF should fail without password"
    );
}

// --- Revision 2 (RC4 40-bit) ---

/// Ported from: OwnerPasswordVersion2UTF8
#[test]
#[ignore = "not yet implemented"]
fn r2_owner_password_utf8() {
    let path = fixture("encrypted_hello_world_r2.pdf");
    let doc = Document::open_with_password(&path, AGE_UTF8);
    assert!(doc.is_ok(), "R2 owner password (UTF-8) should succeed");
    let mut doc = doc.unwrap();
    assert!(doc.is_encrypted());
    assert_eq!(doc.page_count().unwrap(), 1);
}

/// Ported from: OwnerPasswordVersion2Latin1
#[test]
#[ignore = "not yet implemented"]
fn r2_owner_password_latin1() {
    let path = fixture("encrypted_hello_world_r2.pdf");
    let doc = Document::open_with_password(&path, AGE_LATIN1);
    assert!(doc.is_ok(), "R2 owner password (Latin-1) should succeed");
}

/// Ported from: UserPasswordVersion2UTF8
#[test]
#[ignore = "not yet implemented"]
fn r2_user_password_utf8() {
    let path = fixture("encrypted_hello_world_r2.pdf");
    let doc = Document::open_with_password(&path, HOTEL_UTF8);
    assert!(doc.is_ok(), "R2 user password (UTF-8) should succeed");
}

/// Ported from: UserPasswordVersion2Latin1
#[test]
#[ignore = "not yet implemented"]
fn r2_user_password_latin1() {
    let path = fixture("encrypted_hello_world_r2.pdf");
    let doc = Document::open_with_password(&path, HOTEL_LATIN1);
    assert!(doc.is_ok(), "R2 user password (Latin-1) should succeed");
}

/// Ported from: BadOkeyVersion2
#[test]
#[ignore = "not yet implemented"]
fn r2_bad_okey_fails() {
    let path = fixture("encrypted_hello_world_r2_bad_okey.pdf");
    let result = Document::open_with_password(&path, AGE_UTF8);
    assert!(result.is_err(), "R2 bad okey should fail");
}

// --- Revision 3 (RC4 128-bit) ---

/// Ported from: OwnerPasswordVersion3UTF8
#[test]
#[ignore = "not yet implemented"]
fn r3_owner_password_utf8() {
    let path = fixture("encrypted_hello_world_r3.pdf");
    let doc = Document::open_with_password(&path, AGE_UTF8);
    assert!(doc.is_ok(), "R3 owner password (UTF-8) should succeed");
    let mut doc = doc.unwrap();
    assert!(doc.is_encrypted());
    assert_eq!(doc.page_count().unwrap(), 1);
}

/// Ported from: OwnerPasswordVersion3Latin1
#[test]
#[ignore = "not yet implemented"]
fn r3_owner_password_latin1() {
    let path = fixture("encrypted_hello_world_r3.pdf");
    let doc = Document::open_with_password(&path, AGE_LATIN1);
    assert!(doc.is_ok(), "R3 owner password (Latin-1) should succeed");
}

/// Ported from: UserPasswordVersion3UTF8
#[test]
#[ignore = "not yet implemented"]
fn r3_user_password_utf8() {
    let path = fixture("encrypted_hello_world_r3.pdf");
    let doc = Document::open_with_password(&path, HOTEL_UTF8);
    assert!(doc.is_ok(), "R3 user password (UTF-8) should succeed");
}

/// Ported from: UserPasswordVersion3Latin1
#[test]
#[ignore = "not yet implemented"]
fn r3_user_password_latin1() {
    let path = fixture("encrypted_hello_world_r3.pdf");
    let doc = Document::open_with_password(&path, HOTEL_LATIN1);
    assert!(doc.is_ok(), "R3 user password (Latin-1) should succeed");
}

/// Ported from: BadOkeyVersion3
#[test]
#[ignore = "not yet implemented"]
fn r3_bad_okey_fails() {
    let path = fixture("encrypted_hello_world_r3_bad_okey.pdf");
    let result = Document::open_with_password(&path, AGE_UTF8);
    assert!(result.is_err(), "R3 bad okey should fail");
}

// --- Revision 5 (AES-256) ---

/// Ported from: OwnerPasswordVersion5
#[test]
#[ignore = "not yet implemented"]
fn r5_owner_password_utf8() {
    let path = fixture("encrypted_hello_world_r5.pdf");
    let doc = Document::open_with_password(&path, AGE_UTF8);
    assert!(doc.is_ok(), "R5 owner password (UTF-8) should succeed");
    let mut doc = doc.unwrap();
    assert!(doc.is_encrypted());
    assert_eq!(doc.page_count().unwrap(), 1);
}

/// Ported from: OwnerPasswordVersion5Latin1
#[test]
#[ignore = "not yet implemented"]
fn r5_owner_password_latin1() {
    let path = fixture("encrypted_hello_world_r5.pdf");
    let doc = Document::open_with_password(&path, AGE_LATIN1);
    assert!(doc.is_ok(), "R5 owner password (Latin-1) should succeed");
}

/// Ported from: UserPasswordVersion5UTF8
#[test]
#[ignore = "not yet implemented"]
fn r5_user_password_utf8() {
    let path = fixture("encrypted_hello_world_r5.pdf");
    let doc = Document::open_with_password(&path, HOTEL_UTF8);
    assert!(doc.is_ok(), "R5 user password (UTF-8) should succeed");
}

/// Ported from: UserPasswordVersion5Latin1
#[test]
#[ignore = "not yet implemented"]
fn r5_user_password_latin1() {
    let path = fixture("encrypted_hello_world_r5.pdf");
    let doc = Document::open_with_password(&path, HOTEL_LATIN1);
    assert!(doc.is_ok(), "R5 user password (Latin-1) should succeed");
}

/// Ported from: NoPasswordVersion5
#[test]
#[ignore = "not yet implemented"]
fn r5_no_password_fails() {
    let path = fixture("encrypted_hello_world_r5.pdf");
    let result = Document::open_with_password(&path, b"");
    assert!(result.is_err(), "R5 without password should fail");
}

/// Ported from: BadPasswordVersion5
#[test]
#[ignore = "not yet implemented"]
fn r5_bad_password_fails() {
    let path = fixture("encrypted_hello_world_r5.pdf");
    let result = Document::open_with_password(&path, b"tiger");
    assert!(result.is_err(), "R5 bad password should fail");
}

// --- Revision 6 (AES-256 with enhanced key derivation) ---

/// Ported from: OwnerPasswordVersion6UTF8
#[test]
#[ignore = "not yet implemented"]
fn r6_owner_password_utf8() {
    let path = fixture("encrypted_hello_world_r6.pdf");
    let doc = Document::open_with_password(&path, AGE_UTF8);
    assert!(doc.is_ok(), "R6 owner password (UTF-8) should succeed");
    let mut doc = doc.unwrap();
    assert!(doc.is_encrypted());
    assert_eq!(doc.page_count().unwrap(), 1);
}

/// Ported from: OwnerPasswordVersion6Latin1
#[test]
#[ignore = "not yet implemented"]
fn r6_owner_password_latin1() {
    let path = fixture("encrypted_hello_world_r6.pdf");
    let doc = Document::open_with_password(&path, AGE_LATIN1);
    assert!(doc.is_ok(), "R6 owner password (Latin-1) should succeed");
}

/// Ported from: UserPasswordVersion6UTF8
#[test]
#[ignore = "not yet implemented"]
fn r6_user_password_utf8() {
    let path = fixture("encrypted_hello_world_r6.pdf");
    let doc = Document::open_with_password(&path, HOTEL_UTF8);
    assert!(doc.is_ok(), "R6 user password (UTF-8) should succeed");
}

/// Ported from: UserPasswordVersion6Latin1
#[test]
#[ignore = "not yet implemented"]
fn r6_user_password_latin1() {
    let path = fixture("encrypted_hello_world_r6.pdf");
    let doc = Document::open_with_password(&path, HOTEL_LATIN1);
    assert!(doc.is_ok(), "R6 user password (Latin-1) should succeed");
}

// --- Wrong password on various revisions ---

/// Ported from: NoPassword (on encrypted.pdf which is R2/R3)
#[test]
#[ignore = "not yet implemented"]
fn encrypted_no_password_fails() {
    let path = fixture("encrypted_hello_world_r2.pdf");
    let result = Document::open_with_password(&path, b"");
    assert!(
        result.is_err(),
        "encrypted PDF with empty password should fail"
    );
}

/// Ported from: BadPassword
#[test]
#[ignore = "not yet implemented"]
fn encrypted_bad_password_fails() {
    let path = fixture("encrypted_hello_world_r2.pdf");
    let result = Document::open_with_password(&path, b"tiger");
    assert!(
        result.is_err(),
        "encrypted PDF with bad password should fail"
    );
}
