use crate::fpdfapi::page::page_object::PathObject;

/// Render a `PathObject` (fill and/or stroke) onto the pixmap.
#[allow(dead_code)]
pub fn render_path(
    _pixmap: &mut tiny_skia::Pixmap,
    _path_obj: &PathObject,
    _page_to_device: tiny_skia::Transform,
) {
    todo!()
}
