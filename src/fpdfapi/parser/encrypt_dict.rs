use crate::error::{Error, Result};
use crate::fpdfapi::parser::object::PdfDictionary;
use crate::fpdfapi::parser::security::{Cipher, EncryptDict};

/// Parse an `EncryptDict` from a PDF `/Encrypt` dictionary.
///
/// Reads `/Filter`, `/V`, `/R`, `/Length`, `/O`, `/U`, `/P`, `/CF`,
/// `/StmF`, `/StrF`, `/EncryptMetadata`, and R5+ entries
/// (`/OE`, `/UE`, `/Perms`).
pub fn parse_encrypt_dict(dict: &PdfDictionary) -> Result<EncryptDict> {
    // /Filter must be /Standard
    let filter = dict
        .get_name(b"Filter")
        .ok_or_else(|| Error::InvalidPdf("/Encrypt missing /Filter".into()))?;
    if filter.as_bytes() != b"Standard" {
        return Err(Error::InvalidPdf(format!(
            "unsupported security handler: {}",
            filter
        )));
    }

    let v = dict.get_i32(b"V").unwrap_or(0);
    let revision = dict
        .get_i32(b"R")
        .ok_or_else(|| Error::InvalidPdf("/Encrypt missing /R".into()))? as u32;

    // Key length: /Length is in bits, default 40
    let length_bits = dict.get_i32(b"Length").unwrap_or(40);
    let key_length = (length_bits / 8) as usize;

    // Determine cipher from /V and /CF
    let cipher = determine_cipher(v, dict)?;

    // Override key_length for AES-256
    let key_length = if cipher == Cipher::Aes256 {
        32
    } else {
        key_length
    };

    let permissions = dict
        .get_i32(b"P")
        .ok_or_else(|| Error::InvalidPdf("/Encrypt missing /P".into()))?;

    let owner_hash = dict
        .get(b"O")
        .and_then(|o| o.as_str())
        .map(|s| s.as_bytes().to_vec())
        .ok_or_else(|| Error::InvalidPdf("/Encrypt missing /O".into()))?;

    let user_hash = dict
        .get(b"U")
        .and_then(|o| o.as_str())
        .map(|s| s.as_bytes().to_vec())
        .ok_or_else(|| Error::InvalidPdf("/Encrypt missing /U".into()))?;

    // R5+ entries
    let owner_encrypted_key = dict
        .get(b"OE")
        .and_then(|o| o.as_str())
        .map(|s| s.as_bytes().to_vec());
    let user_encrypted_key = dict
        .get(b"UE")
        .and_then(|o| o.as_str())
        .map(|s| s.as_bytes().to_vec());
    let encrypted_perms = dict
        .get(b"Perms")
        .and_then(|o| o.as_str())
        .map(|s| s.as_bytes().to_vec());

    let encrypt_metadata = dict
        .get(b"EncryptMetadata")
        .and_then(|o| o.as_bool())
        .unwrap_or(true);

    Ok(EncryptDict {
        revision,
        key_length,
        cipher,
        permissions,
        owner_hash,
        user_hash,
        owner_encrypted_key,
        user_encrypted_key,
        encrypted_perms,
        encrypt_metadata,
    })
}

/// Extract the first file ID from the trailer `/ID` array.
///
/// PDF spec requires `/ID` to be an array of two strings.
/// Returns the first string as bytes, or an empty vec if absent.
pub fn extract_file_id(trailer: &PdfDictionary) -> Vec<u8> {
    trailer
        .get_array(b"ID")
        .and_then(|arr| arr.first())
        .and_then(|obj| obj.as_str())
        .map(|s| s.as_bytes().to_vec())
        .unwrap_or_default()
}

/// Determine the cipher from the /V value and /CF sub-dictionaries.
fn determine_cipher(v: i32, dict: &PdfDictionary) -> Result<Cipher> {
    match v {
        1 | 2 => Ok(Cipher::Rc4),
        4 => {
            // Check /CF -> /StdCF -> /CFM for AES
            let cfm = dict
                .get_dict(b"CF")
                .and_then(|cf| cf.get_dict(b"StdCF"))
                .and_then(|stdcf| stdcf.get_name(b"CFM"))
                .map(|n| n.as_bytes().to_vec());
            match cfm.as_deref() {
                Some(b"AESV2") => Ok(Cipher::Aes128),
                _ => Ok(Cipher::Rc4), // V4 default or V4RC4
            }
        }
        5 => {
            // Check for AESV3
            let cfm = dict
                .get_dict(b"CF")
                .and_then(|cf| cf.get_dict(b"StdCF"))
                .and_then(|stdcf| stdcf.get_name(b"CFM"))
                .map(|n| n.as_bytes().to_vec());
            match cfm.as_deref() {
                Some(b"AESV3") => Ok(Cipher::Aes256),
                _ => Err(Error::InvalidPdf("V=5 requires AESV3 cipher".into())),
            }
        }
        _ => Err(Error::InvalidPdf(format!("unsupported /V value: {v}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fpdfapi::parser::object::{PdfDictionary, PdfObject};
    use crate::fxcrt::bytestring::PdfByteString;

    /// Helper: build a minimal /Encrypt dictionary for RC4 revision 2.
    fn make_rc4_r2_encrypt_dict() -> PdfDictionary {
        let mut dict = PdfDictionary::new();
        dict.set("Filter", PdfObject::Name(PdfByteString::from("Standard")));
        dict.set("V", PdfObject::Integer(1));
        dict.set("R", PdfObject::Integer(2));
        dict.set("Length", PdfObject::Integer(40));
        dict.set("P", PdfObject::Integer(-4));
        // 32-byte /O and /U values (all zeros for test)
        dict.set("O", PdfObject::String(PdfByteString::from(vec![0u8; 32])));
        dict.set("U", PdfObject::String(PdfByteString::from(vec![0u8; 32])));
        dict
    }

    /// Helper: build a minimal /Encrypt dictionary for AES-128 revision 4.
    fn make_aes128_r4_encrypt_dict() -> PdfDictionary {
        let mut dict = PdfDictionary::new();
        dict.set("Filter", PdfObject::Name(PdfByteString::from("Standard")));
        dict.set("V", PdfObject::Integer(4));
        dict.set("R", PdfObject::Integer(4));
        dict.set("Length", PdfObject::Integer(128));
        dict.set("P", PdfObject::Integer(-4));
        dict.set("O", PdfObject::String(PdfByteString::from(vec![0u8; 32])));
        dict.set("U", PdfObject::String(PdfByteString::from(vec![0u8; 32])));

        // /CF << /StdCF << /CFM /AESV2 /AuthEvent /DocOpen /Length 16 >> >>
        let mut stdcf = PdfDictionary::new();
        stdcf.set("CFM", PdfObject::Name(PdfByteString::from("AESV2")));
        stdcf.set("AuthEvent", PdfObject::Name(PdfByteString::from("DocOpen")));
        stdcf.set("Length", PdfObject::Integer(16));
        let mut cf = PdfDictionary::new();
        cf.set("StdCF", PdfObject::Dictionary(stdcf));
        dict.set("CF", PdfObject::Dictionary(cf));

        dict.set("StmF", PdfObject::Name(PdfByteString::from("StdCF")));
        dict.set("StrF", PdfObject::Name(PdfByteString::from("StdCF")));
        dict
    }

    /// Helper: build a minimal /Encrypt dictionary for AES-256 revision 6.
    fn make_aes256_r6_encrypt_dict() -> PdfDictionary {
        let mut dict = PdfDictionary::new();
        dict.set("Filter", PdfObject::Name(PdfByteString::from("Standard")));
        dict.set("V", PdfObject::Integer(5));
        dict.set("R", PdfObject::Integer(6));
        dict.set("Length", PdfObject::Integer(256));
        dict.set("P", PdfObject::Integer(-4));
        dict.set("O", PdfObject::String(PdfByteString::from(vec![0u8; 48])));
        dict.set("U", PdfObject::String(PdfByteString::from(vec![0u8; 48])));
        dict.set("OE", PdfObject::String(PdfByteString::from(vec![0u8; 32])));
        dict.set("UE", PdfObject::String(PdfByteString::from(vec![0u8; 32])));
        dict.set(
            "Perms",
            PdfObject::String(PdfByteString::from(vec![0u8; 16])),
        );

        let mut stdcf = PdfDictionary::new();
        stdcf.set("CFM", PdfObject::Name(PdfByteString::from("AESV3")));
        stdcf.set("AuthEvent", PdfObject::Name(PdfByteString::from("DocOpen")));
        stdcf.set("Length", PdfObject::Integer(32));
        let mut cf = PdfDictionary::new();
        cf.set("StdCF", PdfObject::Dictionary(stdcf));
        dict.set("CF", PdfObject::Dictionary(cf));

        dict.set("StmF", PdfObject::Name(PdfByteString::from("StdCF")));
        dict.set("StrF", PdfObject::Name(PdfByteString::from("StdCF")));
        dict
    }

    #[test]
    fn parse_rc4_r2_encrypt_dict() {
        let dict = make_rc4_r2_encrypt_dict();
        let ed = parse_encrypt_dict(&dict).unwrap();
        assert_eq!(ed.revision, 2);
        assert_eq!(ed.key_length, 5); // 40 bits = 5 bytes
        assert_eq!(ed.cipher, Cipher::Rc4);
        assert_eq!(ed.permissions, -4);
        assert!(!ed.owner_hash.is_empty());
        assert!(!ed.user_hash.is_empty());
        assert!(ed.encrypt_metadata); // default true
    }

    #[test]
    fn parse_aes128_r4_encrypt_dict() {
        let dict = make_aes128_r4_encrypt_dict();
        let ed = parse_encrypt_dict(&dict).unwrap();
        assert_eq!(ed.revision, 4);
        assert_eq!(ed.key_length, 16); // 128 bits = 16 bytes
        assert_eq!(ed.cipher, Cipher::Aes128);
    }

    #[test]
    fn parse_aes256_r6_encrypt_dict() {
        let dict = make_aes256_r6_encrypt_dict();
        let ed = parse_encrypt_dict(&dict).unwrap();
        assert_eq!(ed.revision, 6);
        assert_eq!(ed.key_length, 32); // 256 bits = 32 bytes
        assert_eq!(ed.cipher, Cipher::Aes256);
        assert!(ed.owner_encrypted_key.is_some());
        assert!(ed.user_encrypted_key.is_some());
        assert!(ed.encrypted_perms.is_some());
    }

    #[test]
    fn parse_encrypt_dict_missing_filter_is_error() {
        let dict = PdfDictionary::new(); // no /Filter
        assert!(parse_encrypt_dict(&dict).is_err());
    }

    #[test]
    fn parse_encrypt_dict_encrypt_metadata_false() {
        let mut dict = make_rc4_r2_encrypt_dict();
        dict.set("EncryptMetadata", PdfObject::Boolean(false));
        let ed = parse_encrypt_dict(&dict).unwrap();
        assert!(!ed.encrypt_metadata);
    }

    #[test]
    fn extract_file_id_from_trailer() {
        let mut trailer = PdfDictionary::new();
        let id_bytes = vec![0x01u8, 0x02, 0x03, 0x04];
        trailer.set(
            "ID",
            PdfObject::Array(vec![
                PdfObject::String(PdfByteString::from(id_bytes.clone())),
                PdfObject::String(PdfByteString::from(vec![0xAA, 0xBB])),
            ]),
        );
        let file_id = extract_file_id(&trailer);
        assert_eq!(file_id, id_bytes);
    }

    #[test]
    fn extract_file_id_missing_returns_empty() {
        let trailer = PdfDictionary::new();
        let file_id = extract_file_id(&trailer);
        assert!(file_id.is_empty());
    }

    #[test]
    fn determine_cipher_v5_non_aesv3_is_error() {
        // V=5 without AESV3 CFM should be an error
        let mut dict = PdfDictionary::new();
        dict.set("Filter", PdfObject::Name(PdfByteString::from("Standard")));
        dict.set("V", PdfObject::Integer(5));
        dict.set("R", PdfObject::Integer(6));
        dict.set("Length", PdfObject::Integer(256));
        dict.set("P", PdfObject::Integer(-4));
        dict.set("O", PdfObject::String(PdfByteString::from(vec![0u8; 48])));
        dict.set("U", PdfObject::String(PdfByteString::from(vec![0u8; 48])));
        dict.set("OE", PdfObject::String(PdfByteString::from(vec![0u8; 32])));
        dict.set("UE", PdfObject::String(PdfByteString::from(vec![0u8; 32])));
        dict.set(
            "Perms",
            PdfObject::String(PdfByteString::from(vec![0u8; 16])),
        );

        // Set CFM to AESV2 (wrong for V=5)
        let mut stdcf = PdfDictionary::new();
        stdcf.set("CFM", PdfObject::Name(PdfByteString::from("AESV2")));
        let mut cf = PdfDictionary::new();
        cf.set("StdCF", PdfObject::Dictionary(stdcf));
        dict.set("CF", PdfObject::Dictionary(cf));

        let result = parse_encrypt_dict(&dict);
        assert!(result.is_err());
    }

    #[test]
    fn determine_cipher_unknown_v_is_error() {
        // Unknown /V value should be an error
        let mut dict = PdfDictionary::new();
        dict.set("Filter", PdfObject::Name(PdfByteString::from("Standard")));
        dict.set("V", PdfObject::Integer(99));
        dict.set("R", PdfObject::Integer(2));
        dict.set("Length", PdfObject::Integer(40));
        dict.set("P", PdfObject::Integer(-4));
        dict.set("O", PdfObject::String(PdfByteString::from(vec![0u8; 32])));
        dict.set("U", PdfObject::String(PdfByteString::from(vec![0u8; 32])));

        let result = parse_encrypt_dict(&dict);
        assert!(result.is_err());
    }
}
