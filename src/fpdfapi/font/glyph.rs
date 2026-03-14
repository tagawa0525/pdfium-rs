use tiny_skia::PathBuilder;
use ttf_parser::Face;

/// Parse font data into a `ttf_parser::Face`.
///
/// Returns `None` if the font data is invalid.
pub fn parse_face(font_data: &[u8]) -> Option<Face<'_>> {
    Face::parse(font_data, 0).ok()
}

/// Extract a glyph outline as a `tiny_skia::Path` from a parsed `Face`.
///
/// Returns `None` if the glyph ID is not found or has no outline (e.g. space).
pub fn glyph_outline_from_face(face: &Face<'_>, glyph_id: u16) -> Option<tiny_skia::Path> {
    let gid = ttf_parser::GlyphId(glyph_id);
    let mut builder = PathCollector::new();
    face.outline_glyph(gid, &mut builder)?;
    builder.finish()
}

/// Map a Unicode character to a glyph ID from a parsed `Face`.
pub fn char_to_glyph_id_from_face(face: &Face<'_>, unicode: char) -> Option<u16> {
    face.glyph_index(unicode).map(|gid| gid.0)
}

/// Extract a glyph outline as a `tiny_skia::Path`.
///
/// Returns `None` if the font data is invalid, the glyph ID is not found,
/// or the glyph has no outline (e.g. space characters).
pub fn glyph_outline(font_data: &[u8], glyph_id: u16) -> Option<tiny_skia::Path> {
    let face = Face::parse(font_data, 0).ok()?;
    glyph_outline_from_face(&face, glyph_id)
}

/// Map a Unicode character to a glyph ID using the font's cmap table.
///
/// Returns `None` if the font data is invalid or the character has no mapping.
pub fn char_to_glyph_id(font_data: &[u8], unicode: char) -> Option<u16> {
    let face = Face::parse(font_data, 0).ok()?;
    char_to_glyph_id_from_face(&face, unicode)
}

/// Get the font's units-per-em value.
///
/// Returns `None` if the font data is invalid.
pub fn units_per_em(font_data: &[u8]) -> Option<u16> {
    let face = Face::parse(font_data, 0).ok()?;
    Some(face.units_per_em())
}

/// Adapter from `ttf_parser::OutlineBuilder` to `tiny_skia::PathBuilder`.
struct PathCollector {
    builder: PathBuilder,
}

impl PathCollector {
    fn new() -> Self {
        PathCollector {
            builder: PathBuilder::new(),
        }
    }

    fn finish(self) -> Option<tiny_skia::Path> {
        self.builder.finish()
    }
}

impl ttf_parser::OutlineBuilder for PathCollector {
    fn move_to(&mut self, x: f32, y: f32) {
        self.builder.move_to(x, y);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.builder.line_to(x, y);
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.builder.quad_to(x1, y1, x, y);
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.builder.cubic_to(x1, y1, x2, y2, x, y);
    }

    fn close(&mut self) {
        self.builder.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Find a Liberation Sans font from standard Linux paths.
    /// Returns None if not found (tests that depend on this will be skipped).
    fn find_liberation_sans() -> Option<Vec<u8>> {
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
        None
    }

    #[test]
    fn glyph_outline_returns_path_for_letter_a() {
        let Some(font_data) = find_liberation_sans() else {
            eprintln!("skipping: Liberation Sans not found on system");
            return;
        };
        let gid = char_to_glyph_id(&font_data, 'A').expect("should map 'A'");
        let path = glyph_outline(&font_data, gid);
        assert!(path.is_some(), "'A' should have an outline");
    }

    #[test]
    fn units_per_em_returns_expected_value() {
        let Some(font_data) = find_liberation_sans() else {
            eprintln!("skipping: Liberation Sans not found on system");
            return;
        };
        let upm = units_per_em(&font_data).expect("should have units_per_em");
        // Liberation Sans has upm of 2048
        assert!(upm > 0, "units_per_em should be positive");
    }

    #[test]
    fn space_has_no_outline() {
        let Some(font_data) = find_liberation_sans() else {
            eprintln!("skipping: Liberation Sans not found on system");
            return;
        };
        let gid = char_to_glyph_id(&font_data, ' ').expect("should map space");
        let path = glyph_outline(&font_data, gid);
        assert!(path.is_none(), "space should have no outline");
    }

    #[test]
    fn invalid_font_data_returns_none() {
        assert!(glyph_outline(b"not a font", 0).is_none());
        assert!(char_to_glyph_id(b"not a font", 'A').is_none());
        assert!(units_per_em(b"not a font").is_none());
    }
}
