use crate::fpdfapi::page::pdf_page::Page;
use crate::fxcrt::coordinates::Matrix;

/// Compute the matrix that transforms PDF user-space coordinates to device
/// (bitmap) coordinates.
///
/// PDF: origin bottom-left, Y up, units = points (1/72 inch).
/// Device: origin top-left, Y down, units = pixels.
///
/// At `dpi` pixels/inch the scale factor is `dpi / 72.0`.
/// The resulting matrix: `[scale, 0, 0, -scale, 0, page_height_pts * scale]`.
pub fn page_to_device_matrix(page: &Page, dpi: f32) -> Matrix {
    let scale = dpi / 72.0;
    let page_height = page.media_box.height();
    // PDF → device: scale X, flip Y, translate Y origin to top
    Matrix::new(scale, 0.0, 0.0, -scale, 0.0, page_height * scale)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fxcrt::coordinates::Rect;

    fn letter_page() -> Page {
        Page {
            media_box: Rect::new(0.0, 0.0, 612.0, 792.0),
            crop_box: None,
            rotation: 0,
            objects: vec![],
        }
    }

    #[test]
    fn page_to_device_matrix_72dpi() {
        let m = page_to_device_matrix(&letter_page(), 72.0);
        // At 72 DPI, scale = 1.0 → [1, 0, 0, -1, 0, 792]
        assert!((m.a - 1.0).abs() < 1e-5);
        assert!((m.b - 0.0).abs() < 1e-5);
        assert!((m.c - 0.0).abs() < 1e-5);
        assert!((m.d - (-1.0)).abs() < 1e-5);
        assert!((m.e - 0.0).abs() < 1e-5);
        assert!((m.f - 792.0).abs() < 1e-5);
    }

    #[test]
    fn page_to_device_matrix_144dpi() {
        let m = page_to_device_matrix(&letter_page(), 144.0);
        // At 144 DPI, scale = 2.0 → [2, 0, 0, -2, 0, 1584]
        assert!((m.a - 2.0).abs() < 1e-5);
        assert!((m.d - (-2.0)).abs() < 1e-5);
        assert!((m.f - 1584.0).abs() < 1e-5);
    }
}
