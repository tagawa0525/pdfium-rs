use crate::fpdfapi::font::pdf_font::PdfFont;
use crate::fxcrt::coordinates::{Matrix, Point};

/// Text rendering state (PDF text state parameters).
#[derive(Debug, Clone)]
pub struct TextState {
    /// Font size in text space units.
    pub font_size: f64,
    /// Character spacing (Tc operator), in unscaled text-space units.
    pub char_space: f64,
    /// Word spacing (Tw operator), in unscaled text-space units.
    pub word_space: f64,
    /// Text rendering mode (Tr operator): 0=fill, 1=stroke, …
    pub text_rendering_mode: u8,
}

impl Default for TextState {
    fn default() -> Self {
        TextState {
            font_size: 0.0,
            char_space: 0.0,
            word_space: 0.0,
            text_rendering_mode: 0,
        }
    }
}

/// Graphics state relevant to text extraction (Phase 3 subset).
///
/// `Clone` is used to save/restore state for `q`/`Q` operators.
#[derive(Debug, Clone)]
pub struct GraphicsState {
    /// Current transformation matrix (CTM).
    pub ctm: Matrix,
    /// Text matrix set by `Tm` operator.
    pub text_matrix: Matrix,
    /// Current text position in text-matrix space (accumulates advances and `Td` offsets).
    pub text_pos: Point,
    /// Start-of-line position in text-matrix space (updated by `Td`, `TD`, `T*`).
    pub text_line_pos: Point,
    /// Text leading (TL operator), in unscaled text-space units.
    pub text_leading: f64,
    /// Text rise (Ts operator), in unscaled text-space units.
    pub text_rise: f64,
    /// Horizontal scaling (Tz operator) as a fraction: 100% → 1.0.
    pub text_horz_scale: f64,
    /// Text state parameters.
    pub text_state: TextState,
    /// Currently active font (set by `Tf` operator).
    pub font: Option<PdfFont>,
}

impl Default for GraphicsState {
    fn default() -> Self {
        GraphicsState {
            ctm: Matrix::default(),
            text_matrix: Matrix::default(),
            text_pos: Point::default(),
            text_line_pos: Point::default(),
            text_leading: 0.0,
            text_rise: 0.0,
            text_horz_scale: 1.0,
            text_state: TextState::default(),
            font: None,
        }
    }
}

impl GraphicsState {
    /// Move the text line origin by `(dx, dy)` in text-matrix space and reset
    /// the current text position to the new line origin.
    ///
    /// Implements the `Td` operator.
    pub fn move_text_point(&mut self, dx: f64, dy: f64) {
        self.text_line_pos.x += dx as f32;
        self.text_line_pos.y += dy as f32;
        self.text_pos = self.text_line_pos;
    }

    /// Move to the start of the next line using the current leading.
    ///
    /// Equivalent to `move_text_point(0.0, -text_leading)`. Implements `T*`.
    pub fn move_to_next_line(&mut self) {
        self.move_text_point(0.0, -self.text_leading);
    }

    /// Set the text matrix and reset both text position and line position to the origin.
    ///
    /// Implements the `Tm` operator.
    pub fn set_text_matrix(&mut self, a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) {
        self.text_matrix = Matrix::new(a as f32, b as f32, c as f32, d as f32, e as f32, f as f32);
        self.text_pos = Point::default();
        self.text_line_pos = Point::default();
    }

    /// Advance the current text position horizontally by `dx` (in text-matrix space).
    ///
    /// Called after rendering each character glyph.
    pub fn advance_text_position(&mut self, dx: f64) {
        self.text_pos.x += dx as f32;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_state_default_values() {
        let ts = TextState::default();
        assert_eq!(ts.font_size, 0.0);
        assert_eq!(ts.char_space, 0.0);
        assert_eq!(ts.word_space, 0.0);
        assert_eq!(ts.text_rendering_mode, 0);
    }

    #[test]
    fn graphics_state_default_values() {
        let gs = GraphicsState::default();
        assert!(gs.ctm.is_identity());
        assert!(gs.text_matrix.is_identity());
        assert_eq!(gs.text_pos, Point::default());
        assert_eq!(gs.text_line_pos, Point::default());
        assert_eq!(gs.text_leading, 0.0);
        assert_eq!(gs.text_rise, 0.0);
        assert_eq!(gs.text_horz_scale, 1.0);
        assert!(gs.font.is_none());
    }

    #[test]
    fn move_text_point_updates_both_positions() {
        let mut gs = GraphicsState::default();
        gs.move_text_point(10.0, -5.0);
        assert!((gs.text_pos.x as f64 - 10.0).abs() < 1e-5);
        assert!((gs.text_pos.y as f64 - (-5.0)).abs() < 1e-5);
        assert!((gs.text_line_pos.x as f64 - 10.0).abs() < 1e-5);
        assert!((gs.text_line_pos.y as f64 - (-5.0)).abs() < 1e-5);
    }

    #[test]
    fn move_text_point_accumulates() {
        let mut gs = GraphicsState::default();
        gs.move_text_point(10.0, -5.0);
        gs.move_text_point(5.0, -3.0);
        assert!((gs.text_line_pos.x as f64 - 15.0).abs() < 1e-5);
        assert!((gs.text_line_pos.y as f64 - (-8.0)).abs() < 1e-5);
        assert_eq!(gs.text_pos, gs.text_line_pos);
    }

    #[test]
    fn move_to_next_line_uses_leading() {
        let mut gs = GraphicsState {
            text_leading: 12.0,
            text_line_pos: Point::new(100.0, 500.0),
            text_pos: Point::new(100.0, 500.0),
            ..Default::default()
        };
        gs.move_to_next_line();
        assert!((gs.text_line_pos.x as f64 - 100.0).abs() < 1e-5);
        assert!((gs.text_line_pos.y as f64 - 488.0).abs() < 1e-4);
        assert_eq!(gs.text_pos, gs.text_line_pos);
    }

    #[test]
    fn set_text_matrix_resets_positions() {
        let mut gs = GraphicsState {
            text_pos: Point::new(50.0, 100.0),
            text_line_pos: Point::new(50.0, 100.0),
            ..Default::default()
        };
        gs.set_text_matrix(1.0, 0.0, 0.0, 1.0, 200.0, 300.0);
        assert_eq!(gs.text_pos, Point::default());
        assert_eq!(gs.text_line_pos, Point::default());
        assert!((gs.text_matrix.e - 200.0).abs() < 1e-5);
        assert!((gs.text_matrix.f - 300.0).abs() < 1e-5);
    }

    #[test]
    fn advance_text_position_moves_x() {
        let mut gs = GraphicsState::default();
        gs.advance_text_position(15.5);
        assert!((gs.text_pos.x as f64 - 15.5).abs() < 1e-5);
        assert_eq!(gs.text_pos.y, 0.0);
    }

    #[test]
    fn advance_text_position_accumulates() {
        let mut gs = GraphicsState::default();
        gs.advance_text_position(10.0);
        gs.advance_text_position(5.0);
        assert!((gs.text_pos.x as f64 - 15.0).abs() < 1e-5);
    }

    #[test]
    fn graphics_state_clone_is_independent() {
        let mut gs = GraphicsState {
            text_leading: 12.0,
            ..Default::default()
        };
        let saved = gs.clone();
        gs.text_leading = 24.0;
        assert_eq!(saved.text_leading, 12.0);
        assert_eq!(gs.text_leading, 24.0);
    }
}
