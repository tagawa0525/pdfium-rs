use crate::fpdfapi::font::font_file::FontData;
use crate::fpdfapi::font::glyph;
use crate::fpdfapi::font::pdf_font::PdfFont;
use crate::fpdfapi::font::standard_fonts;
use crate::fpdfapi::page::page_object::TextObject;
use crate::fxge::color::Color;

/// Render a `TextObject` (fill and/or stroke) onto the pixmap.
pub fn render_text(
    pixmap: &mut tiny_skia::Pixmap,
    text_obj: &TextObject,
    page_to_device: tiny_skia::Transform,
) {
    // Mode 3 = invisible
    if text_obj.text_rendering_mode == 3 {
        return;
    }

    let font_data = match resolve_font_data(&text_obj.font) {
        Some(data) => data,
        None => return,
    };

    let upm = match glyph::units_per_em(font_data) {
        Some(upm) => upm as f32,
        None => return,
    };

    let font_size = text_obj.font_size as f32;

    // Build shape matrix: rotation/scale from CTM × text_matrix (no translation)
    let shape = shape_matrix(&text_obj.ctm, &text_obj.text_matrix);

    let should_fill = text_obj.text_rendering_mode == 0 || text_obj.text_rendering_mode == 2;
    let should_stroke = text_obj.text_rendering_mode == 1 || text_obj.text_rendering_mode == 2;

    for entry in &text_obj.char_entries {
        // Resolve character code → Unicode → glyph ID
        let glyph_id = match resolve_glyph_id(&text_obj.font, entry.code, font_data) {
            Some(id) => id,
            None => continue,
        };

        let glyph_path = match glyph::glyph_outline(font_data, glyph_id) {
            Some(p) => p,
            None => continue,
        };

        // Transform: page_to_device × translate(origin) × shape × scale(fontSize/upm)
        // TrueType glyphs are Y-up; page_to_device flips Y.
        let glyph_scale = font_size / upm;
        // page_to_device already flips Y (PDF Y-up → device Y-down).
        // TrueType glyphs are also Y-up, so the flip from page_to_device
        // naturally inverts them into device space. No additional Y-negate needed.
        let glyph_transform = page_to_device
            .pre_concat(tiny_skia::Transform::from_translate(
                entry.origin.x,
                entry.origin.y,
            ))
            .pre_concat(tiny_skia::Transform::from_row(
                shape.a, shape.b, shape.c, shape.d, 0.0, 0.0,
            ))
            .pre_concat(tiny_skia::Transform::from_scale(glyph_scale, glyph_scale));

        if should_fill {
            let mut paint = tiny_skia::Paint::default();
            paint.set_color(color_to_tiny_skia(text_obj.fill_color));
            paint.anti_alias = true;
            pixmap.fill_path(
                &glyph_path,
                &paint,
                tiny_skia::FillRule::Winding,
                glyph_transform,
                None,
            );
        }

        if should_stroke {
            let mut paint = tiny_skia::Paint::default();
            paint.set_color(color_to_tiny_skia(text_obj.stroke_color));
            paint.anti_alias = true;
            let stroke = tiny_skia::Stroke {
                width: 1.0,
                ..Default::default()
            };
            pixmap.stroke_path(&glyph_path, &paint, &stroke, glyph_transform, None);
        }
    }
}

/// Get font file bytes from a PdfFont: embedded data first, then standard font fallback.
fn resolve_font_data(font: &PdfFont) -> Option<&[u8]> {
    if let PdfFont::Simple {
        font_data: Some(data),
        ..
    } = font
    {
        match data {
            FontData::TrueType(bytes) | FontData::OpenType(bytes) => return Some(bytes),
            FontData::Type1(_) => {} // ttf-parser doesn't handle PFB; fall through
        }
    }
    // Fallback to standard 14 font
    let base_font = match font {
        PdfFont::Simple { base_font, .. } | PdfFont::Unsupported { base_font } => base_font,
    };
    standard_fonts::standard_font_data(base_font)
}

/// Resolve a character code to a glyph ID using the font's encoding and cmap.
fn resolve_glyph_id(font: &PdfFont, code: u32, font_data: &[u8]) -> Option<u16> {
    // Try unicode mapping first
    if let Some(unicode_str) = font.unicode_from_char_code(code)
        && let Some(ch) = unicode_str.chars().next()
        && let Some(gid) = glyph::char_to_glyph_id(font_data, ch)
    {
        return Some(gid);
    }
    // Fallback: try direct char code as Unicode code point
    if let Some(ch) = char::from_u32(code) {
        return glyph::char_to_glyph_id(font_data, ch);
    }
    None
}

/// Extract the rotation/scale component from CTM × text_matrix (strip translation).
fn shape_matrix(
    ctm: &crate::fxcrt::coordinates::Matrix,
    text_matrix: &crate::fxcrt::coordinates::Matrix,
) -> crate::fxcrt::coordinates::Matrix {
    // Multiply ctm × text_matrix, then zero out translation
    let mut result = *ctm;
    result.concat(text_matrix);
    result.e = 0.0;
    result.f = 0.0;
    result
}

fn color_to_tiny_skia(c: Color) -> tiny_skia::Color {
    tiny_skia::Color::from_rgba8(c.r, c.g, c.b, c.a)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fpdfapi::font::font_file::FontData;
    use crate::fpdfapi::font::pdf_font::PdfFont;
    use crate::fpdfapi::page::page_object::{CharEntry, TextObject};
    use crate::fxcrt::coordinates::{Matrix, Point};
    use crate::fxge::color::Color;

    fn test_font_data() -> Option<Vec<u8>> {
        let paths = [
            "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
            "/usr/share/fonts/liberation-sans/LiberationSans-Regular.ttf",
            "/usr/share/fonts/TTF/LiberationSans-Regular.ttf",
        ];
        for path in &paths {
            if let Ok(data) = std::fs::read(path) {
                return Some(data);
            }
        }
        // Try nix store
        if let Ok(entries) = std::fs::read_dir("/nix/store") {
            for entry in entries.flatten() {
                let candidate = entry
                    .path()
                    .join("share/fonts/truetype/LiberationSans-Regular.ttf");
                if candidate.exists()
                    && let Ok(data) = std::fs::read(&candidate)
                {
                    return Some(data);
                }
            }
        }
        None
    }

    fn make_text_object(font_data_bytes: Vec<u8>) -> TextObject {
        use crate::fpdfapi::font::encoding::{FontEncoding, PredefinedEncoding};
        TextObject {
            char_entries: vec![CharEntry {
                code: 72, // 'H'
                origin: Point::new(50.0, 150.0),
                width: 722.0,
            }],
            font: PdfFont::Simple {
                base_font: "TestFont".to_string(),
                encoding: FontEncoding::Predefined(PredefinedEncoding::WinAnsi),
                first_char: 0,
                widths: vec![],
                to_unicode: None,
                font_data: Some(FontData::TrueType(font_data_bytes)),
            },
            font_size: 48.0,
            text_matrix: Matrix::default(),
            ctm: Matrix::default(),
            fill_color: Color::BLACK,
            stroke_color: Color::BLACK,
            text_rendering_mode: 0,
        }
    }

    #[test]
    fn render_text_produces_non_white_pixels() {
        let Some(font_data_bytes) = test_font_data() else {
            eprintln!("skipping test: no test font found");
            return;
        };
        let text_obj = make_text_object(font_data_bytes);

        let mut pixmap = tiny_skia::Pixmap::new(200, 200).unwrap();
        pixmap.fill(tiny_skia::Color::WHITE);

        // page_to_device: identity with Y-flip for a 200pt page
        let page_to_device = tiny_skia::Transform::from_row(1.0, 0.0, 0.0, -1.0, 0.0, 200.0);

        render_text(&mut pixmap, &text_obj, page_to_device);

        // Check that some pixels are non-white (text was drawn)
        let has_non_white = pixmap
            .data()
            .chunks_exact(4)
            .any(|p| p[0] != 255 || p[1] != 255 || p[2] != 255);
        assert!(
            has_non_white,
            "text rendering should produce non-white pixels"
        );
    }

    #[test]
    fn invisible_mode_produces_all_white() {
        let Some(font_data_bytes) = test_font_data() else {
            eprintln!("skipping test: no test font found");
            return;
        };
        let mut text_obj = make_text_object(font_data_bytes);
        text_obj.text_rendering_mode = 3; // invisible

        let mut pixmap = tiny_skia::Pixmap::new(200, 200).unwrap();
        pixmap.fill(tiny_skia::Color::WHITE);

        let page_to_device = tiny_skia::Transform::from_row(1.0, 0.0, 0.0, -1.0, 0.0, 200.0);

        render_text(&mut pixmap, &text_obj, page_to_device);

        let all_white = pixmap
            .data()
            .chunks_exact(4)
            .all(|p| p[0] == 255 && p[1] == 255 && p[2] == 255 && p[3] == 255);
        assert!(all_white, "invisible mode should produce all white");
    }

    #[test]
    fn render_text_with_color() {
        let Some(font_data_bytes) = test_font_data() else {
            eprintln!("skipping test: no test font found");
            return;
        };
        let mut text_obj = make_text_object(font_data_bytes);
        text_obj.fill_color = Color::rgb(255, 0, 0); // red

        let mut pixmap = tiny_skia::Pixmap::new(200, 200).unwrap();
        pixmap.fill(tiny_skia::Color::WHITE);

        let page_to_device = tiny_skia::Transform::from_row(1.0, 0.0, 0.0, -1.0, 0.0, 200.0);

        render_text(&mut pixmap, &text_obj, page_to_device);

        // Check that some pixels have red component
        let has_red = pixmap
            .data()
            .chunks_exact(4)
            .any(|p| p[0] > 200 && p[1] < 50 && p[2] < 50);
        assert!(has_red, "red text should produce red pixels");
    }
}
