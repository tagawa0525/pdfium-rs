use super::color::Color;
use crate::error::Error;

/// RGBA-8888 bitmap.
///
/// Pixel layout: row-major, top-to-bottom, 4 bytes per pixel (R, G, B, A).
/// Corresponds to a simplified version of C++ `CFX_DIBitmap`.
#[derive(Debug, Clone)]
pub struct Bitmap {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

impl Bitmap {
    /// Create a new bitmap filled with transparent black.
    pub fn new(width: u32, height: u32) -> Self {
        todo!()
    }

    /// Fill the entire bitmap with the given color.
    pub fn clear(&mut self, color: Color) {
        todo!()
    }

    /// Read the color at pixel (x, y). Returns `None` if out of bounds.
    pub fn pixel_at(&self, x: u32, y: u32) -> Option<Color> {
        todo!()
    }

    /// Write the color at pixel (x, y). No-op if out of bounds.
    pub fn set_pixel(&mut self, x: u32, y: u32, color: Color) {
        todo!()
    }

    /// Encode the bitmap as a PNG and return the bytes.
    pub fn encode_png(&self) -> Result<Vec<u8>, Error> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "not yet implemented"]
    fn bitmap_new_dimensions() {
        let bmp = Bitmap::new(4, 2);
        assert_eq!(bmp.width, 4);
        assert_eq!(bmp.height, 2);
        assert_eq!(bmp.data.len(), 4 * 2 * 4);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn bitmap_new_transparent() {
        let bmp = Bitmap::new(2, 2);
        for &byte in &bmp.data {
            assert_eq!(byte, 0);
        }
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn bitmap_set_and_get_pixel() {
        let mut bmp = Bitmap::new(2, 2);
        let red = Color::rgb(255, 0, 0);
        bmp.set_pixel(1, 0, red);
        assert_eq!(bmp.pixel_at(1, 0), Some(red));
        assert_eq!(bmp.pixel_at(0, 0), Some(Color::TRANSPARENT));
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn bitmap_set_pixel_out_of_bounds_noop() {
        let mut bmp = Bitmap::new(2, 2);
        bmp.set_pixel(5, 5, Color::WHITE); // should not panic
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn bitmap_pixel_at_out_of_bounds() {
        let bmp = Bitmap::new(2, 2);
        assert_eq!(bmp.pixel_at(2, 0), None);
        assert_eq!(bmp.pixel_at(0, 2), None);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn bitmap_clear() {
        let mut bmp = Bitmap::new(3, 3);
        bmp.clear(Color::WHITE);
        for y in 0..3 {
            for x in 0..3 {
                assert_eq!(bmp.pixel_at(x, y), Some(Color::WHITE));
            }
        }
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn bitmap_encode_png_magic_bytes() {
        let bmp = Bitmap::new(1, 1);
        let png = bmp.encode_png().expect("PNG encoding should succeed");
        // PNG magic: 8 bytes
        assert!(png.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]));
    }
}
