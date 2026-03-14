use crate::error::{Error, Result};
use crate::fpdfapi::page::page_object::PageObject;
use crate::fpdfapi::page::pdf_page::Page;
use crate::fxge::dib::Bitmap;

use super::context::page_to_device_matrix;
use super::image_renderer::render_image;
use super::path_renderer::render_path;

/// Render a PDF page to an RGBA bitmap at the given DPI.
///
/// 1. Compute the page-to-device transformation matrix.
/// 2. Create a `tiny_skia::Pixmap` and fill with white.
/// 3. Iterate over `page.objects` and dispatch to path/image renderers.
/// 4. Convert the `Pixmap` (premultiplied alpha) to a `Bitmap` (straight alpha).
pub fn render(page: &Page, dpi: f32) -> Result<Bitmap> {
    let matrix = page_to_device_matrix(page, dpi);
    let scale = dpi / 72.0;
    let width = (page.media_box.width() * scale).round() as u32;
    let height = (page.media_box.height() * scale).round() as u32;

    let mut pixmap = tiny_skia::Pixmap::new(width, height)
        .ok_or_else(|| Error::InvalidPdf("render: cannot create pixmap".into()))?;

    // Fill with opaque white
    pixmap.fill(tiny_skia::Color::WHITE);

    let page_to_device =
        tiny_skia::Transform::from_row(matrix.a, matrix.b, matrix.c, matrix.d, matrix.e, matrix.f);

    for obj in &page.objects {
        match obj {
            PageObject::Path(path_obj) => {
                render_path(&mut pixmap, path_obj, page_to_device);
            }
            PageObject::Image(image_obj) => {
                render_image(&mut pixmap, image_obj, page_to_device);
            }
            PageObject::Text(_) | PageObject::Form => {
                // Text rendering is Phase 5; Form XObjects are stubs.
            }
        }
    }

    Ok(pixmap_to_bitmap(&pixmap))
}

/// Convert a tiny_skia `Pixmap` (premultiplied alpha) to our `Bitmap` (straight alpha).
fn pixmap_to_bitmap(pixmap: &tiny_skia::Pixmap) -> Bitmap {
    let width = pixmap.width();
    let height = pixmap.height();
    let src = pixmap.data();
    let mut data = Vec::with_capacity(src.len());

    for pixel in src.chunks_exact(4) {
        let (r, g, b, a) = (pixel[0], pixel[1], pixel[2], pixel[3]);
        if a == 0 {
            data.extend_from_slice(&[0, 0, 0, 0]);
        } else if a == 255 {
            data.extend_from_slice(&[r, g, b, 255]);
        } else {
            // Un-premultiply: component = premul * 255 / alpha
            let a16 = a as u16;
            data.push(((r as u16 * 255 + a16 / 2) / a16) as u8);
            data.push(((g as u16 * 255 + a16 / 2) / a16) as u8);
            data.push(((b as u16 * 255 + a16 / 2) / a16) as u8);
            data.push(a);
        }
    }

    Bitmap {
        width,
        height,
        data,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fpdfapi::page::page_object::*;
    use crate::fxcrt::coordinates::{Matrix, Point, Rect};
    use crate::fxge::color::{Color, LineCap, LineJoin};
    use crate::fxge::path::Path;

    fn simple_page(width: f32, height: f32, objects: Vec<PageObject>) -> Page {
        Page {
            media_box: Rect::new(0.0, 0.0, width, height),
            crop_box: None,
            rotation: 0,
            objects,
        }
    }

    #[test]
    fn render_empty_page_is_all_white() {
        let page = simple_page(100.0, 100.0, vec![]);
        let bmp = render(&page, 72.0).unwrap();
        assert_eq!(bmp.width, 100);
        assert_eq!(bmp.height, 100);
        for y in 0..bmp.height {
            for x in 0..bmp.width {
                assert_eq!(bmp.pixel_at(x, y), Some(Color::WHITE));
            }
        }
    }

    #[test]
    fn render_red_rect_center_pixel() {
        // 100×100 pt page with a red filled rect from (25,25) to (75,75)
        let mut path = Path::new();
        path.append_rect(25.0, 25.0, 50.0, 50.0);
        let path_obj = PathObject {
            path,
            fill_rule: FillRule::NonZero,
            stroke: false,
            fill_color: Color::rgb(255, 0, 0),
            stroke_color: Color::BLACK,
            line_width: 1.0,
            line_cap: LineCap::Butt,
            line_join: LineJoin::Miter,
            miter_limit: 10.0,
            dash_array: vec![],
            dash_phase: 0.0,
            ctm: Matrix::default(),
        };
        let page = simple_page(100.0, 100.0, vec![PageObject::Path(Box::new(path_obj))]);
        let bmp = render(&page, 72.0).unwrap();
        // PDF rect (25,25)-(75,75) → device (25,25)-(75,75) at 72 DPI on 100pt page
        let center = bmp.pixel_at(50, 50).unwrap();
        assert_eq!(center.r, 255, "center should be red");
        assert_eq!(center.g, 0);
        assert_eq!(center.b, 0);
        // Outside the rect should be white
        assert_eq!(bmp.pixel_at(0, 0), Some(Color::WHITE));
    }

    #[test]
    fn render_stroke_line() {
        // 100×100 pt page with a green stroked horizontal line at y=50
        let mut path = Path::new();
        path.move_to(Point::new(10.0, 50.0));
        path.line_to(Point::new(90.0, 50.0));
        let path_obj = PathObject {
            path,
            fill_rule: FillRule::None,
            stroke: true,
            fill_color: Color::BLACK,
            stroke_color: Color::rgb(0, 255, 0),
            line_width: 2.0,
            line_cap: LineCap::Butt,
            line_join: LineJoin::Miter,
            miter_limit: 10.0,
            dash_array: vec![],
            dash_phase: 0.0,
            ctm: Matrix::default(),
        };
        let page = simple_page(100.0, 100.0, vec![PageObject::Path(Box::new(path_obj))]);
        let bmp = render(&page, 72.0).unwrap();
        // PDF y=50 → device y=50 on a 100pt page. A pixel on the line should be green.
        let on_line = bmp.pixel_at(50, 50).unwrap();
        assert_eq!(on_line.g, 255, "pixel on line should be green");
    }

    #[test]
    fn render_dpi_scaling_doubles_bitmap_size() {
        let page = simple_page(100.0, 200.0, vec![]);
        let bmp72 = render(&page, 72.0).unwrap();
        assert_eq!(bmp72.width, 100);
        assert_eq!(bmp72.height, 200);
        let bmp144 = render(&page, 144.0).unwrap();
        assert_eq!(bmp144.width, 200);
        assert_eq!(bmp144.height, 400);
    }

    #[test]
    fn render_image_object_placement() {
        // 100×100 pt page with a 2×2 red image placed at (25,25), scaled to 50×50 pt
        // Image CTM: [50, 0, 0, 50, 25, 25]
        let image_obj = ImageObject {
            data: vec![
                255, 0, 0, 255, // pixel (0,0) red
                255, 0, 0, 255, // pixel (1,0) red
                255, 0, 0, 255, // pixel (0,1) red
                255, 0, 0, 255, // pixel (1,1) red
            ],
            width: 2,
            height: 2,
            ctm: Matrix::new(50.0, 0.0, 0.0, 50.0, 25.0, 25.0),
        };
        let page = simple_page(100.0, 100.0, vec![PageObject::Image(Box::new(image_obj))]);
        let bmp = render(&page, 72.0).unwrap();
        // Center of image area should be red
        let center = bmp.pixel_at(50, 50).unwrap();
        assert_eq!(center.r, 255, "image center should be red");
        // Outside image should be white
        assert_eq!(bmp.pixel_at(0, 0), Some(Color::WHITE));
    }
}
