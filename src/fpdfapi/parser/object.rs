use std::collections::BTreeMap;
use std::fmt;

use crate::fxcrt::bytestring::PdfByteString;

/// Identifier for an indirect PDF object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObjectId {
    pub num: u32,
    pub gen_num: u16,
}

/// Core PDF object type, modeled as an enum.
///
/// Corresponds to C++ `CPDF_Object` hierarchy.
#[derive(Clone, PartialEq)]
pub enum PdfObject {
    Boolean(bool),
    Integer(i32),
    Real(f64),
    String(PdfByteString),
    Name(PdfByteString),
    Array(Vec<PdfObject>),
    Dictionary(PdfDictionary),
    Stream(PdfStream),
    Null,
    Reference(ObjectId),
}

/// PDF dictionary: ordered key-value map with name keys.
#[derive(Clone, PartialEq, Default)]
pub struct PdfDictionary {
    entries: BTreeMap<PdfByteString, PdfObject>,
}

/// PDF stream: dictionary metadata + raw byte data.
#[derive(Clone, PartialEq)]
pub struct PdfStream {
    pub dict: PdfDictionary,
    pub data: Vec<u8>,
}

// --- ObjectId ---

impl ObjectId {
    pub fn new(num: u32, gen_num: u16) -> Self {
        ObjectId { num, gen_num }
    }
}

// --- PdfObject accessors ---

impl PdfObject {
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            PdfObject::Boolean(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_i32(&self) -> Option<i32> {
        match self {
            PdfObject::Integer(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            PdfObject::Real(v) => Some(*v),
            PdfObject::Integer(v) => Some(*v as f64),
            _ => None,
        }
    }

    pub fn as_name(&self) -> Option<&PdfByteString> {
        match self {
            PdfObject::Name(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&PdfByteString> {
        match self {
            PdfObject::String(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[PdfObject]> {
        match self {
            PdfObject::Array(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_dict(&self) -> Option<&PdfDictionary> {
        match self {
            PdfObject::Dictionary(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_stream(&self) -> Option<&PdfStream> {
        match self {
            PdfObject::Stream(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_reference(&self) -> Option<ObjectId> {
        match self {
            PdfObject::Reference(id) => Some(*id),
            _ => None,
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, PdfObject::Null)
    }
}

impl fmt::Debug for PdfObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PdfObject::Boolean(v) => write!(f, "Bool({v})"),
            PdfObject::Integer(v) => write!(f, "Int({v})"),
            PdfObject::Real(v) => write!(f, "Real({v})"),
            PdfObject::String(v) => write!(f, "String({v:?})"),
            PdfObject::Name(v) => write!(f, "Name({v:?})"),
            PdfObject::Array(v) => write!(f, "Array({v:?})"),
            PdfObject::Dictionary(v) => write!(f, "Dict({v:?})"),
            PdfObject::Stream(s) => write!(f, "Stream(dict={:?}, {} bytes)", s.dict, s.data.len()),
            PdfObject::Null => write!(f, "Null"),
            PdfObject::Reference(id) => write!(f, "Ref({} {})", id.num, id.gen_num),
        }
    }
}

// --- PdfDictionary ---

impl PdfDictionary {
    pub fn new() -> Self {
        PdfDictionary {
            entries: BTreeMap::new(),
        }
    }

    pub fn get(&self, key: &[u8]) -> Option<&PdfObject> {
        self.entries.get(key)
    }

    pub fn get_name(&self, key: &[u8]) -> Option<&PdfByteString> {
        self.get(key).and_then(|o| o.as_name())
    }

    pub fn get_string(&self, key: &[u8]) -> Option<&PdfByteString> {
        self.get(key).and_then(|o| o.as_str())
    }

    pub fn get_i32(&self, key: &[u8]) -> Option<i32> {
        self.get(key).and_then(|o| o.as_i32())
    }

    pub fn get_dict(&self, key: &[u8]) -> Option<&PdfDictionary> {
        self.get(key).and_then(|o| o.as_dict())
    }

    pub fn get_array(&self, key: &[u8]) -> Option<&[PdfObject]> {
        self.get(key).and_then(|o| o.as_array())
    }

    pub fn set(&mut self, key: impl Into<PdfByteString>, value: PdfObject) {
        self.entries.insert(key.into(), value);
    }

    pub fn remove(&mut self, key: &[u8]) -> Option<PdfObject> {
        self.entries.remove(key)
    }

    pub fn contains_key(&self, key: &[u8]) -> bool {
        self.entries.contains_key(key)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn keys(&self) -> Vec<&PdfByteString> {
        self.entries.keys().collect()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&PdfByteString, &PdfObject)> {
        self.entries.iter()
    }
}

impl fmt::Debug for PdfDictionary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Dict{{")?;
        for (i, (k, v)) in self.entries.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "/{k}: {v:?}")?;
        }
        write!(f, "}}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- ObjectId ---

    #[test]
    fn object_id_equality() {
        let a = ObjectId::new(1, 0);
        let b = ObjectId::new(1, 0);
        let c = ObjectId::new(2, 0);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    // --- PdfObject construction and access ---

    #[test]
    fn object_boolean() {
        let obj = PdfObject::Boolean(true);
        assert_eq!(obj.as_bool(), Some(true));
        assert_eq!(obj.as_i32(), None);
    }

    #[test]
    fn object_integer() {
        let obj = PdfObject::Integer(42);
        assert_eq!(obj.as_i32(), Some(42));
        assert_eq!(obj.as_f64(), Some(42.0));
        assert_eq!(obj.as_bool(), None);
    }

    #[test]
    fn object_real() {
        let obj = PdfObject::Real(3.14);
        assert_eq!(obj.as_f64(), Some(3.14));
        assert_eq!(obj.as_i32(), None);
    }

    #[test]
    fn object_string() {
        let obj = PdfObject::String(PdfByteString::from("hello"));
        assert_eq!(obj.as_str().unwrap().as_bytes(), b"hello");
        assert_eq!(obj.as_name(), None);
    }

    #[test]
    fn object_name() {
        let obj = PdfObject::Name(PdfByteString::from("Type"));
        assert_eq!(obj.as_name().unwrap().as_bytes(), b"Type");
        assert_eq!(obj.as_str(), None);
    }

    #[test]
    fn object_array() {
        let obj = PdfObject::Array(vec![PdfObject::Integer(1), PdfObject::Integer(2)]);
        let arr = obj.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0].as_i32(), Some(1));
    }

    #[test]
    fn object_null() {
        assert!(PdfObject::Null.is_null());
        assert!(!PdfObject::Integer(0).is_null());
    }

    #[test]
    fn object_reference() {
        let obj = PdfObject::Reference(ObjectId::new(10, 0));
        assert_eq!(obj.as_reference(), Some(ObjectId::new(10, 0)));
    }

    #[test]
    fn object_debug() {
        let obj = PdfObject::Integer(42);
        let debug = format!("{obj:?}");
        assert!(!debug.is_empty());
    }

    // --- PdfDictionary ---

    #[test]
    fn dict_new_is_empty() {
        let d = PdfDictionary::new();
        assert!(d.is_empty());
        assert_eq!(d.len(), 0);
    }

    #[test]
    fn dict_set_and_get() {
        let mut d = PdfDictionary::new();
        d.set("Type", PdfObject::Name(PdfByteString::from("Catalog")));
        assert_eq!(d.get_name(b"Type").unwrap().as_bytes(), b"Catalog");
    }

    #[test]
    fn dict_get_missing_key() {
        let d = PdfDictionary::new();
        assert!(d.get(b"Missing").is_none());
    }

    #[test]
    fn dict_get_i32() {
        let mut d = PdfDictionary::new();
        d.set("Count", PdfObject::Integer(5));
        assert_eq!(d.get_i32(b"Count"), Some(5));
    }

    #[test]
    fn dict_get_string() {
        let mut d = PdfDictionary::new();
        d.set("Title", PdfObject::String(PdfByteString::from("My PDF")));
        assert_eq!(d.get_string(b"Title").unwrap().as_bytes(), b"My PDF");
    }

    #[test]
    fn dict_remove() {
        let mut d = PdfDictionary::new();
        d.set("Key", PdfObject::Integer(1));
        assert!(d.contains_key(b"Key"));
        let removed = d.remove(b"Key");
        assert!(removed.is_some());
        assert!(!d.contains_key(b"Key"));
    }

    #[test]
    fn dict_keys() {
        let mut d = PdfDictionary::new();
        d.set("A", PdfObject::Integer(1));
        d.set("B", PdfObject::Integer(2));
        let keys = d.keys();
        assert_eq!(keys.len(), 2);
    }

    #[test]
    fn dict_debug() {
        let d = PdfDictionary::new();
        let debug = format!("{d:?}");
        assert!(!debug.is_empty());
    }
}
