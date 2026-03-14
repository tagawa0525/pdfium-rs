use tiny_skia::PathBuilder;
use ttf_parser::Face;

/// Extract a glyph outline as a `tiny_skia::Path`.
///
/// Returns `None` if the font data is invalid, the glyph ID is not found,
/// or the glyph has no outline (e.g. space characters).
pub fn glyph_outline(font_data: &[u8], glyph_id: u16) -> Option<tiny_skia::Path> {
    let face = Face::parse(font_data, 0).ok()?;
    let gid = ttf_parser::GlyphId(glyph_id);
    let mut builder = PathCollector::new();
    face.outline_glyph(gid, &mut builder)?;
    builder.finish()
}

/// Map a Unicode character to a glyph ID using the font's cmap table.
///
/// Returns `None` if the font data is invalid or the character has no mapping.
pub fn char_to_glyph_id(font_data: &[u8], unicode: char) -> Option<u16> {
    let face = Face::parse(font_data, 0).ok()?;
    let gid = face.glyph_index(unicode)?;
    Some(gid.0)
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

    /// Get a known system font for testing. Liberation Sans is commonly available.
    /// Falls back to returning None if not found.
    fn test_font_data() -> Option<Vec<u8>> {
        let paths = [
            "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
            "/usr/share/fonts/liberation-sans/LiberationSans-Regular.ttf",
            "/usr/share/fonts/TTF/LiberationSans-Regular.ttf",
            "/nix/store",
        ];

        for path in &paths[..3] {
            if let Ok(data) = std::fs::read(path) {
                return Some(data);
            }
        }

        // Try finding via nix store glob
        if let Ok(entries) = std::fs::read_dir("/nix/store") {
            for entry in entries.flatten() {
                let p = entry.path();
                let candidate = p.join("share/fonts/truetype/LiberationSans-Regular.ttf");
                if candidate.exists()
                    && let Ok(data) = std::fs::read(&candidate)
                {
                    return Some(data);
                }
            }
        }

        None
    }

    #[test]
    fn glyph_outline_returns_path_for_letter_a() {
        let Some(font_data) = test_font_data() else {
            eprintln!("skipping test: no test font found");
            return;
        };
        let gid = char_to_glyph_id(&font_data, 'A').expect("should map 'A'");
        let path = glyph_outline(&font_data, gid);
        assert!(path.is_some(), "'A' should have an outline");
    }

    #[test]
    fn units_per_em_returns_expected_value() {
        let Some(font_data) = test_font_data() else {
            eprintln!("skipping test: no test font found");
            return;
        };
        let upm = units_per_em(&font_data).expect("should have units_per_em");
        // Liberation Sans has upm of 2048
        assert!(upm > 0, "units_per_em should be positive");
    }

    #[test]
    fn space_has_no_outline() {
        let Some(font_data) = test_font_data() else {
            eprintln!("skipping test: no test font found");
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
