use crate::fpdfapi::font::font_file::FontData;
use crate::fpdfapi::font::glyph;
use crate::fpdfapi::font::pdf_font::PdfFont;
use crate::fpdfapi::font::standard_fonts;
use crate::fpdfapi::page::page_object::TextObject;
use crate::fxge::color::Color;
use ttf_parser;

/// Render a `TextObject` (fill and/or stroke) onto the pixmap.
pub fn render_text(
    pixmap: &mut tiny_skia::Pixmap,
    text_obj: &TextObject,
    page_to_device: tiny_skia::Transform,
) {
    // PDF text rendering modes (PDF spec §9.3.6):
    //   0 = fill only
    //   1 = stroke only
    //   2 = fill + stroke
    //   3 = invisible
    //   4 = fill + clip (clip deferred; render like fill)
    //   5 = stroke + clip (clip deferred; render like stroke)
    //   6 = fill + stroke + clip (clip deferred; render like fill+stroke)
    //   7 = invisible + clip (clip deferred; no painting)
    let mode = text_obj.text_rendering_mode;
    // Modes 3 and 7 produce no paint (invisible)
    if mode == 3 || mode == 7 {
        return;
    }

    let font_data = match resolve_font_data(&text_obj.font) {
        Some(data) => data,
        None => return,
    };

    // Parse the font face once per TextObject (not per glyph).
    let face = match glyph::parse_face(font_data) {
        Some(f) => f,
        None => return,
    };
    let upm = face.units_per_em() as f32;
    let font_size = text_obj.font_size as f32;

    // Build shape matrix: rotation/scale from CTM × text_matrix (no translation)
    let shape = shape_matrix(&text_obj.ctm, &text_obj.text_matrix);

    // Modes 4-6 paint the same as 0-2 (clipping path update is deferred).
    let should_fill = matches!(mode, 0 | 2 | 4 | 6);
    let should_stroke = matches!(mode, 1 | 2 | 5 | 6);

    // Hoist Paint/Stroke construction outside the per-glyph loop (colors are constant).
    let fill_paint = if should_fill {
        let mut p = tiny_skia::Paint::default();
        p.set_color(color_to_tiny_skia(text_obj.fill_color));
        p.anti_alias = true;
        Some(p)
    } else {
        None
    };
    let (stroke_paint, stroke_style) = if should_stroke {
        let mut p = tiny_skia::Paint::default();
        p.set_color(color_to_tiny_skia(text_obj.stroke_color));
        p.anti_alias = true;
        let s = tiny_skia::Stroke {
            width: text_obj.line_width,
            ..Default::default()
        };
        (Some(p), Some(s))
    } else {
        (None, None)
    };

    let glyph_scale = font_size / upm;
    // Build the shape sub-transform (constant for all glyphs in this object).
    let shape_transform =
        tiny_skia::Transform::from_row(shape.a, shape.b, shape.c, shape.d, 0.0, 0.0)
            .pre_concat(tiny_skia::Transform::from_scale(glyph_scale, glyph_scale));

    for entry in &text_obj.char_entries {
        // Resolve character code → Unicode → glyph ID (reuse parsed face)
        let glyph_id = match resolve_glyph_id_from_face(&text_obj.font, entry.code, &face) {
            Some(id) => id,
            None => continue,
        };

        let glyph_path = match glyph::glyph_outline_from_face(&face, glyph_id) {
            Some(p) => p,
            None => continue,
        };

        // Transform: page_to_device × translate(origin) × shape × scale(fontSize/upm)
        // page_to_device already flips Y (PDF Y-up → device Y-down).
        // TrueType glyphs are also Y-up, so the flip naturally inverts them.
        let glyph_transform = page_to_device
            .pre_concat(tiny_skia::Transform::from_translate(
                entry.origin.x,
                entry.origin.y,
            ))
            .pre_concat(shape_transform);

        if let Some(ref paint) = fill_paint {
            pixmap.fill_path(
                &glyph_path,
                paint,
                tiny_skia::FillRule::Winding,
                glyph_transform,
                None,
            );
        }

        if let (Some(paint), Some(stroke)) = (&stroke_paint, &stroke_style) {
            pixmap.stroke_path(&glyph_path, paint, stroke, glyph_transform, None);
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

/// Resolve a character code to a glyph ID using a pre-parsed `Face`.
fn resolve_glyph_id_from_face(
    font: &PdfFont,
    code: u32,
    face: &ttf_parser::Face<'_>,
) -> Option<u16> {
    // Try unicode mapping first
    if let Some(unicode_str) = font.unicode_from_char_code(code)
        && let Some(ch) = unicode_str.chars().next()
        && let Some(gid) = glyph::char_to_glyph_id_from_face(face, ch)
    {
        return Some(gid);
    }
    // Fallback: try direct char code as Unicode code point
    if let Some(ch) = char::from_u32(code) {
        return glyph::char_to_glyph_id_from_face(face, ch);
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

    /// Use the bundled Liberation Sans font for deterministic tests.
    fn bundled_font_data() -> Vec<u8> {
        include_bytes!("../../../assets/fonts/LiberationSans-Regular.ttf").to_vec()
    }

    fn make_text_object() -> TextObject {
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
                font_data: Some(FontData::TrueType(bundled_font_data())),
            },
            font_size: 48.0,
            text_matrix: Matrix::default(),
            ctm: Matrix::default(),
            fill_color: Color::BLACK,
            stroke_color: Color::BLACK,
            text_rendering_mode: 0,
            line_width: 1.0,
        }
    }

    #[test]
    fn render_text_produces_non_white_pixels() {
        let text_obj = make_text_object();

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
        let mut text_obj = make_text_object();
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
        let mut text_obj = make_text_object();
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
