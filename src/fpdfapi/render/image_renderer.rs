use crate::fpdfapi::page::page_object::ImageObject;

/// Render an `ImageObject` onto the pixmap using its CTM for placement.
#[allow(dead_code)]
pub fn render_image(
    _pixmap: &mut tiny_skia::Pixmap,
    _image_obj: &ImageObject,
    _page_to_device: tiny_skia::Transform,
) {
    todo!()
}
