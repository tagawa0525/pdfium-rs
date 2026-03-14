/// PDF standard 14 font name → bundled Liberation font data.
///
/// Liberation fonts (SIL OFL license) are metric-compatible replacements:
/// - Helvetica variants → Liberation Sans
/// - Times variants → Liberation Serif
/// - Courier variants → Liberation Mono
///
/// Symbol and ZapfDingbats are not covered (different glyph sets).
pub fn standard_font_data(base_font: &str) -> Option<&'static [u8]> {
    // Strip subset prefix (e.g. "ABCDEF+Helvetica" → "Helvetica")
    let name = base_font
        .find('+')
        .map(|i| &base_font[i + 1..])
        .unwrap_or(base_font);

    match name {
        "Helvetica" | "Arial" => Some(include_bytes!(
            "../../../assets/fonts/LiberationSans-Regular.ttf"
        )),
        "Helvetica-Bold" | "Arial-Bold" | "Arial,Bold" => Some(include_bytes!(
            "../../../assets/fonts/LiberationSans-Bold.ttf"
        )),
        "Helvetica-Oblique" | "Helvetica-Italic" | "Arial-Italic" | "Arial,Italic" => Some(
            include_bytes!("../../../assets/fonts/LiberationSans-Italic.ttf"),
        ),
        "Helvetica-BoldOblique"
        | "Helvetica-BoldItalic"
        | "Arial-BoldItalic"
        | "Arial,BoldItalic" => Some(include_bytes!(
            "../../../assets/fonts/LiberationSans-BoldItalic.ttf"
        )),
        "Times-Roman" | "Times" | "TimesNewRoman" | "Times New Roman" => Some(include_bytes!(
            "../../../assets/fonts/LiberationSerif-Regular.ttf"
        )),
        "Times-Bold" | "TimesNewRoman,Bold" | "TimesNewRoman-Bold" => Some(include_bytes!(
            "../../../assets/fonts/LiberationSerif-Bold.ttf"
        )),
        "Times-Italic" | "TimesNewRoman,Italic" | "TimesNewRoman-Italic" => Some(include_bytes!(
            "../../../assets/fonts/LiberationSerif-Italic.ttf"
        )),
        "Times-BoldItalic" | "TimesNewRoman,BoldItalic" | "TimesNewRoman-BoldItalic" => Some(
            include_bytes!("../../../assets/fonts/LiberationSerif-BoldItalic.ttf"),
        ),
        "Courier" | "CourierNew" | "Courier New" => Some(include_bytes!(
            "../../../assets/fonts/LiberationMono-Regular.ttf"
        )),
        "Courier-Bold" | "CourierNew,Bold" | "CourierNew-Bold" => Some(include_bytes!(
            "../../../assets/fonts/LiberationMono-Bold.ttf"
        )),
        "Courier-Oblique" | "Courier-Italic" | "CourierNew,Italic" | "CourierNew-Italic" => Some(
            include_bytes!("../../../assets/fonts/LiberationMono-Italic.ttf"),
        ),
        "Courier-BoldOblique"
        | "Courier-BoldItalic"
        | "CourierNew,BoldItalic"
        | "CourierNew-BoldItalic" => Some(include_bytes!(
            "../../../assets/fonts/LiberationMono-BoldItalic.ttf"
        )),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helvetica_maps_to_some() {
        assert!(standard_font_data("Helvetica").is_some());
    }

    #[test]
    fn times_roman_maps_to_some() {
        assert!(standard_font_data("Times-Roman").is_some());
    }

    #[test]
    fn courier_maps_to_some() {
        assert!(standard_font_data("Courier").is_some());
    }

    #[test]
    fn bold_variants_map_to_some() {
        assert!(standard_font_data("Helvetica-Bold").is_some());
        assert!(standard_font_data("Times-Bold").is_some());
        assert!(standard_font_data("Courier-Bold").is_some());
    }

    #[test]
    fn italic_variants_map_to_some() {
        assert!(standard_font_data("Helvetica-Oblique").is_some());
        assert!(standard_font_data("Times-Italic").is_some());
        assert!(standard_font_data("Courier-Oblique").is_some());
    }

    #[test]
    fn bold_italic_variants_map_to_some() {
        assert!(standard_font_data("Helvetica-BoldOblique").is_some());
        assert!(standard_font_data("Times-BoldItalic").is_some());
        assert!(standard_font_data("Courier-BoldOblique").is_some());
    }

    #[test]
    fn unknown_font_returns_none() {
        assert!(standard_font_data("Unknown").is_none());
        assert!(standard_font_data("Symbol").is_none());
        assert!(standard_font_data("ZapfDingbats").is_none());
    }

    #[test]
    fn subset_prefix_is_stripped() {
        assert!(standard_font_data("ABCDEF+Helvetica").is_some());
        assert!(standard_font_data("XYZABC+Times-Roman").is_some());
    }

    #[test]
    fn returned_data_is_valid_ttf() {
        let data = standard_font_data("Helvetica").unwrap();
        // TrueType/OpenType files start with 0x00010000 or "OTTO"
        assert!(
            data.len() > 4 && (data[..4] == [0x00, 0x01, 0x00, 0x00] || &data[..4] == b"OTTO"),
            "should be a valid TrueType/OpenType file"
        );
    }
}
