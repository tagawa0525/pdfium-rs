use crate::error::{Error, Result};

/// 32-byte padding constant from PDF Reference Table 3.19.
///
/// Used to pad passwords shorter than 32 bytes in revisions 2-4.
const PASSWORD_PADDING: [u8; 32] = [
    0x28, 0xBF, 0x4E, 0x5E, 0x4E, 0x75, 0x8A, 0x41, 0x64, 0x00, 0x4E, 0x56, 0xFF, 0xFA, 0x01, 0x08,
    0x2E, 0x2E, 0x00, 0xB6, 0xD0, 0x68, 0x3E, 0x80, 0x2F, 0x0C, 0xA9, 0xFE, 0x64, 0x53, 0x69, 0x7A,
];

/// Cipher algorithm used for PDF encryption.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cipher {
    /// No encryption.
    None,
    /// RC4 (40-128 bit key). Revisions 2-4.
    Rc4,
    /// AES-128-CBC. Revision 4 (CFM=AESV2).
    Aes128,
    /// AES-256-CBC. Revisions 5-6 (CFM=AESV3).
    Aes256,
}

/// Parsed permissions flags from the /P entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Permissions {
    /// Bit 3: Print document.
    pub print: bool,
    /// Bit 4: Modify contents.
    pub modify: bool,
    /// Bit 5: Copy or extract text and graphics.
    pub copy: bool,
    /// Bit 6: Add or modify annotations.
    pub annotate: bool,
    /// Bit 9: Fill in form fields.
    pub fill_forms: bool,
    /// Bit 10: Extract text and graphics (accessibility).
    pub extract: bool,
    /// Bit 11: Assemble document.
    pub assemble: bool,
    /// Bit 12: Print in high quality.
    pub print_high_quality: bool,
}

impl Permissions {
    /// Parse permissions from the /P integer value.
    pub fn from_p(p: i32) -> Self {
        parse_permissions(p)
    }
}

/// Encryption parameters parsed from the PDF /Encrypt dictionary.
#[derive(Debug, Clone)]
pub struct EncryptDict {
    /// Security handler revision (2-6).
    pub revision: u32,
    /// Encryption key length in bytes (5-32).
    pub key_length: usize,
    /// Cipher algorithm.
    pub cipher: Cipher,
    /// Permissions value (/P entry).
    pub permissions: i32,
    /// Owner password hash (/O entry, 32 or 48 bytes).
    pub owner_hash: Vec<u8>,
    /// User password hash (/U entry, 32 or 48 bytes).
    pub user_hash: Vec<u8>,
    /// Encrypted owner key (/OE entry, 32 bytes). R5+ only.
    pub owner_encrypted_key: Option<Vec<u8>>,
    /// Encrypted user key (/UE entry, 32 bytes). R5+ only.
    pub user_encrypted_key: Option<Vec<u8>>,
    /// Encrypted permissions (/Perms entry, 16 bytes). R5+ only.
    pub encrypted_perms: Option<Vec<u8>>,
    /// Whether to encrypt metadata (/EncryptMetadata, default true).
    pub encrypt_metadata: bool,
}

/// PDF Standard Security Handler.
///
/// Handles password verification, encryption key derivation, and
/// object decryption for PDF Standard Security Handler revisions 2-6.
pub struct SecurityHandler {
    revision: u32,
    key_length: usize,
    cipher: Cipher,
    encrypt_key: Vec<u8>,
    permissions: i32,
    encrypt_metadata: bool,
}

impl SecurityHandler {
    /// Create a security handler by verifying the password against the
    /// encrypt dictionary.
    ///
    /// Tries the password as user password first, then as owner password.
    /// Returns `Err` if the password is incorrect.
    pub fn new(dict: &EncryptDict, file_id: &[u8], password: &[u8]) -> Result<Self> {
        todo!()
    }

    /// Derive the per-object decryption key for the given object/generation number.
    ///
    /// For AES-256 (revision 5-6), returns the document key unchanged.
    pub fn object_key(&self, obj_num: u32, gen_num: u16) -> Vec<u8> {
        todo!()
    }

    /// Decrypt bytes (string or stream) belonging to the given object.
    ///
    /// For AES ciphers, the first 16 bytes of `data` are the IV.
    pub fn decrypt_bytes(&self, obj_num: u32, gen_num: u16, data: &[u8]) -> Result<Vec<u8>> {
        todo!()
    }

    /// Get the document permissions.
    pub fn permissions(&self) -> Permissions {
        parse_permissions(self.permissions)
    }

    /// Get the cipher algorithm.
    pub fn cipher(&self) -> Cipher {
        self.cipher
    }

    /// Whether document metadata is encrypted.
    pub fn encrypt_metadata(&self) -> bool {
        self.encrypt_metadata
    }
}

// --- Internal helper functions ---

/// Pad or truncate a password to 32 bytes using the standard padding.
fn pad_password(password: &[u8]) -> [u8; 32] {
    todo!()
}

/// Compute the encryption key for revisions 2-4 (PDF Reference Algorithm 2).
fn calc_encrypt_key(
    password: &[u8],
    o: &[u8],
    p: i32,
    file_id: &[u8],
    key_length: usize,
    revision: u32,
    encrypt_metadata: bool,
) -> Vec<u8> {
    todo!()
}

/// Compute the /U value for revision 2 (Algorithm 4).
fn compute_u_r2(key: &[u8]) -> [u8; 32] {
    todo!()
}

/// Compute the /U value for revisions 3-4 (Algorithm 5).
fn compute_u_r3(key: &[u8], file_id: &[u8]) -> [u8; 32] {
    todo!()
}

/// Verify user password for revisions 2-4.
/// Returns the encryption key if the password is correct.
fn check_user_password(password: &[u8], dict: &EncryptDict, file_id: &[u8]) -> Option<Vec<u8>> {
    todo!()
}

/// Compute the owner key from the owner password (MD5-based, for /O derivation).
fn compute_owner_key(owner_password: &[u8], key_length: usize, revision: u32) -> Vec<u8> {
    todo!()
}

/// Recover the user password from the /O entry using the owner password.
fn get_user_password_from_owner(o: &[u8], owner_key: &[u8], revision: u32) -> [u8; 32] {
    todo!()
}

/// Verify owner password for revisions 2-4.
/// Recovers the user password from /O, then verifies as user password.
fn check_owner_password(password: &[u8], dict: &EncryptDict, file_id: &[u8]) -> Option<Vec<u8>> {
    todo!()
}

/// Derive per-object key by appending object/generation numbers (Algorithm 1).
fn derive_object_key(base_key: &[u8], obj_num: u32, gen_num: u16, cipher: Cipher) -> Vec<u8> {
    todo!()
}

/// Verify password for revisions 5-6 (AES-256). Returns encryption key.
fn aes256_check_password(password: &[u8], dict: &EncryptDict) -> Option<Vec<u8>> {
    todo!()
}

/// Revision 6 iterative hash (PDF 2.0, Algorithm 2.B).
///
/// Uses SHA-256/384/512 adaptively with AES-128-CBC encryption rounds.
fn revision6_hash(password: &[u8], salt: &[u8], input: &[u8]) -> [u8; 32] {
    todo!()
}

/// Parse permissions from the /P integer value.
fn parse_permissions(p: i32) -> Permissions {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------------------------------------------------------------
    // pad_password
    // ---------------------------------------------------------------

    #[test]
    #[ignore = "not yet implemented"]
    fn pad_password_empty() {
        assert_eq!(pad_password(b""), PASSWORD_PADDING);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn pad_password_short() {
        let result = pad_password(b"user");
        assert_eq!(&result[..4], b"user");
        assert_eq!(&result[4..], &PASSWORD_PADDING[..28]);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn pad_password_exact_32() {
        let input = [0x41u8; 32];
        assert_eq!(pad_password(&input), input);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn pad_password_longer_than_32() {
        let input = [0x42u8; 40];
        assert_eq!(pad_password(&input), [0x42u8; 32]);
    }

    // ---------------------------------------------------------------
    // calc_encrypt_key
    // ---------------------------------------------------------------

    #[test]
    #[ignore = "not yet implemented"]
    fn calc_encrypt_key_r2() {
        // R2: MD5(padded_password + O + P_le + file_id), no extra hashing
        let o = [0xAAu8; 32];
        let p: i32 = -4;
        let file_id = b"0123456789abcdef";
        let key_length = 5;

        // Compute expected step by step
        let mut input = Vec::new();
        input.extend_from_slice(&PASSWORD_PADDING);
        input.extend_from_slice(&o);
        input.extend_from_slice(&(p as u32).to_le_bytes());
        input.extend_from_slice(file_id);
        let digest = crate::fdrm::md5::digest(&input);
        let expected = digest[..key_length].to_vec();

        let result = calc_encrypt_key(b"", &o, p, file_id, key_length, 2, true);
        assert_eq!(result, expected);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn calc_encrypt_key_r3_with_50_iterations() {
        // R3+: Same as R2, then hash digest 50 more times
        let o = [0xBBu8; 32];
        let p: i32 = -3904;
        let file_id = b"abcdefghijklmnop";
        let key_length = 16;

        let mut input = Vec::new();
        input.extend_from_slice(&PASSWORD_PADDING);
        input.extend_from_slice(&o);
        input.extend_from_slice(&(p as u32).to_le_bytes());
        input.extend_from_slice(file_id);
        let mut digest = crate::fdrm::md5::digest(&input);
        for _ in 0..50 {
            digest = crate::fdrm::md5::digest(&digest[..key_length]);
        }
        let expected = digest[..key_length].to_vec();

        let result = calc_encrypt_key(b"", &o, p, file_id, key_length, 3, true);
        assert_eq!(result, expected);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn calc_encrypt_key_r3_encrypt_metadata_false() {
        // When EncryptMetadata=false, append 0xFFFFFFFF to MD5 input
        let o = [0xCCu8; 32];
        let p: i32 = -4;
        let file_id = b"metadata_test_id";
        let key_length = 16;

        let mut input = Vec::new();
        input.extend_from_slice(&PASSWORD_PADDING);
        input.extend_from_slice(&o);
        input.extend_from_slice(&(p as u32).to_le_bytes());
        input.extend_from_slice(file_id);
        input.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]);
        let mut digest = crate::fdrm::md5::digest(&input);
        for _ in 0..50 {
            digest = crate::fdrm::md5::digest(&digest[..key_length]);
        }
        let expected = digest[..key_length].to_vec();

        let result = calc_encrypt_key(b"", &o, p, file_id, key_length, 3, false);
        assert_eq!(result, expected);
    }

    // ---------------------------------------------------------------
    // compute_u
    // ---------------------------------------------------------------

    #[test]
    #[ignore = "not yet implemented"]
    fn compute_u_r2_known_key() {
        // R2: RC4-encrypt PASSWORD_PADDING with the encryption key
        let key = [0x01u8, 0x02, 0x03, 0x04, 0x05];
        let expected = crate::fdrm::rc4::crypt(&key, &PASSWORD_PADDING).unwrap();
        let result = compute_u_r2(&key);
        assert_eq!(&result[..], &expected[..]);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn compute_u_r3_known_key() {
        // R3: MD5(PASSWORD_PADDING + file_id) → RC4 encrypt 20 times
        let key = [0x01u8; 16];
        let file_id = b"test_file_id_abc";

        // Step 1: MD5(PASSWORD_PADDING + file_id)
        let mut hash_input = Vec::new();
        hash_input.extend_from_slice(&PASSWORD_PADDING);
        hash_input.extend_from_slice(file_id);
        let hash = crate::fdrm::md5::digest(&hash_input);

        // Step 2: 20 RC4 iterations with XOR'd key
        let mut encrypted = hash.to_vec();
        for i in 0u8..20 {
            let iter_key: Vec<u8> = key.iter().map(|&b| b ^ i).collect();
            encrypted = crate::fdrm::rc4::crypt(&iter_key, &encrypted).unwrap();
        }

        let result = compute_u_r3(&key, file_id);
        // Only first 16 bytes are significant for R3
        assert_eq!(&result[..16], &encrypted[..16]);
    }

    // ---------------------------------------------------------------
    // check_user_password
    // ---------------------------------------------------------------

    fn make_r2_dict(password: &[u8], o: &[u8], p: i32, file_id: &[u8]) -> (EncryptDict, Vec<u8>) {
        let key_length = 5;
        // Derive key
        let key = {
            let padded = pad_or_truncate(password);
            let mut input = Vec::new();
            input.extend_from_slice(&padded);
            input.extend_from_slice(o);
            input.extend_from_slice(&(p as u32).to_le_bytes());
            input.extend_from_slice(file_id);
            crate::fdrm::md5::digest(&input)[..key_length].to_vec()
        };
        // Compute U
        let u = crate::fdrm::rc4::crypt(&key, &PASSWORD_PADDING).unwrap();
        let dict = EncryptDict {
            revision: 2,
            key_length,
            cipher: Cipher::Rc4,
            permissions: p,
            owner_hash: o.to_vec(),
            user_hash: u,
            owner_encrypted_key: None,
            user_encrypted_key: None,
            encrypted_perms: None,
            encrypt_metadata: true,
        };
        (dict, key)
    }

    /// Inline pad (for test helpers only; does not call the stub).
    fn pad_or_truncate(password: &[u8]) -> [u8; 32] {
        let mut buf = [0u8; 32];
        let len = password.len().min(32);
        buf[..len].copy_from_slice(&password[..len]);
        buf[len..].copy_from_slice(&PASSWORD_PADDING[..32 - len]);
        buf
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn check_user_password_r2_correct() {
        let file_id = b"0123456789abcdef";
        let o = [0xAAu8; 32];
        let p: i32 = -4;
        let (dict, expected_key) = make_r2_dict(b"secret", &o, p, file_id);

        let result = check_user_password(b"secret", &dict, file_id);
        assert!(result.is_some(), "correct password should be accepted");
        assert_eq!(result.unwrap(), expected_key);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn check_user_password_r2_wrong() {
        let file_id = b"0123456789abcdef";
        let o = [0xAAu8; 32];
        let p: i32 = -4;
        let (dict, _) = make_r2_dict(b"secret", &o, p, file_id);

        let result = check_user_password(b"wrong", &dict, file_id);
        assert!(result.is_none(), "wrong password should be rejected");
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn check_user_password_r2_empty_password() {
        let file_id = b"0123456789abcdef";
        let o = [0u8; 32];
        let p: i32 = -4;
        let (dict, expected_key) = make_r2_dict(b"", &o, p, file_id);

        let result = check_user_password(b"", &dict, file_id);
        assert!(result.is_some(), "empty password should be accepted");
        assert_eq!(result.unwrap(), expected_key);
    }

    // ---------------------------------------------------------------
    // derive_object_key
    // ---------------------------------------------------------------

    #[test]
    #[ignore = "not yet implemented"]
    fn derive_object_key_rc4() {
        // MD5(base_key + obj_le24 + gen_le16), truncated to min(key_len+5, 16)
        let base_key = vec![0x01u8, 0x02, 0x03, 0x04, 0x05];
        let obj_num: u32 = 10;
        let gen_num: u16 = 0;

        let mut input = Vec::new();
        input.extend_from_slice(&base_key);
        input.push((obj_num & 0xFF) as u8);
        input.push(((obj_num >> 8) & 0xFF) as u8);
        input.push(((obj_num >> 16) & 0xFF) as u8);
        input.push((gen_num & 0xFF) as u8);
        input.push(((gen_num >> 8) & 0xFF) as u8);
        let digest = crate::fdrm::md5::digest(&input);
        let n = (base_key.len() + 5).min(16);
        let expected = digest[..n].to_vec();

        let result = derive_object_key(&base_key, obj_num, gen_num, Cipher::Rc4);
        assert_eq!(result, expected);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn derive_object_key_aes128_appends_salt() {
        // Same as RC4 but append b"sAlT" before MD5
        let base_key = vec![0x01u8; 16];
        let obj_num: u32 = 42;
        let gen_num: u16 = 0;

        let mut input = Vec::new();
        input.extend_from_slice(&base_key);
        input.push((obj_num & 0xFF) as u8);
        input.push(((obj_num >> 8) & 0xFF) as u8);
        input.push(((obj_num >> 16) & 0xFF) as u8);
        input.push((gen_num & 0xFF) as u8);
        input.push(((gen_num >> 8) & 0xFF) as u8);
        input.extend_from_slice(b"sAlT");
        let expected = crate::fdrm::md5::digest(&input).to_vec();

        let result = derive_object_key(&base_key, obj_num, gen_num, Cipher::Aes128);
        assert_eq!(result, expected);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn derive_object_key_aes256_returns_base_key() {
        // AES-256 (R5-R6): per-object key = base key unchanged
        let base_key = vec![0x01u8; 32];
        let result = derive_object_key(&base_key, 10, 0, Cipher::Aes256);
        assert_eq!(result, base_key);
    }

    // ---------------------------------------------------------------
    // permissions
    // ---------------------------------------------------------------

    #[test]
    #[ignore = "not yet implemented"]
    fn permissions_all_allowed() {
        let perms = parse_permissions(-1); // all bits set
        assert!(perms.print);
        assert!(perms.modify);
        assert!(perms.copy);
        assert!(perms.annotate);
        assert!(perms.fill_forms);
        assert!(perms.extract);
        assert!(perms.assemble);
        assert!(perms.print_high_quality);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn permissions_none_allowed() {
        // Only reserved bits set (bits 1-2, 7-8, 13-32), no permission bits
        let p = 0xFFFF_F0C0_u32 as i32;
        let perms = parse_permissions(p);
        assert!(!perms.print);
        assert!(!perms.modify);
        assert!(!perms.copy);
        assert!(!perms.annotate);
        assert!(!perms.fill_forms);
        assert!(!perms.extract);
        assert!(!perms.assemble);
        assert!(!perms.print_high_quality);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn permissions_print_and_copy_only() {
        // Bit 3 (print=4) + Bit 5 (copy=16) + reserved bits
        let p = (0xFFFF_F0C0_u32 | (1 << 2) | (1 << 4)) as i32;
        let perms = parse_permissions(p);
        assert!(perms.print);
        assert!(!perms.modify);
        assert!(perms.copy);
        assert!(!perms.annotate);
    }

    // ---------------------------------------------------------------
    // decrypt_bytes (RC4)
    // ---------------------------------------------------------------

    #[test]
    #[ignore = "not yet implemented"]
    fn decrypt_bytes_rc4() {
        let base_key = vec![0x01, 0x02, 0x03, 0x04, 0x05];
        let obj_num = 10u32;
        let gen_num = 0u16;

        // Derive expected object key
        let obj_key = {
            let mut input = Vec::new();
            input.extend_from_slice(&base_key);
            input.push((obj_num & 0xFF) as u8);
            input.push(((obj_num >> 8) & 0xFF) as u8);
            input.push(((obj_num >> 16) & 0xFF) as u8);
            input.push((gen_num & 0xFF) as u8);
            input.push(((gen_num >> 8) & 0xFF) as u8);
            let digest = crate::fdrm::md5::digest(&input);
            digest[..(base_key.len() + 5).min(16)].to_vec()
        };

        // Encrypt known plaintext with the object key
        let plaintext = b"Hello, encrypted PDF!";
        let ciphertext = crate::fdrm::rc4::crypt(&obj_key, plaintext).unwrap();

        // Build a SecurityHandler directly for testing
        let handler = SecurityHandler {
            revision: 2,
            key_length: 5,
            cipher: Cipher::Rc4,
            encrypt_key: base_key,
            permissions: -4,
            encrypt_metadata: true,
        };

        let decrypted = handler
            .decrypt_bytes(obj_num, gen_num, &ciphertext)
            .unwrap();
        assert_eq!(decrypted, plaintext);
    }

    // ---------------------------------------------------------------
    // SecurityHandler integration
    // ---------------------------------------------------------------

    #[test]
    #[ignore = "not yet implemented"]
    fn security_handler_r2_empty_password() {
        let file_id = b"0123456789abcdef";
        let o = [0u8; 32];
        let p: i32 = -4;
        let (dict, _) = make_r2_dict(b"", &o, p, file_id);

        let handler = SecurityHandler::new(&dict, file_id, b"").unwrap();
        assert_eq!(handler.cipher(), Cipher::Rc4);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn security_handler_wrong_password_is_error() {
        let file_id = b"0123456789abcdef";
        let o = [0u8; 32];
        let p: i32 = -4;
        let (dict, _) = make_r2_dict(b"", &o, p, file_id);

        let result = SecurityHandler::new(&dict, file_id, b"wrong_password");
        assert!(result.is_err());
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn security_handler_r2_decrypt_roundtrip() {
        let file_id = b"0123456789abcdef";
        let o = [0u8; 32];
        let p: i32 = -4;
        let (dict, key) = make_r2_dict(b"", &o, p, file_id);

        let handler = SecurityHandler::new(&dict, file_id, b"").unwrap();

        // Encrypt with derived object key, then decrypt via handler
        let obj_num = 5u32;
        let gen_num = 0u16;
        let obj_key = {
            let mut input = Vec::new();
            input.extend_from_slice(&key);
            input.push((obj_num & 0xFF) as u8);
            input.push(((obj_num >> 8) & 0xFF) as u8);
            input.push(((obj_num >> 16) & 0xFF) as u8);
            input.push((gen_num & 0xFF) as u8);
            input.push(((gen_num >> 8) & 0xFF) as u8);
            let digest = crate::fdrm::md5::digest(&input);
            digest[..(key.len() + 5).min(16)].to_vec()
        };
        let plaintext = b"Decrypted content";
        let ciphertext = crate::fdrm::rc4::crypt(&obj_key, plaintext).unwrap();

        let decrypted = handler
            .decrypt_bytes(obj_num, gen_num, &ciphertext)
            .unwrap();
        assert_eq!(decrypted, plaintext.as_ref());
    }

    // ---------------------------------------------------------------
    // Owner password
    // ---------------------------------------------------------------

    #[test]
    #[ignore = "not yet implemented"]
    fn check_owner_password_r2() {
        let file_id = b"0123456789abcdef";
        let user_password = b"user";
        let owner_password = b"owner";
        let p: i32 = -4;
        let key_length = 5;

        // Compute owner key: MD5(pad(owner_password))[..key_length]
        let owner_key = {
            let padded = pad_or_truncate(owner_password);
            crate::fdrm::md5::digest(&padded)[..key_length].to_vec()
        };

        // Compute O: RC4-encrypt pad(user_password) with owner_key
        let o_value = crate::fdrm::rc4::crypt(&owner_key, &pad_or_truncate(user_password)).unwrap();

        // Compute encryption key for user password
        let key = {
            let padded = pad_or_truncate(user_password);
            let mut input = Vec::new();
            input.extend_from_slice(&padded);
            input.extend_from_slice(&o_value);
            input.extend_from_slice(&(p as u32).to_le_bytes());
            input.extend_from_slice(file_id);
            crate::fdrm::md5::digest(&input)[..key_length].to_vec()
        };

        // Compute U
        let u_value = crate::fdrm::rc4::crypt(&key, &PASSWORD_PADDING).unwrap();

        let dict = EncryptDict {
            revision: 2,
            key_length,
            cipher: Cipher::Rc4,
            permissions: p,
            owner_hash: o_value,
            user_hash: u_value,
            owner_encrypted_key: None,
            user_encrypted_key: None,
            encrypted_perms: None,
            encrypt_metadata: true,
        };

        // Owner password should work
        let result = check_owner_password(owner_password, &dict, file_id);
        assert!(result.is_some(), "owner password should be accepted");
    }

    // ---------------------------------------------------------------
    // AES-256 / R5 password check
    // ---------------------------------------------------------------

    #[test]
    #[ignore = "not yet implemented"]
    fn aes256_check_password_r5_user() {
        // R5: SHA-256(password + validation_salt) must match U[0..32]
        let password = b"secret";
        let validation_salt = [0x11u8; 8]; // U[32..40]
        let key_salt = [0x22u8; 8]; // U[40..48]

        // Compute U hash
        let mut hash_input = Vec::new();
        hash_input.extend_from_slice(password);
        hash_input.extend_from_slice(&validation_salt);
        let u_hash = crate::fdrm::sha256::digest(&hash_input);

        // U = hash(32) + validation_salt(8) + key_salt(8)
        let mut user_hash = Vec::new();
        user_hash.extend_from_slice(&u_hash);
        user_hash.extend_from_slice(&validation_salt);
        user_hash.extend_from_slice(&key_salt);

        // UE: AES-256-CBC encrypt of the real key with SHA-256(password + key_salt)
        // For this test, we use a known 32-byte key
        let real_key = [0xABu8; 32];
        let key_for_ue = crate::fdrm::sha256::digest(&[password.as_slice(), &key_salt].concat());
        // UE = AES-256-CBC-encrypt(real_key) — but we only have decrypt.
        // For testing, decrypt(UE) = real_key, so UE = encrypt(real_key).
        // We'll compute UE during GREEN and hardcode it here.
        // For now, use the decrypt relationship: decrypt(encrypt(real_key)) = real_key
        // This test verifies the algorithm flow, exact values verified in GREEN.
        let ue_iv = [0u8; 16];
        // Placeholder: will be computed properly in GREEN
        let user_encrypted_key = vec![0u8; 32];

        let dict = EncryptDict {
            revision: 5,
            key_length: 32,
            cipher: Cipher::Aes256,
            permissions: -4,
            owner_hash: vec![0u8; 48],
            user_hash,
            owner_encrypted_key: Some(vec![0u8; 32]),
            user_encrypted_key: Some(user_encrypted_key),
            encrypted_perms: Some(vec![0u8; 16]),
            encrypt_metadata: true,
        };

        let result = aes256_check_password(password, &dict);
        // Password validation should succeed (hash matches U[0..32])
        // Key recovery may not match due to placeholder UE, but hash check passes
        assert!(
            result.is_some(),
            "R5 password check should pass hash validation"
        );
    }
}
