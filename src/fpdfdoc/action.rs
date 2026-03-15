use crate::fpdfapi::parser::object::PdfDictionary;
use crate::fpdfdoc::util::decode_pdf_text_string;

/// Type of a PDF action.
///
/// Corresponds to C++ `CPDF_Action::Type` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionType {
    Unknown,
    GoTo,
    GoToR,
    GoToE,
    Launch,
    Uri,
    Named,
    JavaScript,
    SubmitForm,
    ResetForm,
    ImportData,
    Hide,
    Sound,
    Movie,
    Thread,
    SetOcgState,
    Rendition,
    Trans,
    GoTo3DView,
}

/// A PDF action dictionary.
///
/// Corresponds to C++ `CPDF_Action`.
#[derive(Debug, Clone)]
pub struct Action {
    dict: PdfDictionary,
}

impl Action {
    pub fn from_dict(dict: PdfDictionary) -> Self {
        Action { dict }
    }

    /// Returns the action type parsed from the `/S` entry.
    pub fn action_type(&self) -> ActionType {
        self.dict
            .get_name(b"S")
            .map(|name| match name.as_bytes() {
                b"GoTo" => ActionType::GoTo,
                b"GoToR" => ActionType::GoToR,
                b"GoToE" => ActionType::GoToE,
                b"Launch" => ActionType::Launch,
                b"URI" => ActionType::Uri,
                b"Named" => ActionType::Named,
                b"JavaScript" => ActionType::JavaScript,
                b"SubmitForm" => ActionType::SubmitForm,
                b"ResetForm" => ActionType::ResetForm,
                b"ImportData" => ActionType::ImportData,
                b"Hide" => ActionType::Hide,
                b"Sound" => ActionType::Sound,
                b"Movie" => ActionType::Movie,
                b"Thread" => ActionType::Thread,
                b"SetOCGState" => ActionType::SetOcgState,
                b"Rendition" => ActionType::Rendition,
                b"Trans" => ActionType::Trans,
                b"GoTo3DView" => ActionType::GoTo3DView,
                _ => ActionType::Unknown,
            })
            .unwrap_or(ActionType::Unknown)
    }

    /// Returns the URI string from `/URI` (for `ActionType::Uri`).
    ///
    /// URIs are ASCII per PDF spec; non-ASCII bytes are passed through via
    /// lossy UTF-8 conversion.
    pub fn uri(&self) -> Option<String> {
        self.dict
            .get_string(b"URI")
            .map(|s| String::from_utf8_lossy(s.as_bytes()).into_owned())
    }

    /// Returns the named action string from `/N` (for `ActionType::Named`).
    pub fn named_action(&self) -> Option<String> {
        self.dict
            .get_name(b"N")
            .map(|s| String::from_utf8_lossy(s.as_bytes()).into_owned())
    }

    /// Returns the JavaScript source from `/JS` (string form only).
    ///
    /// Unlike other text fields, control characters (newlines, tabs) are
    /// preserved since they are semantically significant in JavaScript.
    /// Stream-based `/JS` is not supported yet.
    pub fn javascript(&self) -> Option<String> {
        self.dict
            .get_string(b"JS")
            .map(|s| String::from_utf8_lossy(s.as_bytes()).into_owned())
    }

    /// Returns the file path from `/F` (string form only).
    ///
    /// Only handles string `/F` entries. Dictionary file specifications
    /// (e.g., `<< /Type /Filespec /F (...) >>`) are not yet supported.
    pub fn file_path(&self) -> Option<String> {
        self.dict
            .get_string(b"F")
            .map(|s| decode_pdf_text_string(s.as_bytes()))
    }

    /// Returns chained sub-actions from `/Next`.
    ///
    /// `/Next` can be a single action dictionary or an array of dictionaries.
    /// Only direct (inline) dictionaries are collected; indirect references
    /// within `/Next` require `Document` access and are currently skipped.
    pub fn sub_actions(&self) -> Vec<Action> {
        use crate::fpdfapi::parser::object::PdfObject;
        match self.dict.get(b"Next") {
            Some(PdfObject::Dictionary(d)) => vec![Action::from_dict(d.clone())],
            Some(PdfObject::Array(arr)) => arr
                .iter()
                .filter_map(|obj| obj.as_dict())
                .map(|d| Action::from_dict(d.clone()))
                .collect(),
            _ => vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fpdfapi::parser::object::PdfObject;
    use crate::fxcrt::bytestring::PdfByteString;

    fn make_action(s_value: &str) -> Action {
        let mut dict = PdfDictionary::new();
        dict.set("S", PdfObject::Name(PdfByteString::from(s_value)));
        Action::from_dict(dict)
    }

    fn make_action_with_string(s_value: &str, key: &str, value: &str) -> Action {
        let mut dict = PdfDictionary::new();
        dict.set("S", PdfObject::Name(PdfByteString::from(s_value)));
        dict.set(key, PdfObject::String(PdfByteString::from(value)));
        Action::from_dict(dict)
    }

    // --- ActionType from /S ---

    #[test]

    fn action_type_goto() {
        assert_eq!(make_action("GoTo").action_type(), ActionType::GoTo);
    }

    #[test]

    fn action_type_gotor() {
        assert_eq!(make_action("GoToR").action_type(), ActionType::GoToR);
    }

    #[test]

    fn action_type_gotoe() {
        assert_eq!(make_action("GoToE").action_type(), ActionType::GoToE);
    }

    #[test]

    fn action_type_launch() {
        assert_eq!(make_action("Launch").action_type(), ActionType::Launch);
    }

    #[test]

    fn action_type_uri() {
        assert_eq!(make_action("URI").action_type(), ActionType::Uri);
    }

    #[test]

    fn action_type_named() {
        assert_eq!(make_action("Named").action_type(), ActionType::Named);
    }

    #[test]

    fn action_type_javascript() {
        assert_eq!(
            make_action("JavaScript").action_type(),
            ActionType::JavaScript
        );
    }

    #[test]

    fn action_type_submitform() {
        assert_eq!(
            make_action("SubmitForm").action_type(),
            ActionType::SubmitForm
        );
    }

    #[test]

    fn action_type_resetform() {
        assert_eq!(
            make_action("ResetForm").action_type(),
            ActionType::ResetForm
        );
    }

    #[test]

    fn action_type_importdata() {
        assert_eq!(
            make_action("ImportData").action_type(),
            ActionType::ImportData
        );
    }

    #[test]

    fn action_type_hide() {
        assert_eq!(make_action("Hide").action_type(), ActionType::Hide);
    }

    #[test]

    fn action_type_sound() {
        assert_eq!(make_action("Sound").action_type(), ActionType::Sound);
    }

    #[test]

    fn action_type_movie() {
        assert_eq!(make_action("Movie").action_type(), ActionType::Movie);
    }

    #[test]

    fn action_type_thread() {
        assert_eq!(make_action("Thread").action_type(), ActionType::Thread);
    }

    #[test]

    fn action_type_setocgstate() {
        assert_eq!(
            make_action("SetOCGState").action_type(),
            ActionType::SetOcgState
        );
    }

    #[test]

    fn action_type_rendition() {
        assert_eq!(
            make_action("Rendition").action_type(),
            ActionType::Rendition
        );
    }

    #[test]

    fn action_type_trans() {
        assert_eq!(make_action("Trans").action_type(), ActionType::Trans);
    }

    #[test]

    fn action_type_goto3dview() {
        assert_eq!(
            make_action("GoTo3DView").action_type(),
            ActionType::GoTo3DView
        );
    }

    #[test]

    fn action_type_unknown_for_unrecognized() {
        assert_eq!(make_action("Bogus").action_type(), ActionType::Unknown);
    }

    #[test]

    fn action_type_unknown_when_no_s_entry() {
        let dict = PdfDictionary::new();
        let action = Action::from_dict(dict);
        assert_eq!(action.action_type(), ActionType::Unknown);
    }

    // --- URI extraction ---

    #[test]

    fn uri_returns_string_value() {
        let action = make_action_with_string("URI", "URI", "https://example.com");
        assert_eq!(action.uri(), Some("https://example.com".to_string()));
    }

    #[test]

    fn uri_returns_none_when_missing() {
        assert_eq!(make_action("URI").uri(), None);
    }

    // --- Named action extraction ---

    #[test]

    fn named_action_returns_name_value() {
        let mut dict = PdfDictionary::new();
        dict.set("S", PdfObject::Name(PdfByteString::from("Named")));
        dict.set("N", PdfObject::Name(PdfByteString::from("NextPage")));
        let action = Action::from_dict(dict);
        assert_eq!(action.named_action(), Some("NextPage".to_string()));
    }

    #[test]

    fn named_action_returns_none_when_missing() {
        assert_eq!(make_action("Named").named_action(), None);
    }

    // --- JavaScript extraction ---

    #[test]

    fn javascript_returns_string_value() {
        let action = make_action_with_string("JavaScript", "JS", "app.alert('hello')");
        assert_eq!(action.javascript(), Some("app.alert('hello')".to_string()));
    }

    #[test]

    fn javascript_returns_none_when_missing() {
        assert_eq!(make_action("JavaScript").javascript(), None);
    }

    // --- File path extraction ---

    #[test]

    fn file_path_returns_string_value() {
        let action = make_action_with_string("Launch", "F", "/path/to/file.pdf");
        assert_eq!(action.file_path(), Some("/path/to/file.pdf".to_string()));
    }

    #[test]

    fn file_path_returns_none_when_missing() {
        assert_eq!(make_action("Launch").file_path(), None);
    }

    // --- Sub-actions ---

    #[test]

    fn sub_actions_empty_when_no_next() {
        assert!(make_action("GoTo").sub_actions().is_empty());
    }

    #[test]

    fn sub_actions_single_dict() {
        let mut next_dict = PdfDictionary::new();
        next_dict.set("S", PdfObject::Name(PdfByteString::from("URI")));
        next_dict.set(
            "URI",
            PdfObject::String(PdfByteString::from("https://next.example.com")),
        );

        let mut dict = PdfDictionary::new();
        dict.set("S", PdfObject::Name(PdfByteString::from("GoTo")));
        dict.set("Next", PdfObject::Dictionary(next_dict));
        let action = Action::from_dict(dict);

        let subs = action.sub_actions();
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0].action_type(), ActionType::Uri);
        assert_eq!(subs[0].uri(), Some("https://next.example.com".to_string()));
    }

    #[test]

    fn sub_actions_array_of_dicts() {
        let mut d1 = PdfDictionary::new();
        d1.set("S", PdfObject::Name(PdfByteString::from("URI")));
        let mut d2 = PdfDictionary::new();
        d2.set("S", PdfObject::Name(PdfByteString::from("Named")));

        let mut dict = PdfDictionary::new();
        dict.set("S", PdfObject::Name(PdfByteString::from("GoTo")));
        dict.set(
            "Next",
            PdfObject::Array(vec![PdfObject::Dictionary(d1), PdfObject::Dictionary(d2)]),
        );
        let action = Action::from_dict(dict);

        let subs = action.sub_actions();
        assert_eq!(subs.len(), 2);
        assert_eq!(subs[0].action_type(), ActionType::Uri);
        assert_eq!(subs[1].action_type(), ActionType::Named);
    }
}
