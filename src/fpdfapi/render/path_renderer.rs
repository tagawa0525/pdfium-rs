use crate::fpdfapi::page::page_object::{FillRule, PathObject};
use crate::fxge::color::Color;
use crate::fxge::path::PathPointKind;

/// Render a `PathObject` (fill and/or stroke) onto the pixmap.
pub fn render_path(
    pixmap: &mut tiny_skia::Pixmap,
    path_obj: &PathObject,
    page_to_device: tiny_skia::Transform,
) {
    // Combine the object's CTM with the page-to-device transform.
    let obj_transform = matrix_to_transform(&path_obj.ctm);
    let transform = page_to_device.pre_concat(obj_transform);

    let Some(path) = build_tiny_skia_path(&path_obj.path) else {
        return;
    };

    // Fill
    if path_obj.fill_rule != FillRule::None {
        let fill_rule = match path_obj.fill_rule {
            FillRule::EvenOdd => tiny_skia::FillRule::EvenOdd,
            _ => tiny_skia::FillRule::Winding,
        };
        let mut paint = tiny_skia::Paint::default();
        paint.set_color(color_to_tiny_skia(path_obj.fill_color));
        paint.anti_alias = true;
        pixmap.fill_path(&path, &paint, fill_rule, transform, None);
    }

    // Stroke
    if path_obj.stroke {
        let mut paint = tiny_skia::Paint::default();
        paint.set_color(color_to_tiny_skia(path_obj.stroke_color));
        paint.anti_alias = true;

        let line_cap = match path_obj.line_cap {
            crate::fxge::color::LineCap::Butt => tiny_skia::LineCap::Butt,
            crate::fxge::color::LineCap::Round => tiny_skia::LineCap::Round,
            crate::fxge::color::LineCap::Square => tiny_skia::LineCap::Square,
        };
        let line_join = match path_obj.line_join {
            crate::fxge::color::LineJoin::Miter => tiny_skia::LineJoin::Miter,
            crate::fxge::color::LineJoin::Round => tiny_skia::LineJoin::Round,
            crate::fxge::color::LineJoin::Bevel => tiny_skia::LineJoin::Bevel,
        };

        let mut stroke = tiny_skia::Stroke {
            width: path_obj.line_width,
            line_cap,
            line_join,
            miter_limit: path_obj.miter_limit,
            ..Default::default()
        };

        if !path_obj.dash_array.is_empty()
            && let Some(dash) =
                tiny_skia::StrokeDash::new(path_obj.dash_array.clone(), path_obj.dash_phase)
        {
            stroke.dash = Some(dash);
        }

        pixmap.stroke_path(&path, &paint, &stroke, transform, None);
    }
}

/// Convert our `Path` to a `tiny_skia::Path`.
fn build_tiny_skia_path(path: &crate::fxge::path::Path) -> Option<tiny_skia::Path> {
    let mut pb = tiny_skia::PathBuilder::new();
    let points = &path.points;
    let mut i = 0;

    while i < points.len() {
        let pp = &points[i];
        match pp.kind {
            PathPointKind::Move => {
                pb.move_to(pp.point.x, pp.point.y);
                if pp.close {
                    pb.close();
                }
                i += 1;
            }
            PathPointKind::Line => {
                pb.line_to(pp.point.x, pp.point.y);
                if pp.close {
                    pb.close();
                }
                i += 1;
            }
            PathPointKind::BezierTo => {
                // Three consecutive BezierTo points: ctrl1, ctrl2, end
                if i + 2 < points.len() {
                    let ctrl1 = &points[i];
                    let ctrl2 = &points[i + 1];
                    let end = &points[i + 2];
                    pb.cubic_to(
                        ctrl1.point.x,
                        ctrl1.point.y,
                        ctrl2.point.x,
                        ctrl2.point.y,
                        end.point.x,
                        end.point.y,
                    );
                    if end.close {
                        pb.close();
                    }
                    i += 3;
                } else {
                    i += 1; // malformed, skip
                }
            }
        }
    }

    pb.finish()
}

fn color_to_tiny_skia(c: Color) -> tiny_skia::Color {
    tiny_skia::Color::from_rgba8(c.r, c.g, c.b, c.a)
}

fn matrix_to_transform(m: &crate::fxcrt::coordinates::Matrix) -> tiny_skia::Transform {
    tiny_skia::Transform::from_row(m.a, m.b, m.c, m.d, m.e, m.f)
}
