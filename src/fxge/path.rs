use crate::fxcrt::coordinates::{Matrix, Point, Rect};

/// Classification of a path point.
///
/// For cubic Bézier curves (`cubic_to`), three consecutive `BezierTo` points are added:
/// the two control points and the end point. Renderers identify the endpoint as the
/// third `BezierTo` in each group of three.
///
/// This convention matches C++ pdfium's `FXPT_TYPE::BezierTo`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathPointKind {
    Move,
    Line,
    /// Point in a cubic Bézier group.
    ///
    /// Each curve consists of three consecutive `BezierTo` points:
    /// control point 1, control point 2, and the curve endpoint.
    BezierTo,
}

/// A single point in a path with its role and optional close flag.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PathPoint {
    pub point: Point,
    pub kind: PathPointKind,
    /// If true, a close sub-path command follows this point.
    pub close: bool,
}

/// A sequence of path points forming one or more sub-paths.
///
/// Corresponds to C++ `CFX_Path`.
#[derive(Debug, Clone, Default)]
pub struct Path {
    pub points: Vec<PathPoint>,
}

impl Path {
    pub fn new() -> Self {
        Path { points: Vec::new() }
    }

    pub fn move_to(&mut self, p: Point) {
        self.points.push(PathPoint {
            point: p,
            kind: PathPointKind::Move,
            close: false,
        });
    }

    pub fn line_to(&mut self, p: Point) {
        self.points.push(PathPoint {
            point: p,
            kind: PathPointKind::Line,
            close: false,
        });
    }

    /// Append a cubic Bézier curve.
    ///
    /// Adds three `BezierTo` points: ctrl1, ctrl2, end (curve endpoint).
    pub fn cubic_to(&mut self, ctrl1: Point, ctrl2: Point, end: Point) {
        for p in [ctrl1, ctrl2, end] {
            self.points.push(PathPoint {
                point: p,
                kind: PathPointKind::BezierTo,
                close: false,
            });
        }
    }

    /// Close the current sub-path by marking the last point with `close = true`.
    pub fn close(&mut self) {
        if let Some(last) = self.points.last_mut() {
            last.close = true;
        }
    }

    /// Append a rectangle as a closed sub-path (move + 3 lines + close).
    pub fn append_rect(&mut self, x: f32, y: f32, w: f32, h: f32) {
        self.move_to(Point::new(x, y));
        self.line_to(Point::new(x + w, y));
        self.line_to(Point::new(x + w, y + h));
        self.line_to(Point::new(x, y + h));
        self.close();
    }

    /// Transform all points by the given matrix.
    pub fn transform(&mut self, m: &Matrix) {
        for pp in &mut self.points {
            pp.point = m.transform_point(pp.point);
        }
    }

    /// Compute the axis-aligned bounding box of all points.
    ///
    /// Returns `Rect::default()` for an empty path.
    pub fn bounding_box(&self) -> Rect {
        if self.points.is_empty() {
            return Rect::default();
        }
        let pts: Vec<Point> = self.points.iter().map(|pp| pp.point).collect();
        Rect::from_points(&pts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_new_is_empty() {
        let p = Path::new();
        assert!(p.points.is_empty());
    }

    #[test]
    fn path_move_to_line_to() {
        let mut p = Path::new();
        p.move_to(Point::new(0.0, 0.0));
        p.line_to(Point::new(10.0, 10.0));
        assert_eq!(p.points.len(), 2);
        assert_eq!(p.points[0].kind, PathPointKind::Move);
        assert_eq!(p.points[1].kind, PathPointKind::Line);
    }

    #[test]
    fn path_append_rect_bounding_box() {
        let mut p = Path::new();
        p.append_rect(10.0, 20.0, 100.0, 50.0);
        let bb = p.bounding_box();
        assert!((bb.left - 10.0).abs() < 1e-5);
        assert!((bb.bottom - 20.0).abs() < 1e-5);
        assert!((bb.right - 110.0).abs() < 1e-5);
        assert!((bb.top - 70.0).abs() < 1e-5);
    }

    #[test]
    fn path_bounding_box_empty() {
        let p = Path::new();
        let bb = p.bounding_box();
        assert_eq!(bb, Rect::default());
    }

    #[test]
    fn path_transform_translates_points() {
        let mut p = Path::new();
        p.move_to(Point::new(0.0, 0.0));
        p.line_to(Point::new(10.0, 0.0));

        let mut m = Matrix::default();
        m.translate(5.0, 3.0);
        p.transform(&m);

        assert!((p.points[0].point.x - 5.0).abs() < 1e-5);
        assert!((p.points[0].point.y - 3.0).abs() < 1e-5);
        assert!((p.points[1].point.x - 15.0).abs() < 1e-5);
        assert!((p.points[1].point.y - 3.0).abs() < 1e-5);
    }

    #[test]
    fn path_cubic_to_adds_three_points() {
        let mut p = Path::new();
        p.move_to(Point::new(0.0, 0.0));
        p.cubic_to(
            Point::new(1.0, 2.0),
            Point::new(3.0, 4.0),
            Point::new(5.0, 0.0),
        );
        // move + 3 bezier control points
        assert_eq!(p.points.len(), 4);
        assert_eq!(p.points[1].kind, PathPointKind::BezierTo);
        assert_eq!(p.points[2].kind, PathPointKind::BezierTo);
        assert_eq!(p.points[3].kind, PathPointKind::BezierTo);
    }
}
