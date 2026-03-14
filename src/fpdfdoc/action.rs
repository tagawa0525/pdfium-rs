use crate::fpdfapi::parser::object::PdfDictionary;

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
    #[allow(dead_code)]
    dict: PdfDictionary,
}

impl Action {
    pub fn from_dict(dict: PdfDictionary) -> Self {
        let _ = dict;
        todo!()
    }

    /// Returns the action type parsed from the `/S` entry.
    pub fn action_type(&self) -> ActionType {
        todo!()
    }

    /// Returns the URI string from `/URI` (for `ActionType::Uri`).
    pub fn uri(&self) -> Option<String> {
        todo!()
    }

    /// Returns the named action string from `/N` (for `ActionType::Named`).
    pub fn named_action(&self) -> Option<String> {
        todo!()
    }

    /// Returns the JavaScript source from `/JS` (string form only).
    pub fn javascript(&self) -> Option<String> {
        todo!()
    }

    /// Returns the file path from `/F` (for `GoToR`, `Launch` etc.).
    pub fn file_path(&self) -> Option<String> {
        todo!()
    }

    /// Returns chained sub-actions from `/Next`.
    pub fn sub_actions(&self) -> Vec<Action> {
        todo!()
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
    #[ignore = "not yet implemented"]
    fn action_type_goto() {
        assert_eq!(make_action("GoTo").action_type(), ActionType::GoTo);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn action_type_gotor() {
        assert_eq!(make_action("GoToR").action_type(), ActionType::GoToR);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn action_type_gotoe() {
        assert_eq!(make_action("GoToE").action_type(), ActionType::GoToE);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn action_type_launch() {
        assert_eq!(make_action("Launch").action_type(), ActionType::Launch);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn action_type_uri() {
        assert_eq!(make_action("URI").action_type(), ActionType::Uri);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn action_type_named() {
        assert_eq!(make_action("Named").action_type(), ActionType::Named);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn action_type_javascript() {
        assert_eq!(
            make_action("JavaScript").action_type(),
            ActionType::JavaScript
        );
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn action_type_submitform() {
        assert_eq!(
            make_action("SubmitForm").action_type(),
            ActionType::SubmitForm
        );
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn action_type_resetform() {
        assert_eq!(
            make_action("ResetForm").action_type(),
            ActionType::ResetForm
        );
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn action_type_importdata() {
        assert_eq!(
            make_action("ImportData").action_type(),
            ActionType::ImportData
        );
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn action_type_hide() {
        assert_eq!(make_action("Hide").action_type(), ActionType::Hide);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn action_type_sound() {
        assert_eq!(make_action("Sound").action_type(), ActionType::Sound);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn action_type_movie() {
        assert_eq!(make_action("Movie").action_type(), ActionType::Movie);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn action_type_thread() {
        assert_eq!(make_action("Thread").action_type(), ActionType::Thread);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn action_type_setocgstate() {
        assert_eq!(
            make_action("SetOCGState").action_type(),
            ActionType::SetOcgState
        );
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn action_type_rendition() {
        assert_eq!(
            make_action("Rendition").action_type(),
            ActionType::Rendition
        );
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn action_type_trans() {
        assert_eq!(make_action("Trans").action_type(), ActionType::Trans);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn action_type_goto3dview() {
        assert_eq!(
            make_action("GoTo3DView").action_type(),
            ActionType::GoTo3DView
        );
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn action_type_unknown_for_unrecognized() {
        assert_eq!(make_action("Bogus").action_type(), ActionType::Unknown);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn action_type_unknown_when_no_s_entry() {
        let dict = PdfDictionary::new();
        let action = Action::from_dict(dict);
        assert_eq!(action.action_type(), ActionType::Unknown);
    }

    // --- URI extraction ---

    #[test]
    #[ignore = "not yet implemented"]
    fn uri_returns_string_value() {
        let action = make_action_with_string("URI", "URI", "https://example.com");
        assert_eq!(action.uri(), Some("https://example.com".to_string()));
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn uri_returns_none_when_missing() {
        assert_eq!(make_action("URI").uri(), None);
    }

    // --- Named action extraction ---

    #[test]
    #[ignore = "not yet implemented"]
    fn named_action_returns_name_value() {
        let mut dict = PdfDictionary::new();
        dict.set("S", PdfObject::Name(PdfByteString::from("Named")));
        dict.set("N", PdfObject::Name(PdfByteString::from("NextPage")));
        let action = Action::from_dict(dict);
        assert_eq!(action.named_action(), Some("NextPage".to_string()));
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn named_action_returns_none_when_missing() {
        assert_eq!(make_action("Named").named_action(), None);
    }

    // --- JavaScript extraction ---

    #[test]
    #[ignore = "not yet implemented"]
    fn javascript_returns_string_value() {
        let action = make_action_with_string("JavaScript", "JS", "app.alert('hello')");
        assert_eq!(action.javascript(), Some("app.alert('hello')".to_string()));
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn javascript_returns_none_when_missing() {
        assert_eq!(make_action("JavaScript").javascript(), None);
    }

    // --- File path extraction ---

    #[test]
    #[ignore = "not yet implemented"]
    fn file_path_returns_string_value() {
        let action = make_action_with_string("Launch", "F", "/path/to/file.pdf");
        assert_eq!(action.file_path(), Some("/path/to/file.pdf".to_string()));
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn file_path_returns_none_when_missing() {
        assert_eq!(make_action("Launch").file_path(), None);
    }

    // --- Sub-actions ---

    #[test]
    #[ignore = "not yet implemented"]
    fn sub_actions_empty_when_no_next() {
        assert!(make_action("GoTo").sub_actions().is_empty());
    }

    #[test]
    #[ignore = "not yet implemented"]
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
    #[ignore = "not yet implemented"]
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
