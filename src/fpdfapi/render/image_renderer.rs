use crate::fpdfapi::page::page_object::ImageObject;

/// Render an `ImageObject` onto the pixmap using its CTM for placement.
///
/// The image CTM maps from image space `(0,0)-(1,1)` to PDF user space.
/// Combined with `page_to_device`, this places the image at the correct
/// position and size in the output bitmap.
pub fn render_image(
    pixmap: &mut tiny_skia::Pixmap,
    image_obj: &ImageObject,
    page_to_device: tiny_skia::Transform,
) {
    // Validate dimensions and data length before any allocation.
    let expected_len = (image_obj.width as usize)
        .checked_mul(image_obj.height as usize)
        .and_then(|n| n.checked_mul(4))
        .unwrap_or(0);
    if expected_len == 0 || image_obj.data.len() != expected_len {
        return;
    }

    // Build a tiny_skia::PixmapRef from the RGBA image data (straight alpha).
    // tiny-skia expects premultiplied alpha, so convert.
    let mut premul_data = image_obj.data.clone();
    for pixel in premul_data.chunks_exact_mut(4) {
        let a = pixel[3] as u16;
        pixel[0] = ((pixel[0] as u16 * a + 128) / 255) as u8;
        pixel[1] = ((pixel[1] as u16 * a + 128) / 255) as u8;
        pixel[2] = ((pixel[2] as u16 * a + 128) / 255) as u8;
    }

    let Some(img_pixmap) =
        tiny_skia::PixmapRef::from_bytes(&premul_data, image_obj.width, image_obj.height)
    else {
        return;
    };

    // The image CTM maps unit square (0,0)-(1,1) to PDF space.
    // We need to scale from pixel dimensions to unit square first:
    //   pixel_to_unit = scale(1/width, 1/height)
    // Then apply the image CTM, then the page-to-device transform.
    let image_ctm = tiny_skia::Transform::from_row(
        image_obj.ctm.a,
        image_obj.ctm.b,
        image_obj.ctm.c,
        image_obj.ctm.d,
        image_obj.ctm.e,
        image_obj.ctm.f,
    );

    // Combined transform: pixel → unit → PDF → device
    let pixel_to_unit = tiny_skia::Transform::from_scale(
        1.0 / image_obj.width as f32,
        1.0 / image_obj.height as f32,
    );
    let transform = page_to_device
        .pre_concat(image_ctm)
        .pre_concat(pixel_to_unit);

    let paint = tiny_skia::PixmapPaint {
        quality: tiny_skia::FilterQuality::Bilinear,
        ..Default::default()
    };

    pixmap.draw_pixmap(0, 0, img_pixmap, &paint, transform, None);
}
