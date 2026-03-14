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
        todo!()
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
    fn default() -> Self {
        todo!()
    }
}

impl ColorState {
    /// Resolve the current fill color.
    pub fn fill_color(&self) -> Color {
        todo!()
    }

    /// Resolve the current stroke color.
    pub fn stroke_color(&self) -> Color {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- ColorSpace::to_color ---

    #[test]
    #[ignore = "not yet implemented"]
    fn gray_zero_is_black() {
        let c = ColorSpace::DeviceGray.to_color(&[0.0]).unwrap();
        assert_eq!(c, Color::BLACK);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn gray_one_is_white() {
        let c = ColorSpace::DeviceGray.to_color(&[1.0]).unwrap();
        assert_eq!(c, Color::WHITE);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn gray_half() {
        let c = ColorSpace::DeviceGray.to_color(&[0.5]).unwrap();
        assert_eq!(c.r, 128);
        assert_eq!(c.r, c.g);
        assert_eq!(c.g, c.b);
        assert_eq!(c.a, 255);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn rgb_red() {
        let c = ColorSpace::DeviceRGB.to_color(&[1.0, 0.0, 0.0]).unwrap();
        assert_eq!(c, Color::rgb(255, 0, 0));
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn rgb_green() {
        let c = ColorSpace::DeviceRGB.to_color(&[0.0, 1.0, 0.0]).unwrap();
        assert_eq!(c, Color::rgb(0, 255, 0));
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn cmyk_cyan() {
        let c = ColorSpace::DeviceCMYK
            .to_color(&[1.0, 0.0, 0.0, 0.0])
            .unwrap();
        assert_eq!(c, Color::from_cmyk(1.0, 0.0, 0.0, 0.0));
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn wrong_component_count_returns_none() {
        assert!(ColorSpace::DeviceGray.to_color(&[]).is_none());
        assert!(ColorSpace::DeviceRGB.to_color(&[1.0]).is_none());
        assert!(ColorSpace::DeviceCMYK.to_color(&[1.0, 0.0]).is_none());
    }

    // --- ColorState ---

    #[test]
    #[ignore = "not yet implemented"]
    fn color_state_default_is_black() {
        let cs = ColorState::default();
        assert_eq!(cs.fill_color(), Color::BLACK);
        assert_eq!(cs.stroke_color(), Color::BLACK);
    }
}
