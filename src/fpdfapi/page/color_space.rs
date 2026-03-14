use crate::fxge::color::Color;

/// PDF color space (PDF spec §8.6).
///
/// ICCBased/CalGray/CalRGB are approximated as their Device equivalents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorSpace {
    DeviceGray,
    DeviceRGB,
    DeviceCMYK,
}

impl ColorSpace {
    /// Number of components for this color space.
    pub fn num_components(&self) -> usize {
        match self {
            ColorSpace::DeviceGray => 1,
            ColorSpace::DeviceRGB => 3,
            ColorSpace::DeviceCMYK => 4,
        }
    }

    /// Convert color components to an RGBA `Color`.
    ///
    /// The number of components must match the color space:
    /// - DeviceGray: 1 component
    /// - DeviceRGB: 3 components
    /// - DeviceCMYK: 4 components
    ///
    /// All components are clamped to [0.0, 1.0].
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
                Some(Color::from_cmyk(
                    c.clamp(0.0, 1.0),
                    m.clamp(0.0, 1.0),
                    y.clamp(0.0, 1.0),
                    k.clamp(0.0, 1.0),
                ))
            }
        }
    }
}

/// Fixed-capacity buffer for color components (max 4: CMYK).
#[derive(Debug, Clone, Copy)]
pub struct Components {
    values: [f32; 4],
    len: u8,
}

impl Components {
    pub fn new(src: &[f32]) -> Self {
        let len = src.len().min(4);
        let mut values = [0.0f32; 4];
        values[..len].copy_from_slice(&src[..len]);
        Components {
            values,
            len: len as u8,
        }
    }

    pub fn as_slice(&self) -> &[f32] {
        &self.values[..self.len as usize]
    }
}

impl Default for Components {
    fn default() -> Self {
        Components {
            values: [0.0; 4],
            len: 1,
        }
    }
}

/// Tracks fill and stroke color state for the graphics state stack.
#[derive(Debug, Clone, Copy)]
pub struct ColorState {
    pub fill_color_space: ColorSpace,
    pub fill_components: Components,
    pub stroke_color_space: ColorSpace,
    pub stroke_components: Components,
}

impl Default for ColorState {
    /// PDF default: DeviceGray with value 0.0 (black) for both fill and stroke.
    fn default() -> Self {
        ColorState {
            fill_color_space: ColorSpace::DeviceGray,
            fill_components: Components::default(),
            stroke_color_space: ColorSpace::DeviceGray,
            stroke_components: Components::default(),
        }
    }
}

impl ColorState {
    /// Resolve the current fill color.
    pub fn fill_color(&self) -> Color {
        self.fill_color_space
            .to_color(self.fill_components.as_slice())
            .unwrap_or(Color::BLACK)
    }

    /// Resolve the current stroke color.
    pub fn stroke_color(&self) -> Color {
        self.stroke_color_space
            .to_color(self.stroke_components.as_slice())
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
