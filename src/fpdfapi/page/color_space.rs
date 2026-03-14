use crate::fxge::color::Color;

/// PDF color space (PDF spec §8.6).
///
/// ICCBased/CalGray/CalRGB are approximated as their Device equivalents.
#[derive(Debug, Clone, PartialEq)]
pub enum ColorSpace {
    DeviceGray,
    DeviceRGB,
    DeviceCMYK,
}

impl ColorSpace {
    /// Convert color components to an RGBA `Color`.
    ///
    /// The number of components must match the color space:
    /// - DeviceGray: 1 component
    /// - DeviceRGB: 3 components
    /// - DeviceCMYK: 4 components
    ///
    /// Returns `None` if the component count is wrong.
    pub fn to_color(&self, components: &[f32]) -> Option<Color> {
        match self {
            ColorSpace::DeviceGray => {
                let [g] = components else { return None };
                let v = (g.clamp(0.0, 1.0) * 255.0).round() as u8;
                Some(Color::gray(v))
            }
            ColorSpace::DeviceRGB => {
                let [r, g, b] = components else {
                    return None;
                };
                Some(Color::rgb(
                    (r.clamp(0.0, 1.0) * 255.0).round() as u8,
                    (g.clamp(0.0, 1.0) * 255.0).round() as u8,
                    (b.clamp(0.0, 1.0) * 255.0).round() as u8,
                ))
            }
            ColorSpace::DeviceCMYK => {
                let [c, m, y, k] = components else {
                    return None;
                };
                Some(Color::from_cmyk(*c, *m, *y, *k))
            }
        }
    }
}

/// Tracks fill and stroke color state for the graphics state stack.
#[derive(Debug, Clone)]
pub struct ColorState {
    pub fill_color_space: ColorSpace,
    pub fill_components: Vec<f32>,
    pub stroke_color_space: ColorSpace,
    pub stroke_components: Vec<f32>,
}

impl Default for ColorState {
    /// PDF default: DeviceGray with value 0.0 (black) for both fill and stroke.
    fn default() -> Self {
        ColorState {
            fill_color_space: ColorSpace::DeviceGray,
            fill_components: vec![0.0],
            stroke_color_space: ColorSpace::DeviceGray,
            stroke_components: vec![0.0],
        }
    }
}

impl ColorState {
    /// Resolve the current fill color.
    pub fn fill_color(&self) -> Color {
        self.fill_color_space
            .to_color(&self.fill_components)
            .unwrap_or(Color::BLACK)
    }

    /// Resolve the current stroke color.
    pub fn stroke_color(&self) -> Color {
        self.stroke_color_space
            .to_color(&self.stroke_components)
            .unwrap_or(Color::BLACK)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- ColorSpace::to_color ---

    #[test]
    fn gray_zero_is_black() {
        let c = ColorSpace::DeviceGray.to_color(&[0.0]).unwrap();
        assert_eq!(c, Color::BLACK);
    }

    #[test]
    fn gray_one_is_white() {
        let c = ColorSpace::DeviceGray.to_color(&[1.0]).unwrap();
        assert_eq!(c, Color::WHITE);
    }

    #[test]
    fn gray_half() {
        let c = ColorSpace::DeviceGray.to_color(&[0.5]).unwrap();
        assert_eq!(c.r, 128);
        assert_eq!(c.r, c.g);
        assert_eq!(c.g, c.b);
        assert_eq!(c.a, 255);
    }

    #[test]
    fn rgb_red() {
        let c = ColorSpace::DeviceRGB.to_color(&[1.0, 0.0, 0.0]).unwrap();
        assert_eq!(c, Color::rgb(255, 0, 0));
    }

    #[test]
    fn rgb_green() {
        let c = ColorSpace::DeviceRGB.to_color(&[0.0, 1.0, 0.0]).unwrap();
        assert_eq!(c, Color::rgb(0, 255, 0));
    }

    #[test]
    fn cmyk_cyan() {
        let c = ColorSpace::DeviceCMYK
            .to_color(&[1.0, 0.0, 0.0, 0.0])
            .unwrap();
        assert_eq!(c, Color::from_cmyk(1.0, 0.0, 0.0, 0.0));
    }

    #[test]
    fn wrong_component_count_returns_none() {
        assert!(ColorSpace::DeviceGray.to_color(&[]).is_none());
        assert!(ColorSpace::DeviceRGB.to_color(&[1.0]).is_none());
        assert!(ColorSpace::DeviceCMYK.to_color(&[1.0, 0.0]).is_none());
    }

    // --- ColorState ---

    #[test]
    fn color_state_default_is_black() {
        let cs = ColorState::default();
        assert_eq!(cs.fill_color(), Color::BLACK);
        assert_eq!(cs.stroke_color(), Color::BLACK);
    }
}
