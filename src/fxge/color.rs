/// RGBA color.
///
/// Corresponds to C++ `FX_ARGB` (packed u32), restructured for clarity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const WHITE: Color = Color {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    };
    pub const BLACK: Color = Color {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    };
    pub const TRANSPARENT: Color = Color {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    };

    pub fn gray(v: u8) -> Self {
        todo!()
    }

    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        todo!()
    }

    pub fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        todo!()
    }

    /// Approximate CMYK → RGB conversion.
    ///
    /// Components are in [0.0, 1.0] range.
    pub fn from_cmyk(c: f32, m: f32, y: f32, k: f32) -> Self {
        todo!()
    }
}

/// Line cap style (PDF spec §8.4.3.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LineCap {
    #[default]
    Butt,
    Round,
    Square,
}

/// Line join style (PDF spec §8.4.3.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LineJoin {
    #[default]
    Miter,
    Round,
    Bevel,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "not yet implemented"]
    fn color_gray() {
        let c = Color::gray(0);
        assert_eq!(
            c,
            Color {
                r: 0,
                g: 0,
                b: 0,
                a: 255
            }
        );
        let c = Color::gray(128);
        assert_eq!(
            c,
            Color {
                r: 128,
                g: 128,
                b: 128,
                a: 255
            }
        );
        let c = Color::gray(255);
        assert_eq!(
            c,
            Color {
                r: 255,
                g: 255,
                b: 255,
                a: 255
            }
        );
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn color_rgb() {
        let c = Color::rgb(255, 0, 0);
        assert_eq!(c.r, 255);
        assert_eq!(c.g, 0);
        assert_eq!(c.b, 0);
        assert_eq!(c.a, 255);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn color_rgba() {
        let c = Color::rgba(10, 20, 30, 128);
        assert_eq!(c.r, 10);
        assert_eq!(c.g, 20);
        assert_eq!(c.b, 30);
        assert_eq!(c.a, 128);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn color_from_cmyk_cyan() {
        // CMYK (1,0,0,0) → cyan
        let c = Color::from_cmyk(1.0, 0.0, 0.0, 0.0);
        assert_eq!(c.r, 0);
        assert_eq!(c.g, 255);
        assert_eq!(c.b, 255);
        assert_eq!(c.a, 255);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn color_from_cmyk_black() {
        // CMYK (0,0,0,1) → black
        let c = Color::from_cmyk(0.0, 0.0, 0.0, 1.0);
        assert_eq!(c.r, 0);
        assert_eq!(c.g, 0);
        assert_eq!(c.b, 0);
        assert_eq!(c.a, 255);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn color_from_cmyk_white() {
        // CMYK (0,0,0,0) → white
        let c = Color::from_cmyk(0.0, 0.0, 0.0, 0.0);
        assert_eq!(c.r, 255);
        assert_eq!(c.g, 255);
        assert_eq!(c.b, 255);
        assert_eq!(c.a, 255);
    }
}
