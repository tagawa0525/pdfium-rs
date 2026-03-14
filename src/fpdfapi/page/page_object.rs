use crate::fpdfapi::font::pdf_font::PdfFont;
use crate::fxcrt::coordinates::{Matrix, Point};
use crate::fxge::color::{Color, LineCap, LineJoin};

/// A single character entry within a text object.
pub struct CharEntry {
    /// Raw character code (single-byte for Simple fonts, multi-byte for CID fonts).
    pub code: u32,
    /// Character origin in user space (after CTM × text_matrix × text_pos transformation).
    pub origin: Point,
    /// Glyph advance width in font units (1/1000 of text-space unit).
    pub width: f64,
}

/// A text object extracted from a content stream (`BT` … `ET` block).
pub struct TextObject {
    /// Character entries in rendering order (each entry is a full glyph record).
    pub char_entries: Vec<CharEntry>,
    /// The font active when this object was rendered.
    pub font: PdfFont,
    /// Font size in text-space units (from `Tf` operator).
    pub font_size: f64,
    /// Text matrix at the start of this text object.
    pub text_matrix: Matrix,
    /// Current transformation matrix at the start of this text object.
    pub ctm: Matrix,
}

/// Fill rule for path painting operations (PDF spec §8.5.3.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FillRule {
    /// Non-zero winding number rule.
    #[default]
    NonZero,
    /// Even-odd rule.
    EvenOdd,
    /// No fill (stroke only).
    None,
}

/// A path object with painting attributes.
pub struct PathObject {
    pub path: crate::fxge::path::Path,
    pub fill_rule: FillRule,
    pub stroke: bool,
    pub fill_color: Color,
    pub stroke_color: Color,
    pub line_width: f32,
    pub line_cap: LineCap,
    pub line_join: LineJoin,
    pub miter_limit: f32,
    pub dash_array: Vec<f32>,
    pub dash_phase: f32,
    pub ctm: Matrix,
}

/// An image object with decoded pixel data.
pub struct ImageObject {
    /// Decoded RGBA pixels.
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub ctm: Matrix,
}

/// A page content object.
pub enum PageObject {
    Text(Box<TextObject>),
    /// Path objects (lines, curves, rectangles).
    Path(Box<PathObject>),
    /// Image XObjects.
    Image(Box<ImageObject>),
    /// Form XObjects — minimally handled for recursion; stub otherwise.
    Form,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fpdfapi::font::pdf_font::PdfFont;

    #[test]
    fn char_entry_fields_accessible() {
        let entry = CharEntry {
            code: 65,
            origin: Point::new(10.0, 20.0),
            width: 600.0,
        };
        assert_eq!(entry.code, 65);
        assert_eq!(entry.origin, Point::new(10.0, 20.0));
        assert!((entry.width - 600.0).abs() < 1e-9);
    }

    #[test]
    fn text_object_fields_accessible() {
        let obj = TextObject {
            char_entries: vec![CharEntry {
                code: 72,
                origin: Point::new(0.0, 0.0),
                width: 750.0,
            }],
            font: PdfFont::Unsupported {
                base_font: "Helvetica".to_string(),
            },
            font_size: 12.0,
            text_matrix: Matrix::default(),
            ctm: Matrix::default(),
        };
        assert_eq!(obj.char_entries.len(), 1);
        assert!((obj.font_size - 12.0).abs() < 1e-9);
        assert!(obj.text_matrix.is_identity());
    }

    #[test]
    fn page_object_text_variant() {
        let obj = PageObject::Text(Box::new(TextObject {
            char_entries: vec![],
            font: PdfFont::Unsupported {
                base_font: "Times-Roman".to_string(),
            },
            font_size: 10.0,
            text_matrix: Matrix::default(),
            ctm: Matrix::default(),
        }));
        assert!(matches!(obj, PageObject::Text(_)));
    }

    #[test]
    fn page_object_stub_variants() {
        assert!(matches!(PageObject::Form, PageObject::Form));
    }

    #[test]
    fn path_object_default_fields() {
        let obj = PathObject {
            path: crate::fxge::path::Path::new(),
            fill_rule: FillRule::default(),
            stroke: false,
            fill_color: Color::BLACK,
            stroke_color: Color::BLACK,
            line_width: 1.0,
            line_cap: LineCap::default(),
            line_join: LineJoin::default(),
            miter_limit: 10.0,
            dash_array: Vec::new(),
            dash_phase: 0.0,
            ctm: Matrix::default(),
        };
        assert_eq!(obj.fill_rule, FillRule::NonZero);
        assert!(!obj.stroke);
        assert_eq!(obj.fill_color, Color::BLACK);
        assert_eq!(obj.line_width, 1.0);
        assert_eq!(obj.miter_limit, 10.0);
        assert!(obj.dash_array.is_empty());
        assert_eq!(obj.dash_phase, 0.0);
    }

    #[test]
    fn image_object_fields() {
        let obj = ImageObject {
            data: vec![255, 0, 0, 255],
            width: 1,
            height: 1,
            ctm: Matrix::default(),
        };
        assert_eq!(obj.width, 1);
        assert_eq!(obj.height, 1);
        assert_eq!(obj.data.len(), 4);
    }

    #[test]
    fn graphics_state_has_color_and_line_style() {
        let gs = crate::fpdfapi::page::graphics_state::GraphicsState::default();
        assert_eq!(gs.line_width, 1.0);
        assert_eq!(gs.line_cap, LineCap::Butt);
        assert_eq!(gs.line_join, LineJoin::Miter);
        assert_eq!(gs.miter_limit, 10.0);
        assert!(gs.dash_array.is_empty());
        assert_eq!(gs.dash_phase, 0.0);
        assert_eq!(gs.color_state.fill_color(), Color::BLACK);
    }
}
