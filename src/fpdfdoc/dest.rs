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
///
/// `params` preserves PDF `null` as `None` so callers can distinguish "omitted"
/// from an explicit `0` coordinate — e.g., an XYZ zoom of `0` means "keep current".
#[derive(Debug, Clone, PartialEq)]
pub struct Dest {
    pub page_index: Option<u32>,
    pub zoom_mode: ZoomMode,
    pub params: Vec<Option<f32>>,
}

impl Dest {
    /// Parse a destination from a PDF array (excluding the page reference element).
    ///
    /// `arr` should be the elements after the page reference (i.e., the zoom mode name
    /// followed by numeric parameters). `page_index` is the resolved 0-based page index.
    pub fn from_array(arr: &[PdfObject], page_index: Option<u32>) -> Self {
        let zoom_mode = arr
            .first()
            .and_then(|obj| obj.as_name())
            .map(|name| match name.as_bytes() {
                b"XYZ" => ZoomMode::XYZ,
                b"Fit" => ZoomMode::Fit,
                b"FitH" => ZoomMode::FitH,
                b"FitV" => ZoomMode::FitV,
                b"FitR" => ZoomMode::FitR,
                b"FitB" => ZoomMode::FitB,
                b"FitBH" => ZoomMode::FitBH,
                b"FitBV" => ZoomMode::FitBV,
                _ => ZoomMode::Unknown,
            })
            .unwrap_or(ZoomMode::Unknown);

        let params: Vec<Option<f32>> = arr
            .iter()
            .skip(1)
            .map(|obj| obj.as_f64().map(|v| v as f32))
            .collect();

        Dest {
            page_index,
            zoom_mode,
            params,
        }
    }

    /// Extract XYZ parameters: (left, top, zoom).
    /// Returns `None` if the zoom mode is not XYZ.
    pub fn xyz(&self) -> Option<(Option<f32>, Option<f32>, Option<f32>)> {
        if self.zoom_mode != ZoomMode::XYZ {
            return None;
        }
        Some((
            self.params.first().copied().flatten(),
            self.params.get(1).copied().flatten(),
            self.params.get(2).copied().flatten(),
        ))
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

    fn xyz_zoom_mode() {
        let arr = [name("XYZ"), real(100.0), real(200.0), real(1.5)];
        let dest = Dest::from_array(&arr, Some(0));
        assert_eq!(dest.zoom_mode, ZoomMode::XYZ);
        assert_eq!(dest.page_index, Some(0));
        assert_eq!(dest.params, vec![Some(100.0f32), Some(200.0), Some(1.5)]);
    }

    #[test]

    fn fit_zoom_mode() {
        let arr = [name("Fit")];
        let dest = Dest::from_array(&arr, Some(2));
        assert_eq!(dest.zoom_mode, ZoomMode::Fit);
        assert!(dest.params.is_empty());
    }

    #[test]

    fn fit_h_zoom_mode() {
        let arr = [name("FitH"), real(300.0)];
        let dest = Dest::from_array(&arr, Some(1));
        assert_eq!(dest.zoom_mode, ZoomMode::FitH);
        assert_eq!(dest.params, vec![Some(300.0f32)]);
    }

    #[test]

    fn fit_v_zoom_mode() {
        let arr = [name("FitV"), real(50.0)];
        let dest = Dest::from_array(&arr, None);
        assert_eq!(dest.zoom_mode, ZoomMode::FitV);
        assert_eq!(dest.params, vec![Some(50.0f32)]);
    }

    #[test]

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
        assert_eq!(
            dest.params,
            vec![Some(10.0f32), Some(20.0), Some(300.0), Some(400.0)]
        );
    }

    #[test]

    fn fit_b_zoom_mode() {
        let arr = [name("FitB")];
        let dest = Dest::from_array(&arr, Some(0));
        assert_eq!(dest.zoom_mode, ZoomMode::FitB);
        assert!(dest.params.is_empty());
    }

    #[test]

    fn fit_bh_zoom_mode() {
        let arr = [name("FitBH"), real(500.0)];
        let dest = Dest::from_array(&arr, Some(0));
        assert_eq!(dest.zoom_mode, ZoomMode::FitBH);
        assert_eq!(dest.params, vec![Some(500.0f32)]);
    }

    #[test]

    fn fit_bv_zoom_mode() {
        let arr = [name("FitBV"), integer(72)];
        let dest = Dest::from_array(&arr, Some(0));
        assert_eq!(dest.zoom_mode, ZoomMode::FitBV);
        assert_eq!(dest.params, vec![Some(72.0f32)]);
    }

    #[test]

    fn unknown_zoom_mode() {
        let arr = [name("Invalid")];
        let dest = Dest::from_array(&arr, Some(0));
        assert_eq!(dest.zoom_mode, ZoomMode::Unknown);
    }

    // --- Null parameters ---

    #[test]

    fn xyz_with_null_params() {
        let arr = [name("XYZ"), null(), null(), null()];
        let dest = Dest::from_array(&arr, Some(0));
        assert_eq!(dest.zoom_mode, ZoomMode::XYZ);
        assert_eq!(dest.params.len(), 3);
    }

    // --- Empty / malformed arrays ---

    #[test]

    fn empty_array() {
        let arr: [PdfObject; 0] = [];
        let dest = Dest::from_array(&arr, Some(0));
        assert_eq!(dest.zoom_mode, ZoomMode::Unknown);
    }

    // --- xyz() accessor ---

    #[test]

    fn xyz_accessor_returns_values() {
        let arr = [name("XYZ"), real(10.0), real(20.0), real(2.0)];
        let dest = Dest::from_array(&arr, Some(0));
        let (left, top, zoom) = dest.xyz().unwrap();
        assert_eq!(left, Some(10.0f32));
        assert_eq!(top, Some(20.0f32));
        assert_eq!(zoom, Some(2.0f32));
    }

    #[test]

    fn xyz_accessor_with_null_values() {
        let arr = [name("XYZ"), null(), real(20.0), null()];
        let dest = Dest::from_array(&arr, Some(0));
        let (left, top, zoom) = dest.xyz().unwrap();
        assert_eq!(left, None);
        assert_eq!(top, Some(20.0f32));
        assert_eq!(zoom, None);
    }

    #[test]

    fn xyz_accessor_returns_none_for_non_xyz() {
        let arr = [name("Fit")];
        let dest = Dest::from_array(&arr, Some(0));
        assert!(dest.xyz().is_none());
    }

    // --- Integer parameters ---

    #[test]

    fn integer_params_converted_to_f32() {
        let arr = [name("XYZ"), integer(100), integer(200), integer(1)];
        let dest = Dest::from_array(&arr, Some(0));
        assert_eq!(dest.params, vec![Some(100.0f32), Some(200.0), Some(1.0)]);
    }

    // --- Explicit zero coordinates ---

    #[test]
    fn xyz_explicit_zero_not_null() {
        // PDF XYZ with left=0, top=0, zoom=0 means "retain current position"
        // These must round-trip as Some(0.0), not None.
        let arr = [name("XYZ"), real(0.0), real(0.0), real(0.0)];
        let dest = Dest::from_array(&arr, Some(0));
        assert_eq!(dest.params, vec![Some(0.0f32), Some(0.0), Some(0.0)]);
        let (left, top, zoom) = dest.xyz().unwrap();
        assert_eq!(left, Some(0.0f32));
        assert_eq!(top, Some(0.0f32));
        assert_eq!(zoom, Some(0.0f32));
    }
}
