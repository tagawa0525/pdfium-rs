use crate::fpdfapi::parser::object::PdfObject;

/// Zoom mode for a PDF destination.
///
/// Corresponds to C++ `CPDF_Dest` zoom mode parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoomMode {
    Unknown,
    XYZ,
    Fit,
    FitH,
    FitV,
    FitR,
    FitB,
    FitBH,
    FitBV,
}

/// A resolved PDF destination (page + zoom).
///
/// Corresponds to C++ `CPDF_Dest`.
#[derive(Debug, Clone, PartialEq)]
pub struct Dest {
    pub page_index: Option<u32>,
    pub zoom_mode: ZoomMode,
    pub params: Vec<f32>,
}

impl Dest {
    /// Parse a destination from a PDF array (excluding the page reference element).
    ///
    /// `arr` should be the elements after the page reference (i.e., the zoom mode name
    /// followed by numeric parameters). `page_index` is the resolved 0-based page index.
    pub fn from_array(arr: &[PdfObject], page_index: Option<u32>) -> Self {
        let _ = (arr, page_index);
        todo!()
    }

    /// Extract XYZ parameters: (left, top, zoom).
    /// Returns `None` if the zoom mode is not XYZ.
    pub fn xyz(&self) -> Option<(Option<f32>, Option<f32>, Option<f32>)> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fxcrt::bytestring::PdfByteString;

    fn name(s: &str) -> PdfObject {
        PdfObject::Name(PdfByteString::from(s))
    }

    fn real(v: f64) -> PdfObject {
        PdfObject::Real(v)
    }

    fn integer(v: i32) -> PdfObject {
        PdfObject::Integer(v)
    }

    fn null() -> PdfObject {
        PdfObject::Null
    }

    // --- ZoomMode parsing ---

    #[test]
    #[ignore = "not yet implemented"]
    fn xyz_zoom_mode() {
        let arr = [name("XYZ"), real(100.0), real(200.0), real(1.5)];
        let dest = Dest::from_array(&arr, Some(0));
        assert_eq!(dest.zoom_mode, ZoomMode::XYZ);
        assert_eq!(dest.page_index, Some(0));
        assert_eq!(dest.params, vec![100.0f32, 200.0, 1.5]);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn fit_zoom_mode() {
        let arr = [name("Fit")];
        let dest = Dest::from_array(&arr, Some(2));
        assert_eq!(dest.zoom_mode, ZoomMode::Fit);
        assert!(dest.params.is_empty());
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn fit_h_zoom_mode() {
        let arr = [name("FitH"), real(300.0)];
        let dest = Dest::from_array(&arr, Some(1));
        assert_eq!(dest.zoom_mode, ZoomMode::FitH);
        assert_eq!(dest.params, vec![300.0f32]);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn fit_v_zoom_mode() {
        let arr = [name("FitV"), real(50.0)];
        let dest = Dest::from_array(&arr, None);
        assert_eq!(dest.zoom_mode, ZoomMode::FitV);
        assert_eq!(dest.params, vec![50.0f32]);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn fit_r_zoom_mode() {
        let arr = [
            name("FitR"),
            real(10.0),
            real(20.0),
            real(300.0),
            real(400.0),
        ];
        let dest = Dest::from_array(&arr, Some(0));
        assert_eq!(dest.zoom_mode, ZoomMode::FitR);
        assert_eq!(dest.params, vec![10.0f32, 20.0, 300.0, 400.0]);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn fit_b_zoom_mode() {
        let arr = [name("FitB")];
        let dest = Dest::from_array(&arr, Some(0));
        assert_eq!(dest.zoom_mode, ZoomMode::FitB);
        assert!(dest.params.is_empty());
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn fit_bh_zoom_mode() {
        let arr = [name("FitBH"), real(500.0)];
        let dest = Dest::from_array(&arr, Some(0));
        assert_eq!(dest.zoom_mode, ZoomMode::FitBH);
        assert_eq!(dest.params, vec![500.0f32]);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn fit_bv_zoom_mode() {
        let arr = [name("FitBV"), integer(72)];
        let dest = Dest::from_array(&arr, Some(0));
        assert_eq!(dest.zoom_mode, ZoomMode::FitBV);
        assert_eq!(dest.params, vec![72.0f32]);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn unknown_zoom_mode() {
        let arr = [name("Invalid")];
        let dest = Dest::from_array(&arr, Some(0));
        assert_eq!(dest.zoom_mode, ZoomMode::Unknown);
    }

    // --- Null parameters ---

    #[test]
    #[ignore = "not yet implemented"]
    fn xyz_with_null_params() {
        let arr = [name("XYZ"), null(), null(), null()];
        let dest = Dest::from_array(&arr, Some(0));
        assert_eq!(dest.zoom_mode, ZoomMode::XYZ);
        assert_eq!(dest.params.len(), 3);
    }

    // --- Empty / malformed arrays ---

    #[test]
    #[ignore = "not yet implemented"]
    fn empty_array() {
        let arr: [PdfObject; 0] = [];
        let dest = Dest::from_array(&arr, Some(0));
        assert_eq!(dest.zoom_mode, ZoomMode::Unknown);
    }

    // --- xyz() accessor ---

    #[test]
    #[ignore = "not yet implemented"]
    fn xyz_accessor_returns_values() {
        let arr = [name("XYZ"), real(10.0), real(20.0), real(2.0)];
        let dest = Dest::from_array(&arr, Some(0));
        let (left, top, zoom) = dest.xyz().unwrap();
        assert_eq!(left, Some(10.0f32));
        assert_eq!(top, Some(20.0f32));
        assert_eq!(zoom, Some(2.0f32));
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn xyz_accessor_with_null_values() {
        let arr = [name("XYZ"), null(), real(20.0), null()];
        let dest = Dest::from_array(&arr, Some(0));
        let (left, top, zoom) = dest.xyz().unwrap();
        assert_eq!(left, None);
        assert_eq!(top, Some(20.0f32));
        assert_eq!(zoom, None);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn xyz_accessor_returns_none_for_non_xyz() {
        let arr = [name("Fit")];
        let dest = Dest::from_array(&arr, Some(0));
        assert!(dest.xyz().is_none());
    }

    // --- Integer parameters ---

    #[test]
    #[ignore = "not yet implemented"]
    fn integer_params_converted_to_f32() {
        let arr = [name("XYZ"), integer(100), integer(200), integer(1)];
        let dest = Dest::from_array(&arr, Some(0));
        assert_eq!(dest.params, vec![100.0f32, 200.0, 1.0]);
    }
}
