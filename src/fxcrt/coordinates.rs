use std::fmt;
use std::ops::{Add, AddAssign, Mul, Sub, SubAssign};

/// 2D point in floating-point coordinates.
///
/// Corresponds to C++ `CFX_PointF`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

/// 2D size in floating-point dimensions.
///
/// Corresponds to C++ `CFX_SizeF`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

/// Rectangle in PDF coordinate system (y-axis upward).
/// Uses left/bottom/right/top (LBRT) representation.
///
/// Corresponds to C++ `CFX_FloatRect`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    pub left: f32,
    pub bottom: f32,
    pub right: f32,
    pub top: f32,
}

/// Affine transformation matrix.
///
/// Represents the matrix:
/// ```text
/// | a  b  0 |
/// | c  d  0 |
/// | e  f  1 |
/// ```
///
/// Corresponds to C++ `CFX_Matrix`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matrix {
    pub a: f32,
    pub b: f32,
    pub c: f32,
    pub d: f32,
    pub e: f32,
    pub f: f32,
}

// --- Point ---

impl Point {
    pub fn new(x: f32, y: f32) -> Self {
        todo!()
    }
}

impl Add for Point {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        todo!()
    }
}

impl Sub for Point {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        todo!()
    }
}

impl AddAssign for Point {
    fn add_assign(&mut self, rhs: Self) {
        todo!()
    }
}

impl SubAssign for Point {
    fn sub_assign(&mut self, rhs: Self) {
        todo!()
    }
}

impl Mul<f32> for Point {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self {
        todo!()
    }
}

impl Mul<Point> for f32 {
    type Output = Point;
    fn mul(self, rhs: Point) -> Point {
        todo!()
    }
}

// --- Size ---

impl Size {
    pub fn new(width: f32, height: f32) -> Self {
        todo!()
    }
}

impl Add for Size {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        todo!()
    }
}

impl Sub for Size {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        todo!()
    }
}

// --- Rect ---

impl Rect {
    pub fn new(left: f32, bottom: f32, right: f32, top: f32) -> Self {
        todo!()
    }

    pub fn from_points(points: &[Point]) -> Self {
        todo!()
    }

    pub fn width(&self) -> f32 {
        todo!()
    }

    pub fn height(&self) -> f32 {
        todo!()
    }

    pub fn is_empty(&self) -> bool {
        todo!()
    }

    pub fn contains_point(&self, point: Point) -> bool {
        todo!()
    }

    pub fn contains_rect(&self, other: &Rect) -> bool {
        todo!()
    }

    pub fn normalize(&mut self) {
        todo!()
    }

    pub fn intersect(&mut self, other: &Rect) {
        todo!()
    }

    pub fn union(&mut self, other: &Rect) {
        todo!()
    }

    pub fn translate(&mut self, dx: f32, dy: f32) {
        todo!()
    }

    pub fn scale(&mut self, factor: f32) {
        todo!()
    }

    pub fn scale_from_center(&mut self, factor: f32) {
        todo!()
    }

    pub fn inflate(&mut self, x: f32, y: f32) {
        todo!()
    }

    pub fn deflate(&mut self, x: f32, y: f32) -> Rect {
        todo!()
    }
}

impl fmt::Display for Rect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

// --- Matrix ---

impl Default for Matrix {
    fn default() -> Self {
        todo!()
    }
}

impl Matrix {
    pub fn new(a: f32, b: f32, c: f32, d: f32, e: f32, f: f32) -> Self {
        todo!()
    }

    pub fn is_identity(&self) -> bool {
        todo!()
    }

    pub fn inverse(&self) -> Self {
        todo!()
    }

    pub fn translate(&mut self, x: f32, y: f32) {
        todo!()
    }

    pub fn scale(&mut self, sx: f32, sy: f32) {
        todo!()
    }

    pub fn rotate(&mut self, radians: f32) {
        todo!()
    }

    pub fn concat(&mut self, other: &Matrix) {
        todo!()
    }

    pub fn transform_point(&self, point: Point) -> Point {
        todo!()
    }

    pub fn transform_rect(&self, rect: &Rect) -> Rect {
        todo!()
    }
}

impl Mul for Matrix {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        todo!()
    }
}

impl MulAssign for Matrix {
    fn mul_assign(&mut self, rhs: Self) {
        todo!()
    }
}

use std::ops::MulAssign;

#[cfg(test)]
mod tests {
    use super::*;

    // --- Point tests ---

    #[test]
    fn point_default() {
        let p = Point::default();
        assert_eq!(p.x, 0.0);
        assert_eq!(p.y, 0.0);
    }

    #[test]
    fn point_new() {
        let p = Point::new(3.0, 4.0);
        assert_eq!(p.x, 3.0);
        assert_eq!(p.y, 4.0);
    }

    #[test]
    fn point_add() {
        let a = Point::new(1.0, 2.0);
        let b = Point::new(3.0, 4.0);
        let c = a + b;
        assert_eq!(c.x, 4.0);
        assert_eq!(c.y, 6.0);
    }

    #[test]
    fn point_sub() {
        let a = Point::new(5.0, 7.0);
        let b = Point::new(2.0, 3.0);
        let c = a - b;
        assert_eq!(c.x, 3.0);
        assert_eq!(c.y, 4.0);
    }

    #[test]
    fn point_add_assign() {
        let mut a = Point::new(1.0, 2.0);
        a += Point::new(3.0, 4.0);
        assert_eq!(a, Point::new(4.0, 6.0));
    }

    #[test]
    fn point_sub_assign() {
        let mut a = Point::new(5.0, 7.0);
        a -= Point::new(2.0, 3.0);
        assert_eq!(a, Point::new(3.0, 4.0));
    }

    #[test]
    fn point_scalar_mul() {
        let p = Point::new(2.0, 3.0);
        assert_eq!(p * 2.0, Point::new(4.0, 6.0));
        assert_eq!(2.0 * p, Point::new(4.0, 6.0));
    }

    // --- Size tests ---

    #[test]
    fn size_default() {
        let s = Size::default();
        assert_eq!(s.width, 0.0);
        assert_eq!(s.height, 0.0);
    }

    #[test]
    fn size_new() {
        let s = Size::new(10.0, 20.0);
        assert_eq!(s.width, 10.0);
        assert_eq!(s.height, 20.0);
    }

    #[test]
    fn size_add() {
        let a = Size::new(1.0, 2.0);
        let b = Size::new(3.0, 4.0);
        let c = a + b;
        assert_eq!(c, Size::new(4.0, 6.0));
    }

    #[test]
    fn size_sub() {
        let a = Size::new(5.0, 7.0);
        let b = Size::new(2.0, 3.0);
        let c = a - b;
        assert_eq!(c, Size::new(3.0, 4.0));
    }

    // --- Rect tests (ported from CFXFloatRectTest) ---

    #[test]
    fn rect_default_is_zero() {
        let r = Rect::default();
        assert_eq!(r.left, 0.0);
        assert_eq!(r.bottom, 0.0);
        assert_eq!(r.right, 0.0);
        assert_eq!(r.top, 0.0);
    }

    #[test]
    fn rect_new() {
        let r = Rect::new(-1.0, -3.0, 4.5, 3.2);
        assert_eq!(r.left, -1.0);
        assert_eq!(r.bottom, -3.0);
        assert_eq!(r.right, 4.5);
        assert_eq!(r.top, 3.2);
    }

    #[test]
    fn rect_width_height() {
        let r = Rect::new(-1.0, -3.0, 4.5, 3.2);
        assert!((r.width() - 5.5).abs() < 1e-6);
        assert!((r.height() - 6.2).abs() < 1e-6);
    }

    #[test]
    fn rect_is_empty() {
        assert!(Rect::default().is_empty());
        assert!(Rect::new(1.0, 0.0, 1.0, 1.0).is_empty()); // left == right
        assert!(!Rect::new(0.0, 0.0, 1.0, 1.0).is_empty());
    }

    #[test]
    fn rect_contains_point() {
        let r = Rect::new(0.0, 0.0, 10.0, 10.0);
        assert!(r.contains_point(Point::new(5.0, 5.0)));
        assert!(r.contains_point(Point::new(0.0, 0.0)));
        assert!(!r.contains_point(Point::new(-1.0, 5.0)));
        assert!(!r.contains_point(Point::new(5.0, 11.0)));
    }

    #[test]
    fn rect_contains_rect() {
        let outer = Rect::new(0.0, 0.0, 10.0, 10.0);
        let inner = Rect::new(2.0, 2.0, 8.0, 8.0);
        assert!(outer.contains_rect(&inner));
        assert!(!inner.contains_rect(&outer));
    }

    #[test]
    fn rect_normalize() {
        let mut r = Rect::default();
        r.normalize();
        assert_eq!(r, Rect::default());

        let mut r = Rect::new(-1.0, -3.0, 4.5, 3.2);
        r.normalize();
        assert_eq!(r, Rect::new(-1.0, -3.0, 4.5, 3.2));

        // Swap left/right and bottom/top
        let mut r = Rect::new(4.5, 3.2, -1.0, -3.0);
        r.normalize();
        assert_eq!(r.left, -1.0);
        assert_eq!(r.bottom, -3.0);
        assert_eq!(r.right, 4.5);
        assert_eq!(r.top, 3.2);
    }

    #[test]
    fn rect_intersect() {
        let mut a = Rect::new(0.0, 0.0, 10.0, 10.0);
        let b = Rect::new(5.0, 5.0, 15.0, 15.0);
        a.intersect(&b);
        assert_eq!(a, Rect::new(5.0, 5.0, 10.0, 10.0));
    }

    #[test]
    fn rect_union() {
        let mut a = Rect::new(0.0, 0.0, 5.0, 5.0);
        let b = Rect::new(3.0, 3.0, 10.0, 10.0);
        a.union(&b);
        assert_eq!(a, Rect::new(0.0, 0.0, 10.0, 10.0));
    }

    #[test]
    fn rect_translate() {
        let mut r = Rect::new(0.0, 0.0, 10.0, 10.0);
        r.translate(5.0, 3.0);
        assert_eq!(r, Rect::new(5.0, 3.0, 15.0, 13.0));
    }

    #[test]
    fn rect_scale() {
        let mut r = Rect::new(-1.0, -3.0, 4.5, 3.2);
        r.scale(1.0);
        assert_eq!(r.left, -1.0);
        assert_eq!(r.bottom, -3.0);
        assert_eq!(r.right, 4.5);
        assert_eq!(r.top, 3.2);

        r.scale(0.5);
        assert!((r.left - (-0.5)).abs() < 1e-6);
        assert!((r.bottom - (-1.5)).abs() < 1e-6);
        assert!((r.right - 2.25).abs() < 1e-6);
        assert!((r.top - 1.6).abs() < 1e-6);

        r.scale(2.0);
        assert!((r.left - (-1.0)).abs() < 1e-6);
        assert!((r.bottom - (-3.0)).abs() < 1e-6);
        assert!((r.right - 4.5).abs() < 1e-6);
        assert!((r.top - 3.2).abs() < 1e-6);

        r.scale(-1.0);
        assert!((r.left - 1.0).abs() < 1e-6);
        assert!((r.bottom - 3.0).abs() < 1e-6);
        assert!((r.right - (-4.5)).abs() < 1e-6);
        assert!((r.top - (-3.2)).abs() < 1e-6);

        r.scale(0.0);
        assert_eq!(r.left, 0.0);
        assert_eq!(r.bottom, 0.0);
        assert_eq!(r.right, 0.0);
        assert_eq!(r.top, 0.0);
    }

    #[test]
    fn rect_scale_empty() {
        let mut r = Rect::default();
        r.scale(1.0);
        assert_eq!(r, Rect::default());
        r.scale(0.5);
        assert_eq!(r, Rect::default());
        r.scale(2.0);
        assert_eq!(r, Rect::default());
        r.scale(0.0);
        assert_eq!(r, Rect::default());
    }

    #[test]
    fn rect_scale_from_center() {
        let mut r = Rect::new(-1.0, -3.0, 4.5, 3.2);
        r.scale_from_center(1.0);
        assert_eq!(r.left, -1.0);
        assert_eq!(r.bottom, -3.0);
        assert_eq!(r.right, 4.5);
        assert_eq!(r.top, 3.2);

        r.scale_from_center(0.5);
        assert!((r.left - 0.375).abs() < 1e-6);
        assert!((r.bottom - (-1.45)).abs() < 1e-6);
        assert!((r.right - 3.125).abs() < 1e-6);
        assert!((r.top - 1.65).abs() < 1e-6);

        r.scale_from_center(2.0);
        assert!((r.left - (-1.0)).abs() < 1e-6);
        assert!((r.bottom - (-3.0)).abs() < 1e-6);
        assert!((r.right - 4.5).abs() < 1e-6);
        assert!((r.top - 3.2).abs() < 1e-6);

        r.scale_from_center(-1.0);
        assert!((r.left - 4.5).abs() < 1e-6);
        assert!((r.bottom - 3.2).abs() < 1e-6);
        assert!((r.right - (-1.0)).abs() < 1e-6);
        assert!((r.top - (-3.0)).abs() < 1e-6);

        r.scale_from_center(0.0);
        assert!((r.left - 1.75).abs() < 1e-6);
        assert!((r.bottom - 0.1).abs() < 1e-3);
        assert!((r.right - 1.75).abs() < 1e-6);
        assert!((r.top - 0.1).abs() < 1e-3);
    }

    #[test]
    fn rect_scale_from_center_empty() {
        let mut r = Rect::default();
        r.scale_from_center(1.0);
        assert_eq!(r, Rect::default());
        r.scale_from_center(0.5);
        assert_eq!(r, Rect::default());
        r.scale_from_center(2.0);
        assert_eq!(r, Rect::default());
        r.scale_from_center(0.0);
        assert_eq!(r, Rect::default());
    }

    #[test]
    fn rect_get_bbox_empty() {
        let r = Rect::from_points(&[]);
        assert_eq!(r, Rect::default());
    }

    #[test]
    fn rect_get_bbox_single_point() {
        let r = Rect::from_points(&[Point::new(0.0, 0.0)]);
        assert_eq!(r, Rect::new(0.0, 0.0, 0.0, 0.0));
    }

    #[test]
    fn rect_get_bbox_multiple_points() {
        let points = vec![
            Point::new(0.0, 0.0),
            Point::new(2.5, 6.2),
            Point::new(1.5, 6.2),
        ];
        let r = Rect::from_points(&points[..2]);
        assert_eq!(r.left, 0.0);
        assert_eq!(r.bottom, 0.0);
        assert_eq!(r.right, 2.5);
        assert_eq!(r.top, 6.2);

        let r = Rect::from_points(&points);
        assert_eq!(r.left, 0.0);
        assert_eq!(r.bottom, 0.0);
        assert_eq!(r.right, 2.5);
        assert_eq!(r.top, 6.2);
    }

    #[test]
    fn rect_get_bbox_with_negatives() {
        let points = vec![
            Point::new(0.0, 0.0),
            Point::new(2.5, 6.3),
            Point::new(-3.0, 6.3),
            Point::new(4.0, -8.0),
        ];
        let r = Rect::from_points(&points);
        assert_eq!(r.left, -3.0);
        assert_eq!(r.bottom, -8.0);
        assert_eq!(r.right, 4.0);
        assert_eq!(r.top, 6.3);
    }

    #[test]
    fn rect_inflate() {
        let mut r = Rect::new(0.0, 0.0, 10.0, 10.0);
        r.inflate(2.0, 3.0);
        assert_eq!(r, Rect::new(-2.0, -3.0, 12.0, 13.0));
    }

    #[test]
    fn rect_display() {
        let r = Rect::default();
        assert_eq!(format!("{r}"), "rect[w 0 x h 0 (left 0, bot 0)]");

        let r = Rect::new(10.0, 20.0, 14.0, 23.0);
        assert_eq!(format!("{r}"), "rect[w 4 x h 3 (left 10, bot 20)]");

        let r = Rect::new(10.5, 20.5, 14.75, 23.75);
        assert_eq!(
            format!("{r}"),
            "rect[w 4.25 x h 3.25 (left 10.5, bot 20.5)]"
        );
    }

    // --- Matrix tests (ported from CFXMatrixTest) ---

    #[test]
    fn matrix_default_is_identity() {
        let m = Matrix::default();
        assert_eq!(m.a, 1.0);
        assert_eq!(m.b, 0.0);
        assert_eq!(m.c, 0.0);
        assert_eq!(m.d, 1.0);
        assert_eq!(m.e, 0.0);
        assert_eq!(m.f, 0.0);
        assert!(m.is_identity());
    }

    #[test]
    fn matrix_is_identity_false() {
        let mut m = Matrix::default();
        m.a = -1.0;
        assert!(!m.is_identity());
    }

    #[test]
    fn matrix_inverse_identity() {
        let m = Matrix::default();
        let inv = m.inverse();
        assert_eq!(inv.a, 1.0);
        assert_eq!(inv.b, 0.0);
        assert_eq!(inv.c, 0.0);
        assert_eq!(inv.d, 1.0);
        assert_eq!(inv.e, 0.0);
        assert_eq!(inv.f, 0.0);

        let p = Point::new(2.0, 3.0);
        let result = inv.transform_point(m.transform_point(p));
        assert!((result.x - 2.0).abs() < 1e-5);
        assert!((result.y - 3.0).abs() < 1e-5);
    }

    #[test]
    fn matrix_inverse() {
        let m = Matrix::new(3.0, 0.0, 2.0, 3.0, 1.0, 4.0);
        let inv = m.inverse();
        assert!((inv.a - 0.33333334).abs() < 1e-6);
        assert_eq!(inv.b, 0.0);
        assert!((inv.c - (-0.22222222)).abs() < 1e-6);
        assert!((inv.d - 0.33333334).abs() < 1e-6);
        assert!((inv.e - 0.55555556).abs() < 1e-6);
        assert!((inv.f - (-1.3333334)).abs() < 1e-5);

        let p = Point::new(2.0, 3.0);
        let result = inv.transform_point(m.transform_point(p));
        assert!((result.x - 2.0).abs() < 1e-5);
        assert!((result.y - 3.0).abs() < 1e-5);
    }

    #[test]
    fn matrix_compose_transformations() {
        use std::f32::consts::FRAC_PI_2;

        let mut rotate_90 = Matrix::default();
        rotate_90.rotate(FRAC_PI_2);
        assert!((rotate_90.a).abs() < 1e-5);
        assert!((rotate_90.b - 1.0).abs() < 1e-5);
        assert!((rotate_90.c - (-1.0)).abs() < 1e-5);
        assert!((rotate_90.d).abs() < 1e-5);

        let mut translate_23_11 = Matrix::default();
        translate_23_11.translate(23.0, 11.0);
        assert_eq!(translate_23_11.a, 1.0);
        assert_eq!(translate_23_11.b, 0.0);
        assert_eq!(translate_23_11.c, 0.0);
        assert_eq!(translate_23_11.d, 1.0);
        assert_eq!(translate_23_11.e, 23.0);
        assert_eq!(translate_23_11.f, 11.0);

        let mut scale_5_13 = Matrix::default();
        scale_5_13.scale(5.0, 13.0);
        assert_eq!(scale_5_13.a, 5.0);
        assert_eq!(scale_5_13.b, 0.0);
        assert_eq!(scale_5_13.c, 0.0);
        assert_eq!(scale_5_13.d, 13.0);

        // Step-by-step transform: rotate, translate, scale
        let p = Point::new(10.0, 20.0);
        let p1 = rotate_90.transform_point(p);
        assert!((p1.x - (-20.0)).abs() < 1e-4);
        assert!((p1.y - 10.0).abs() < 1e-4);

        let p2 = translate_23_11.transform_point(p1);
        assert!((p2.x - 3.0).abs() < 1e-4);
        assert!((p2.y - 21.0).abs() < 1e-4);

        let p3 = scale_5_13.transform_point(p2);
        assert!((p3.x - 15.0).abs() < 1e-4);
        assert!((p3.y - 273.0).abs() < 1e-4);

        // Compose: rotate then translate then scale
        let mut m = Matrix::default();
        m.concat(&rotate_90);
        m.concat(&translate_23_11);
        m.concat(&scale_5_13);
        assert!((m.a).abs() < 1e-5);
        assert!((m.b - 13.0).abs() < 1e-5);
        assert!((m.c - (-5.0)).abs() < 1e-5);
        assert!((m.d).abs() < 1e-5);
        assert_eq!(m.e, 115.0);
        assert_eq!(m.f, 143.0);

        let origin = m.transform_point(Point::new(0.0, 0.0));
        assert!((origin.x - 115.0).abs() < 1e-4);
        assert!((origin.y - 143.0).abs() < 1e-4);

        let result = m.transform_point(Point::new(10.0, 20.0));
        assert!((result.x - 15.0).abs() < 1e-4);
        assert!((result.y - 273.0).abs() < 1e-4);
    }

    #[test]
    fn matrix_mul_operator() {
        let a = Matrix::new(3.0, 0.0, 2.0, 3.0, 1.0, 4.0);
        let b = Matrix::new(1.0, 2.0, 0.0, 1.0, 5.0, 3.0);
        let c = a * b;
        // Verify multiplication result matches concat behavior
        let mut d = a;
        d.concat(&b);
        assert_eq!(c, d);
    }

    #[test]
    fn matrix_transform_point() {
        let m = Matrix::new(2.0, 0.0, 0.0, 3.0, 10.0, 20.0);
        let p = m.transform_point(Point::new(1.0, 1.0));
        assert!((p.x - 12.0).abs() < 1e-6);
        assert!((p.y - 23.0).abs() < 1e-6);
    }

    #[test]
    fn matrix_transform_rect() {
        use std::f32::consts::FRAC_PI_2;

        let mut rotate_90 = Matrix::default();
        rotate_90.rotate(FRAC_PI_2);

        let rect = Rect::new(5.5, 0.0, 12.25, 2.7);
        let result = rotate_90.transform_rect(&rect);
        assert!((result.left - (-2.7)).abs() < 1e-4);
        assert!((result.bottom - 5.5).abs() < 1e-4);
        assert!(result.right.abs() < 1e-4);
        assert!((result.top - 12.25).abs() < 1e-4);

        let mut scale_5_13 = Matrix::default();
        scale_5_13.scale(5.0, 13.0);
        let result2 = scale_5_13.transform_rect(&result);
        assert!((result2.left - (-13.5)).abs() < 1e-4);
        assert!((result2.bottom - 71.5).abs() < 1e-4);
        assert!(result2.right.abs() < 1e-4);
        assert!((result2.top - 159.25).abs() < 1e-4);
    }
}
