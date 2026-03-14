//! Integration tests for page rendering, ported from
//! `fpdf_progressive_render_embeddertest.cpp` and related embeddertests
//! in the C++ PDFium reference.
//!
//! These tests use real PDF fixtures from `tests/fixtures/` to exercise the
//! full pipeline: PDF parse → page tree → content stream → path rendering →
//! bitmap output.

use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

use pdfium_rs::fxge::color::Color;
use pdfium_rs::{Bitmap, Document};

/// Open a test PDF fixture by filename.
fn open_fixture(name: &str) -> Document<BufReader<File>> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    Document::open(&path).unwrap_or_else(|e| panic!("failed to open {}: {e}", path.display()))
}

/// Render page 0 of a fixture PDF at the given DPI.
fn render_fixture(name: &str, dpi: f32) -> Bitmap {
    let mut doc = open_fixture(name);
    let page = doc.page(0).unwrap();
    page.render(dpi).unwrap()
}

// ─── rectangles.pdf (200×300 pt) ────────────────────────────────────────────
//
// Content stream:
//   q
//   0 0 0 rg                        # black fill
//   0 290 10 10 re B*               # corner rect (top-left in device)
//   10 150 50 30 re B*              # center-left rect
//   0 0 1 rg                        # blue fill
//   190 290 10 10 re B*             # corner rect (top-right in device)
//   70 232 50 30 re B*              # upper-center rect
//   0 1 0 rg                        # green fill
//   190 0 10 10 re B*               # corner rect (bottom-right in device)
//   130 150 50 30 re B*             # center-right rect
//   1 0 0 rg                        # red fill
//   0 0 10 10 re B*                 # corner rect (bottom-left in device)
//   70 67 50 30 re B*               # lower-center rect
//   Q
//
// At 72 DPI (scale=1), device y = 300 - pdf_y.

#[test]
fn rectangles_render_dimensions() {
    let bmp = render_fixture("rectangles.pdf", 72.0);
    assert_eq!(bmp.width, 200);
    assert_eq!(bmp.height, 300);
}

/// Verify corner rectangles: each corner has a 10×10 rect with a specific color.
/// - Top-left:     black (0,0,0)     — PDF (0,290)
/// - Top-right:    blue  (0,0,255)   — PDF (190,290)
/// - Bottom-left:  red   (255,0,0)   — PDF (0,0)
/// - Bottom-right: green (0,255,0)   — PDF (190,0)
#[test]
fn rectangles_corner_colors() {
    let bmp = render_fixture("rectangles.pdf", 72.0);

    // Top-left corner: black rect, check center at device (5, 5)
    let tl = bmp.pixel_at(5, 5).unwrap();
    assert_eq!((tl.r, tl.g, tl.b), (0, 0, 0), "top-left should be black");

    // Top-right corner: blue rect, check center at device (195, 5)
    let tr = bmp.pixel_at(195, 5).unwrap();
    assert_eq!((tr.r, tr.g, tr.b), (0, 0, 255), "top-right should be blue");

    // Bottom-left corner: red rect, check center at device (5, 295)
    let bl = bmp.pixel_at(5, 295).unwrap();
    assert_eq!((bl.r, bl.g, bl.b), (255, 0, 0), "bottom-left should be red");

    // Bottom-right corner: green rect, check center at device (195, 295)
    let br = bmp.pixel_at(195, 295).unwrap();
    assert_eq!(
        (br.r, br.g, br.b),
        (0, 255, 0),
        "bottom-right should be green"
    );
}

/// Verify center rectangles have the correct fill color.
#[test]
fn rectangles_center_rect_colors() {
    let bmp = render_fixture("rectangles.pdf", 72.0);

    // Black center rect: PDF (10,150)-(60,180) → device (10, 120)-(60, 150), center ~(35, 135)
    let c = bmp.pixel_at(35, 135).unwrap();
    assert_eq!((c.r, c.g, c.b), (0, 0, 0), "center-left should be black");

    // Blue center rect: PDF (70,232)-(120,262) → device (70, 38)-(120, 68), center ~(95, 53)
    let c = bmp.pixel_at(95, 53).unwrap();
    assert_eq!((c.r, c.g, c.b), (0, 0, 255), "upper-center should be blue");

    // Green center rect: PDF (130,150)-(180,180) → device (130, 120)-(180, 150), center ~(155, 135)
    let c = bmp.pixel_at(155, 135).unwrap();
    assert_eq!((c.r, c.g, c.b), (0, 255, 0), "center-right should be green");

    // Red center rect: PDF (70,67)-(120,97) → device (70, 203)-(120, 233), center ~(95, 218)
    let c = bmp.pixel_at(95, 218).unwrap();
    assert_eq!((c.r, c.g, c.b), (255, 0, 0), "lower-center should be red");
}

/// Background pixels (not covered by any rectangle) should be white.
#[test]
fn rectangles_background_is_white() {
    let bmp = render_fixture("rectangles.pdf", 72.0);
    // Center of page (100, 150 device) is not covered by any rect
    assert_eq!(bmp.pixel_at(100, 150), Some(Color::WHITE));
    // Also check a few other empty spots
    assert_eq!(bmp.pixel_at(100, 10), Some(Color::WHITE));
    assert_eq!(bmp.pixel_at(100, 280), Some(Color::WHITE));
}

// ─── dashed_lines.pdf (200×100 pt) ──────────────────────────────────────────
//
// Content stream:
//   q
//   0 0 0 rg
//   10 25 m 190 25 l S              # solid black line at PDF y=25
//   [6 5 4 3 2 1] 5 d              # set dash pattern
//   10 50 m 190 50 l S              # dashed black line at PDF y=50
//   [] 0 d                          # reset dash
//   10 75 m 190 75 l S              # solid black line at PDF y=75
//   Q
//
// At 72 DPI: device y = 100 - pdf_y.

#[test]
fn dashed_lines_render_dimensions() {
    let bmp = render_fixture("dashed_lines.pdf", 72.0);
    assert_eq!(bmp.width, 200);
    assert_eq!(bmp.height, 100);
}

/// Verify pixels on the solid lines are dark (black or near-black with anti-aliasing).
/// PDF y=25 → device y=75, PDF y=75 → device y=25.
///
/// A single-pixel probe at the exact centerline can miss due to rasterization rounding
/// or anti-aliasing. Instead, scan a ±1 neighbourhood for at least one dark pixel.
fn has_dark_pixel_near(bmp: &Bitmap, cx: u32, cy: u32) -> bool {
    let lo_x = cx.saturating_sub(1);
    let lo_y = cy.saturating_sub(1);
    let hi_x = (cx + 1).min(bmp.width - 1);
    let hi_y = (cy + 1).min(bmp.height - 1);
    for y in lo_y..=hi_y {
        for x in lo_x..=hi_x {
            if bmp
                .pixel_at(x, y)
                .is_some_and(|p| p.r < 200 && p.g < 200 && p.b < 200)
            {
                return true;
            }
        }
    }
    false
}

#[test]
fn dashed_lines_solid_line_pixels() {
    let bmp = render_fixture("dashed_lines.pdf", 72.0);
    // Solid line at device y=75 (PDF y=25), x=100 (center of line)
    assert!(
        has_dark_pixel_near(&bmp, 100, 75),
        "solid line at y≈75 should have a dark pixel"
    );
    // Solid line at device y=25 (PDF y=75), x=100
    assert!(
        has_dark_pixel_near(&bmp, 100, 25),
        "solid line at y≈25 should have a dark pixel"
    );
}

/// Verify the dashed line (PDF y=50, device y=50) actually renders with gaps.
/// With dash pattern [6 5 4 3 2 1] and phase 5, the line should have alternating
/// dark (ink) and white (gap) pixels — not a fully solid black line.
/// Scan from x=10 to x=190 and assert both dark pixels and white pixels exist.
#[test]
fn dashed_line_has_gaps() {
    let bmp = render_fixture("dashed_lines.pdf", 72.0);
    let device_y = 50u32; // PDF y=50 → device y = 100-50 = 50
    let mut dark_count = 0u32;
    let mut white_count = 0u32;
    for x in 10u32..190 {
        // Check the pixel and its immediate neighbours for robustness
        let is_dark = (device_y.saturating_sub(1)..=(device_y + 1).min(bmp.height - 1))
            .filter_map(|y| bmp.pixel_at(x, y))
            .any(|p| p.r < 200 && p.g < 200 && p.b < 200);
        if is_dark {
            dark_count += 1;
        } else {
            white_count += 1;
        }
    }
    assert!(
        dark_count > 0,
        "dashed line should have at least one dark pixel"
    );
    assert!(
        white_count > 0,
        "dashed line should have at least one gap (white pixel)"
    );
}

// ─── DPI scaling ────────────────────────────────────────────────────────────

/// Rendering at 144 DPI should produce a bitmap twice the size of 72 DPI.
#[test]
fn dpi_scaling_doubles_dimensions() {
    let bmp72 = render_fixture("rectangles.pdf", 72.0);
    let bmp144 = render_fixture("rectangles.pdf", 144.0);
    assert_eq!(bmp72.width * 2, bmp144.width);
    assert_eq!(bmp72.height * 2, bmp144.height);
}

/// At 144 DPI the corner colors should still be correct.
#[test]
fn dpi_scaling_preserves_colors() {
    let bmp = render_fixture("rectangles.pdf", 144.0);
    // 144 DPI → 400×600 bitmap. Rects scale proportionally.
    // Top-left black rect: PDF (0,290)-(10,300) → device (0,0)-(20,20), center (10,10)
    let tl = bmp.pixel_at(10, 10).unwrap();
    assert_eq!((tl.r, tl.g, tl.b), (0, 0, 0), "top-left should be black");

    // Bottom-right green rect: PDF (190,0)-(200,10) → device (380,580)-(400,600), center (390,590)
    let br = bmp.pixel_at(390, 590).unwrap();
    assert_eq!(
        (br.r, br.g, br.b),
        (0, 255, 0),
        "bottom-right should be green"
    );
}

// ─── PNG encoding ───────────────────────────────────────────────────────────

/// Render and encode to PNG, verify PNG signature.
#[test]
fn render_to_png_has_valid_signature() {
    let bmp = render_fixture("rectangles.pdf", 72.0);
    let png_data = bmp.encode_png().expect("PNG encoding should succeed");
    assert!(
        png_data.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]),
        "PNG should start with magic bytes"
    );
    assert!(png_data.len() > 100, "PNG should have non-trivial size");
}

// ─── hello_world.pdf (text-only) ────────────────────────────────────────────

/// hello_world.pdf contains only text objects (no paths/images).
/// Rendering should succeed without panic, producing a white bitmap
/// (since text rendering is deferred to Phase 5).
#[test]
fn hello_world_renders_without_panic() {
    let bmp = render_fixture("hello_world.pdf", 72.0);
    // hello_world.pdf: 200×200 pt → 200×200 px at 72 DPI
    assert_eq!(bmp.width, 200);
    assert_eq!(bmp.height, 200);
    // Should be all white (text objects are skipped in Phase 4)
    assert_eq!(bmp.pixel_at(100, 100), Some(Color::WHITE));
}
