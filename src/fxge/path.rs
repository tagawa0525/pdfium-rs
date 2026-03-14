use crate::fxcrt::coordinates::{Matrix, Point, Rect};

/// Classification of a path point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathPointKind {
    Move,
    Line,
    /// First control point of a cubic Bézier segment.
    BezierControl,
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
        todo!()
    }

    pub fn move_to(&mut self, p: Point) {
        todo!()
    }

    pub fn line_to(&mut self, p: Point) {
        todo!()
    }

    /// Append a cubic Bézier curve with two control points and an end point.
    pub fn cubic_to(&mut self, ctrl1: Point, ctrl2: Point, end: Point) {
        todo!()
    }

    /// Close the current sub-path.
    pub fn close(&mut self) {
        todo!()
    }

    /// Append a rectangle as a closed sub-path.
    pub fn append_rect(&mut self, x: f32, y: f32, w: f32, h: f32) {
        todo!()
    }

    /// Transform all points by the given matrix.
    pub fn transform(&mut self, m: &Matrix) {
        todo!()
    }

    /// Compute the axis-aligned bounding box of all points.
    pub fn bounding_box(&self) -> Rect {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "not yet implemented"]
    fn path_new_is_empty() {
        let p = Path::new();
        assert!(p.points.is_empty());
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn path_move_to_line_to() {
        let mut p = Path::new();
        p.move_to(Point::new(0.0, 0.0));
        p.line_to(Point::new(10.0, 10.0));
        assert_eq!(p.points.len(), 2);
        assert_eq!(p.points[0].kind, PathPointKind::Move);
        assert_eq!(p.points[1].kind, PathPointKind::Line);
    }

    #[test]
    #[ignore = "not yet implemented"]
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
    #[ignore = "not yet implemented"]
    fn path_bounding_box_empty() {
        let p = Path::new();
        let bb = p.bounding_box();
        assert_eq!(bb, Rect::default());
    }

    #[test]
    #[ignore = "not yet implemented"]
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
    #[ignore = "not yet implemented"]
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
        assert_eq!(p.points[1].kind, PathPointKind::BezierControl);
        assert_eq!(p.points[2].kind, PathPointKind::BezierControl);
        assert_eq!(p.points[3].kind, PathPointKind::BezierControl);
    }
}
