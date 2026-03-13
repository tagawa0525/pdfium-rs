use std::borrow::Borrow;
use std::fmt;
use std::ops::Deref;

/// PDF byte string. Wraps `Vec<u8>` for raw PDF byte sequences
/// that are not guaranteed to be valid UTF-8.
///
/// Corresponds to C++ `ByteString`.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct PdfByteString {
    data: Vec<u8>,
}

impl PdfByteString {
    pub fn new() -> Self {
        PdfByteString { data: Vec::new() }
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        PdfByteString {
            data: bytes.to_vec(),
        }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    /// Try to interpret as UTF-8 string.
    pub fn as_str(&self) -> Option<&str> {
        std::str::from_utf8(&self.data).ok()
    }

    /// Encode bytes as uppercase hex string.
    pub fn to_hex(&self) -> String {
        self.data.iter().map(|b| format!("{b:02X}")).collect()
    }

    /// Decode hex string into bytes.
    /// Odd-length hex strings are padded with a trailing 0 nibble (PDF spec behavior).
    pub fn from_hex(hex: &str) -> Option<Self> {
        let hex = hex.as_bytes();
        let mut bytes = Vec::with_capacity((hex.len() + 1) / 2);

        let mut i = 0;
        while i < hex.len() {
            let hi = hex_nibble(hex[i])?;
            let lo = if i + 1 < hex.len() {
                hex_nibble(hex[i + 1])?
            } else {
                0
            };
            bytes.push((hi << 4) | lo);
            i += 2;
        }

        Some(PdfByteString { data: bytes })
    }

    /// Case-insensitive comparison (ASCII only).
    pub fn eq_ignore_ascii_case(&self, other: &PdfByteString) -> bool {
        self.data.eq_ignore_ascii_case(&other.data)
    }

    pub fn starts_with(&self, prefix: &[u8]) -> bool {
        self.data.starts_with(prefix)
    }

    pub fn find(&self, needle: &[u8]) -> Option<usize> {
        self.data.windows(needle.len()).position(|w| w == needle)
    }

    pub fn substr(&self, offset: usize, count: usize) -> Self {
        let end = (offset + count).min(self.data.len());
        let start = offset.min(self.data.len());
        PdfByteString {
            data: self.data[start..end].to_vec(),
        }
    }

    pub fn to_uppercase(&self) -> Self {
        PdfByteString {
            data: self.data.to_ascii_uppercase(),
        }
    }

    pub fn to_lowercase(&self) -> Self {
        PdfByteString {
            data: self.data.to_ascii_lowercase(),
        }
    }

    pub fn trim_whitespace(&self) -> Self {
        let start = self.data.iter().position(|b| !b.is_ascii_whitespace());
        let end = self.data.iter().rposition(|b| !b.is_ascii_whitespace());
        match (start, end) {
            (Some(s), Some(e)) => PdfByteString {
                data: self.data[s..=e].to_vec(),
            },
            _ => PdfByteString::new(),
        }
    }
}

fn hex_nibble(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

impl From<&[u8]> for PdfByteString {
    fn from(bytes: &[u8]) -> Self {
        PdfByteString::from_bytes(bytes)
    }
}

impl From<&str> for PdfByteString {
    fn from(s: &str) -> Self {
        PdfByteString {
            data: s.as_bytes().to_vec(),
        }
    }
}

impl From<Vec<u8>> for PdfByteString {
    fn from(v: Vec<u8>) -> Self {
        PdfByteString { data: v }
    }
}

impl Deref for PdfByteString {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        &self.data
    }
}

impl Borrow<[u8]> for PdfByteString {
    fn borrow(&self) -> &[u8] {
        &self.data
    }
}

impl fmt::Debug for PdfByteString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match std::str::from_utf8(&self.data) {
            Ok(s) => write!(f, "PdfByteString({s:?})"),
            Err(_) => write!(f, "PdfByteString(hex:{})", self.to_hex()),
        }
    }
}

impl fmt::Display for PdfByteString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match std::str::from_utf8(&self.data) {
            Ok(s) => f.write_str(s),
            Err(_) => write!(f, "<{}>", self.to_hex()),
        }
    }
}

impl PartialEq<&[u8]> for PdfByteString {
    fn eq(&self, other: &&[u8]) -> bool {
        self.data.as_slice() == *other
    }
}

impl PartialEq<&str> for PdfByteString {
    fn eq(&self, other: &&str) -> bool {
        self.data.as_slice() == other.as_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_is_empty() {
        let s = PdfByteString::new();
        assert!(s.is_empty());
        assert_eq!(s.len(), 0);
    }

    #[test]
    fn from_bytes() {
        let s = PdfByteString::from_bytes(b"hello");
        assert_eq!(s.len(), 5);
        assert_eq!(s.as_bytes(), b"hello");
    }

    #[test]
    fn from_str() {
        let s = PdfByteString::from("hello");
        assert_eq!(s.as_bytes(), b"hello");
        assert_eq!(s.as_str(), Some("hello"));
    }

    #[test]
    fn from_non_utf8() {
        let s = PdfByteString::from_bytes(&[0xFF, 0xFE, 0x00]);
        assert_eq!(s.len(), 3);
        assert_eq!(s.as_str(), None);
    }

    #[test]
    fn deref_to_slice() {
        let s = PdfByteString::from("abc");
        let slice: &[u8] = &s;
        assert_eq!(slice, b"abc");
    }

    #[test]
    fn equality() {
        let a = PdfByteString::from("hello");
        let b = PdfByteString::from("hello");
        let c = PdfByteString::from("world");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn eq_slice() {
        let s = PdfByteString::from("hello");
        assert_eq!(s, b"hello".as_slice());
    }

    #[test]
    fn eq_str() {
        let s = PdfByteString::from("hello");
        assert_eq!(s, "hello");
    }

    #[test]
    fn hex_encode() {
        let s = PdfByteString::from_bytes(&[0xDE, 0xAD, 0xBE, 0xEF]);
        assert_eq!(s.to_hex(), "DEADBEEF");
    }

    #[test]
    fn hex_encode_empty() {
        let s = PdfByteString::new();
        assert_eq!(s.to_hex(), "");
    }

    #[test]
    fn hex_decode() {
        let s = PdfByteString::from_hex("DEADBEEF").unwrap();
        assert_eq!(s.as_bytes(), &[0xDE, 0xAD, 0xBE, 0xEF]);
    }

    #[test]
    fn hex_decode_lowercase() {
        let s = PdfByteString::from_hex("deadbeef").unwrap();
        assert_eq!(s.as_bytes(), &[0xDE, 0xAD, 0xBE, 0xEF]);
    }

    #[test]
    fn hex_decode_odd_length() {
        // Odd-length hex: last nibble padded with 0
        let s = PdfByteString::from_hex("ABC").unwrap();
        assert_eq!(s.as_bytes(), &[0xAB, 0xC0]);
    }

    #[test]
    fn hex_decode_invalid() {
        assert!(PdfByteString::from_hex("GHIJ").is_none());
    }

    #[test]
    fn eq_ignore_ascii_case() {
        let a = PdfByteString::from("Hello");
        let b = PdfByteString::from("hello");
        let c = PdfByteString::from("HELLO");
        assert!(a.eq_ignore_ascii_case(&b));
        assert!(a.eq_ignore_ascii_case(&c));
        assert!(!a.eq_ignore_ascii_case(&PdfByteString::from("world")));
    }

    #[test]
    fn starts_with() {
        let s = PdfByteString::from("hello world");
        assert!(s.starts_with(b"hello"));
        assert!(!s.starts_with(b"world"));
        assert!(s.starts_with(b""));
    }

    #[test]
    fn find_bytes() {
        let s = PdfByteString::from("hello world");
        assert_eq!(s.find(b"world"), Some(6));
        assert_eq!(s.find(b"xyz"), None);
        assert_eq!(s.find(b"hello"), Some(0));
    }

    #[test]
    fn substr() {
        let s = PdfByteString::from("hello world");
        assert_eq!(s.substr(6, 5), "world");
    }

    #[test]
    fn to_uppercase() {
        let s = PdfByteString::from("Hello World");
        assert_eq!(s.to_uppercase(), "HELLO WORLD");
    }

    #[test]
    fn to_lowercase() {
        let s = PdfByteString::from("Hello World");
        assert_eq!(s.to_lowercase(), "hello world");
    }

    #[test]
    fn trim_whitespace() {
        let s = PdfByteString::from("  hello  ");
        assert_eq!(s.trim_whitespace(), "hello");
    }

    #[test]
    fn display_utf8() {
        let s = PdfByteString::from("hello");
        assert_eq!(format!("{s}"), "hello");
    }

    #[test]
    fn display_non_utf8() {
        let s = PdfByteString::from_bytes(&[0xFF, 0xFE]);
        let display = format!("{s}");
        assert!(display.contains("FF"), "got: {display}");
        assert!(display.contains("FE"), "got: {display}");
    }

    #[test]
    fn debug_format() {
        let s = PdfByteString::from("test");
        let debug = format!("{s:?}");
        assert!(debug.contains("test"), "got: {debug}");
    }

    #[test]
    fn from_vec() {
        let v = vec![1u8, 2, 3];
        let s = PdfByteString::from(v);
        assert_eq!(s.as_bytes(), &[1, 2, 3]);
    }

    #[test]
    fn default_is_empty() {
        let s = PdfByteString::default();
        assert!(s.is_empty());
    }
}
