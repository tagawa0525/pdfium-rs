use crate::error::{Error, Result};
use crate::fdrm::{aes, md5, rc4, sha256};

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
        let encrypt_key = check_password_with_encoding(password, dict, file_id)
            .ok_or_else(|| Error::InvalidPdf("incorrect password".into()))?;
        Ok(SecurityHandler {
            cipher: dict.cipher,
            encrypt_key,
            permissions: dict.permissions,
            encrypt_metadata: dict.encrypt_metadata,
        })
    }

    /// Derive the per-object decryption key for the given object/generation number.
    ///
    /// For AES-256 (revision 5-6), returns the document key unchanged.
    pub fn object_key(&self, obj_num: u32, gen_num: u16) -> Vec<u8> {
        derive_object_key(&self.encrypt_key, obj_num, gen_num, self.cipher)
    }

    /// Decrypt bytes (string or stream) belonging to the given object.
    ///
    /// For AES ciphers, the first 16 bytes of `data` are the IV.
    pub fn decrypt_bytes(&self, obj_num: u32, gen_num: u16, data: &[u8]) -> Result<Vec<u8>> {
        let key = self.object_key(obj_num, gen_num);
        match self.cipher {
            Cipher::None => Ok(data.to_vec()),
            Cipher::Rc4 => rc4::crypt(&key, data),
            Cipher::Aes128 | Cipher::Aes256 => {
                if data.len() < 16 {
                    return Err(Error::InvalidPdf(
                        "AES encrypted data must be at least 16 bytes (IV)".into(),
                    ));
                }
                let (iv, ciphertext) = data.split_at(16);
                if self.cipher == Cipher::Aes256 {
                    aes::decrypt_aes256_cbc(&key, iv, ciphertext)
                } else {
                    aes::decrypt_aes128_cbc(&key, iv, ciphertext)
                }
            }
        }
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

/// Try password as-is, then fall back to encoding conversion if non-ASCII.
///
/// For Rev 2-4: passwords use PDFDocEncoding (Latin-1). If the as-is check
/// fails and the password is non-ASCII, try converting UTF-8 → Latin-1.
/// For Rev 5+: passwords use UTF-8. If the as-is check fails and the
/// password is non-ASCII, try converting Latin-1 → UTF-8.
fn check_password_with_encoding(
    password: &[u8],
    dict: &EncryptDict,
    file_id: &[u8],
) -> Option<Vec<u8>> {
    // Try password as-is
    let result = check_password_impl(password, dict, file_id);
    if result.is_some() {
        return result;
    }

    // Only attempt conversion for non-ASCII passwords
    if password.iter().all(|&b| b < 0x80) {
        return None;
    }

    if dict.revision >= 5 {
        // Latin-1 → UTF-8
        let utf8 = latin1_to_utf8(password);
        check_password_impl(&utf8, dict, file_id)
    } else {
        // UTF-8 → Latin-1
        if let Some(latin1) = utf8_to_latin1(password) {
            check_password_impl(&latin1, dict, file_id)
        } else {
            None
        }
    }
}

/// Core password check without encoding conversion.
fn check_password_impl(password: &[u8], dict: &EncryptDict, file_id: &[u8]) -> Option<Vec<u8>> {
    if dict.revision >= 5 {
        aes256_check_password(password, dict)
    } else {
        check_user_password(password, dict, file_id)
            .or_else(|| check_owner_password(password, dict, file_id))
    }
}

/// Convert Latin-1 bytes to UTF-8.
fn latin1_to_utf8(input: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(input.len() * 2);
    for &b in input {
        if b < 0x80 {
            out.push(b);
        } else {
            // Latin-1 byte 0x80-0xFF maps to Unicode codepoint of same value
            out.push(0xC0 | (b >> 6));
            out.push(0x80 | (b & 0x3F));
        }
    }
    out
}

/// Convert UTF-8 bytes to Latin-1. Returns None if any codepoint > U+00FF.
fn utf8_to_latin1(input: &[u8]) -> Option<Vec<u8>> {
    let s = core::str::from_utf8(input).ok()?;
    let mut out = Vec::with_capacity(s.len());
    for ch in s.chars() {
        if ch as u32 > 0xFF {
            return None;
        }
        out.push(ch as u8);
    }
    Some(out)
}

/// Pad or truncate a password to 32 bytes using the standard padding.
fn pad_password(password: &[u8]) -> [u8; 32] {
    let mut buf = [0u8; 32];
    let len = password.len().min(32);
    buf[..len].copy_from_slice(&password[..len]);
    if len < 32 {
        buf[len..].copy_from_slice(&PASSWORD_PADDING[..32 - len]);
    }
    buf
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
    // MD5 output is 16 bytes; PDF spec allows key_length 5-16.
    // Clamp to prevent out-of-bounds access on the digest.
    let key_length = key_length.clamp(1, 16);

    let padded = pad_password(password);
    let mut input = Vec::with_capacity(32 + o.len() + 4 + file_id.len() + 4);
    input.extend_from_slice(&padded);
    input.extend_from_slice(o);
    input.extend_from_slice(&(p as u32).to_le_bytes());
    input.extend_from_slice(file_id);
    if revision >= 3 && !encrypt_metadata {
        input.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]);
    }
    let mut digest = md5::digest(&input);
    if revision >= 3 {
        for _ in 0..50 {
            digest = md5::digest(&digest[..key_length]);
        }
    }
    digest[..key_length].to_vec()
}

/// Compute the /U value for revision 2 (Algorithm 4).
fn compute_u_r2(key: &[u8]) -> [u8; 32] {
    let encrypted = rc4::crypt(key, &PASSWORD_PADDING).expect("key must not be empty");
    let mut result = [0u8; 32];
    result.copy_from_slice(&encrypted);
    result
}

/// Compute the /U value for revisions 3-4 (Algorithm 5).
fn compute_u_r3(key: &[u8], file_id: &[u8]) -> [u8; 32] {
    // MD5(PASSWORD_PADDING + file_id)
    let mut hash_input = Vec::with_capacity(32 + file_id.len());
    hash_input.extend_from_slice(&PASSWORD_PADDING);
    hash_input.extend_from_slice(file_id);
    let hash = md5::digest(&hash_input);

    // 20 RC4 iterations with XOR'd key
    let mut encrypted = hash.to_vec();
    for i in 0u8..20 {
        let iter_key: Vec<u8> = key.iter().map(|&b| b ^ i).collect();
        encrypted = rc4::crypt(&iter_key, &encrypted).expect("key must not be empty");
    }

    // First 16 bytes are significant, pad remaining with arbitrary bytes
    let mut result = [0u8; 32];
    result[..16].copy_from_slice(&encrypted[..16]);
    result
}

/// Verify user password for revisions 2-4.
/// Returns the encryption key if the password is correct.
fn check_user_password(password: &[u8], dict: &EncryptDict, file_id: &[u8]) -> Option<Vec<u8>> {
    let key = calc_encrypt_key(
        password,
        &dict.owner_hash,
        dict.permissions,
        file_id,
        dict.key_length,
        dict.revision,
        dict.encrypt_metadata,
    );
    if dict.revision == 2 {
        let computed_u = compute_u_r2(&key);
        if computed_u == dict.user_hash.as_slice() {
            return Some(key);
        }
    } else {
        if dict.user_hash.len() < 16 {
            return None;
        }
        let computed_u = compute_u_r3(&key, file_id);
        // Only compare first 16 bytes for R3+
        if computed_u[..16] == dict.user_hash[..16] {
            return Some(key);
        }
    }
    None
}

/// Compute the owner key from the owner password (MD5-based, for /O derivation).
fn compute_owner_key(owner_password: &[u8], key_length: usize, revision: u32) -> Vec<u8> {
    let padded = pad_password(owner_password);
    let mut digest = md5::digest(&padded);
    if revision >= 3 {
        for _ in 0..50 {
            digest = md5::digest(&digest);
        }
    }
    digest[..key_length].to_vec()
}

/// Recover the user password from the /O entry using the owner password.
fn get_user_password_from_owner(o: &[u8], owner_key: &[u8], revision: u32) -> [u8; 32] {
    let mut decrypted = o.to_vec();
    if revision == 2 {
        decrypted = rc4::crypt(owner_key, &decrypted).expect("key must not be empty");
    } else {
        // R3+: 20 RC4 iterations in reverse (19..=0)
        for i in (0u8..20).rev() {
            let iter_key: Vec<u8> = owner_key.iter().map(|&b| b ^ i).collect();
            decrypted = rc4::crypt(&iter_key, &decrypted).expect("key must not be empty");
        }
    }
    let mut result = [0u8; 32];
    let len = decrypted.len().min(32);
    result[..len].copy_from_slice(&decrypted[..len]);
    result
}

/// Verify owner password for revisions 2-4.
/// Recovers the user password from /O, then verifies as user password.
fn check_owner_password(password: &[u8], dict: &EncryptDict, file_id: &[u8]) -> Option<Vec<u8>> {
    let owner_key = compute_owner_key(password, dict.key_length, dict.revision);
    let user_password = get_user_password_from_owner(&dict.owner_hash, &owner_key, dict.revision);
    check_user_password(&user_password, dict, file_id)
}

/// Derive per-object key by appending object/generation numbers (Algorithm 1).
fn derive_object_key(base_key: &[u8], obj_num: u32, gen_num: u16, cipher: Cipher) -> Vec<u8> {
    if cipher == Cipher::Aes256 {
        return base_key.to_vec();
    }
    let mut input = Vec::with_capacity(base_key.len() + 5 + 4);
    input.extend_from_slice(base_key);
    input.push((obj_num & 0xFF) as u8);
    input.push(((obj_num >> 8) & 0xFF) as u8);
    input.push(((obj_num >> 16) & 0xFF) as u8);
    input.push((gen_num & 0xFF) as u8);
    input.push(((gen_num >> 8) & 0xFF) as u8);
    if cipher == Cipher::Aes128 {
        input.extend_from_slice(b"sAlT");
    }
    let digest = md5::digest(&input);
    let n = (base_key.len() + 5).min(16);
    digest[..n].to_vec()
}

/// Verify password for revisions 5-6 (AES-256). Returns encryption key.
fn aes256_check_password(password: &[u8], dict: &EncryptDict) -> Option<Vec<u8>> {
    // Truncate password to 127 bytes per spec
    let password = &password[..password.len().min(127)];

    // Try user password first
    if dict.user_hash.len() >= 48 {
        let validation_salt = &dict.user_hash[32..40];
        let key_salt = &dict.user_hash[40..48];

        let hash = if dict.revision == 6 {
            revision6_hash(password, validation_salt, &[])?
        } else {
            sha256::digest(&[password, validation_salt].concat())
        };

        if hash[..] == dict.user_hash[..32] {
            // Recover key from UE
            let key_hash = if dict.revision == 6 {
                revision6_hash(password, key_salt, &[])?
            } else {
                sha256::digest(&[password, key_salt].concat())
            };
            if let Some(ue) = &dict.user_encrypted_key {
                let iv = [0u8; 16];
                if let Ok(key) = aes::decrypt_aes256_cbc(&key_hash, &iv, ue) {
                    return Some(key);
                }
            }
        }
    }

    // Try owner password
    if dict.owner_hash.len() >= 48 {
        let validation_salt = &dict.owner_hash[32..40];
        let key_salt = &dict.owner_hash[40..48];

        let hash = if dict.revision == 6 {
            revision6_hash(password, validation_salt, &dict.user_hash[..48])?
        } else {
            sha256::digest(&[password, validation_salt, &dict.user_hash[..48]].concat())
        };

        if hash[..] == dict.owner_hash[..32] {
            let key_hash = if dict.revision == 6 {
                revision6_hash(password, key_salt, &dict.user_hash[..48])?
            } else {
                sha256::digest(&[password, key_salt, &dict.user_hash[..48]].concat())
            };
            if let Some(oe) = &dict.owner_encrypted_key {
                let iv = [0u8; 16];
                if let Ok(key) = aes::decrypt_aes256_cbc(&key_hash, &iv, oe) {
                    return Some(key);
                }
            }
        }
    }

    None
}

/// Revision 6 iterative hash (PDF 2.0, Algorithm 2.B).
///
/// Uses SHA-256/384/512 adaptively with AES-128-CBC encryption rounds.
/// Ported from PDFium `Revision6_Hash()`.
fn revision6_hash(password: &[u8], salt: &[u8], vector: &[u8]) -> Option<[u8; 32]> {
    use crate::fdrm::{sha384, sha512};

    // Initial SHA-256 hash
    let mut initial_input = Vec::with_capacity(password.len() + salt.len() + vector.len());
    initial_input.extend_from_slice(password);
    initial_input.extend_from_slice(salt);
    initial_input.extend_from_slice(vector);
    let digest = sha256::digest(&initial_input);

    // `hash_result` holds the current hash (32, 48, or 64 bytes depending on round)
    let mut hash_result: Vec<u8> = digest.to_vec();
    let mut block_size: usize = 32;
    let mut i: usize = 0;

    loop {
        // Build K1 = password + hash_result[..block_size] + vector
        let input_span = &hash_result[..block_size];
        let k1_len = password.len() + block_size + vector.len();

        // Repeat K1 64 times to form content
        let mut content = Vec::with_capacity(k1_len * 64);
        for _ in 0..64 {
            content.extend_from_slice(password);
            content.extend_from_slice(input_span);
            content.extend_from_slice(vector);
        }

        // AES-128-CBC encrypt with key = first 16 bytes, iv = next 16 bytes
        let aes_key = &hash_result[..16];
        let aes_iv = &hash_result[16..32];
        let encrypted = aes::encrypt_aes128_cbc(aes_key, aes_iv, &content).ok()?;

        // Select hash based on first 16 bytes interpreted as big-endian mod 3
        let selector = big_order_mod3(&encrypted);
        let new_hash: Vec<u8> = match selector {
            0 => {
                block_size = 32;
                sha256::digest(&encrypted).to_vec()
            }
            1 => {
                block_size = 48;
                sha384::digest(&encrypted).to_vec()
            }
            _ => {
                block_size = 64;
                sha512::digest(&encrypted).to_vec()
            }
        };

        hash_result = new_hash;
        i += 1;

        // Termination: at least 64 rounds, then check last byte of encrypted output
        let last_byte = *encrypted.last().unwrap_or(&0) as usize;
        if i >= 64 && (i - 32) >= last_byte {
            break;
        }
    }

    let mut out = [0u8; 32];
    out.copy_from_slice(&hash_result[..32]);
    Some(out)
}

/// Compute `first 16 bytes as big-endian 128-bit integer mod 3`.
///
/// Ported from PDFium `BigOrder64BitsMod3()`.
fn big_order_mod3(data: &[u8]) -> u64 {
    let mut ret: u64 = 0;
    for i in 0..4 {
        ret <<= 32;
        let offset = i * 4;
        ret |= u32::from_be_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]) as u64;
        ret %= 3;
    }
    ret
}

/// Parse permissions from the /P integer value.
fn parse_permissions(p: i32) -> Permissions {
    let bits = p as u32;
    Permissions {
        print: bits & (1 << 2) != 0,
        modify: bits & (1 << 3) != 0,
        copy: bits & (1 << 4) != 0,
        annotate: bits & (1 << 5) != 0,
        fill_forms: bits & (1 << 8) != 0,
        extract: bits & (1 << 9) != 0,
        assemble: bits & (1 << 10) != 0,
        print_high_quality: bits & (1 << 11) != 0,
    }
}

/// Test helpers exposed to other modules within the crate.
#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Expose `pad_password` for integration tests.
    pub fn pad_password_helper(password: &[u8]) -> [u8; 32] {
        pad_password(password)
    }

    /// Expose `calc_encrypt_key` for integration tests.
    pub fn calc_encrypt_key_helper(
        password: &[u8],
        o: &[u8],
        p: i32,
        file_id: &[u8],
        key_length: usize,
        revision: u32,
        encrypt_metadata: bool,
    ) -> Vec<u8> {
        calc_encrypt_key(
            password,
            o,
            p,
            file_id,
            key_length,
            revision,
            encrypt_metadata,
        )
    }

    /// Expose `compute_u_r2` for integration tests.
    pub fn compute_u_r2_helper(key: &[u8]) -> [u8; 32] {
        compute_u_r2(key)
    }

    /// Expose `derive_object_key` for integration tests.
    pub fn derive_object_key_helper(
        key: &[u8],
        obj_num: u32,
        gen_num: u16,
        cipher: Cipher,
    ) -> Vec<u8> {
        derive_object_key(key, obj_num, gen_num, cipher)
    }

    // ---------------------------------------------------------------
    // pad_password
    // ---------------------------------------------------------------

    #[test]
    fn pad_password_empty() {
        assert_eq!(pad_password(b""), PASSWORD_PADDING);
    }

    #[test]
    fn pad_password_short() {
        let result = pad_password(b"user");
        assert_eq!(&result[..4], b"user");
        assert_eq!(&result[4..], &PASSWORD_PADDING[..28]);
    }

    #[test]
    fn pad_password_exact_32() {
        let input = [0x41u8; 32];
        assert_eq!(pad_password(&input), input);
    }

    #[test]
    fn pad_password_longer_than_32() {
        let input = [0x42u8; 40];
        assert_eq!(pad_password(&input), [0x42u8; 32]);
    }

    // ---------------------------------------------------------------
    // calc_encrypt_key
    // ---------------------------------------------------------------

    #[test]
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
    fn calc_encrypt_key_oversized_key_length_is_clamped() {
        // key_length > 16 should be clamped to 16 (MD5 output size)
        let o = [0xAAu8; 32];
        let p: i32 = -4;
        let file_id = b"0123456789abcdef";

        // key_length=32 would panic without clamping; with clamp it should
        // produce the same result as key_length=16
        let result_clamped = calc_encrypt_key(b"", &o, p, file_id, 32, 2, true);
        let result_16 = calc_encrypt_key(b"", &o, p, file_id, 16, 2, true);
        assert_eq!(result_clamped, result_16);
        assert_eq!(result_clamped.len(), 16);
    }

    #[test]
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
    fn compute_u_r2_known_key() {
        // R2: RC4-encrypt PASSWORD_PADDING with the encryption key
        let key = [0x01u8, 0x02, 0x03, 0x04, 0x05];
        let expected = crate::fdrm::rc4::crypt(&key, &PASSWORD_PADDING).unwrap();
        let result = compute_u_r2(&key);
        assert_eq!(&result[..], &expected[..]);
    }

    #[test]
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
    fn check_user_password_r2_wrong() {
        let file_id = b"0123456789abcdef";
        let o = [0xAAu8; 32];
        let p: i32 = -4;
        let (dict, _) = make_r2_dict(b"secret", &o, p, file_id);

        let result = check_user_password(b"wrong", &dict, file_id);
        assert!(result.is_none(), "wrong password should be rejected");
    }

    #[test]
    fn check_user_password_r2_empty_password() {
        let file_id = b"0123456789abcdef";
        let o = [0u8; 32];
        let p: i32 = -4;
        let (dict, expected_key) = make_r2_dict(b"", &o, p, file_id);

        let result = check_user_password(b"", &dict, file_id);
        assert!(result.is_some(), "empty password should be accepted");
        assert_eq!(result.unwrap(), expected_key);
    }

    #[test]
    fn check_user_password_short_user_hash_returns_none() {
        let file_id = b"0123456789abcdef";
        let o = [0xAAu8; 32];
        let p: i32 = -4;
        // Build a dict with user_hash shorter than 16 bytes
        let dict = EncryptDict {
            revision: 3,
            key_length: 16,
            cipher: Cipher::Rc4,
            permissions: p,
            owner_hash: o.to_vec(),
            user_hash: vec![0u8; 10], // too short
            owner_encrypted_key: None,
            user_encrypted_key: None,
            encrypted_perms: None,
            encrypt_metadata: true,
        };

        let result = check_user_password(b"", &dict, file_id);
        assert!(result.is_none(), "short user_hash should return None");
    }

    // ---------------------------------------------------------------
    // derive_object_key
    // ---------------------------------------------------------------

    #[test]
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
    fn security_handler_r2_empty_password() {
        let file_id = b"0123456789abcdef";
        let o = [0u8; 32];
        let p: i32 = -4;
        let (dict, _) = make_r2_dict(b"", &o, p, file_id);

        let handler = SecurityHandler::new(&dict, file_id, b"").unwrap();
        assert_eq!(handler.cipher(), Cipher::Rc4);
    }

    #[test]
    fn security_handler_wrong_password_is_error() {
        let file_id = b"0123456789abcdef";
        let o = [0u8; 32];
        let p: i32 = -4;
        let (dict, _) = make_r2_dict(b"", &o, p, file_id);

        let result = SecurityHandler::new(&dict, file_id, b"wrong_password");
        assert!(result.is_err());
    }

    #[test]
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

        // UE: pick a known UE, derive the expected key by decryption.
        // Since we don't have AES-256 encrypt, we work backwards:
        // key_for_ue = SHA-256(password + key_salt)
        // expected_key = AES-256-CBC-decrypt(UE, key_for_ue, iv=[0;16])
        let key_for_ue = crate::fdrm::sha256::digest(&[password.as_slice(), &key_salt].concat());
        let user_encrypted_key = vec![0xFFu8; 32]; // arbitrary 32-byte UE
        let expected_key =
            crate::fdrm::aes::decrypt_aes256_cbc(&key_for_ue, &[0u8; 16], &user_encrypted_key)
                .unwrap();

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
        assert!(result.is_some(), "R5 password check should pass");
        assert_eq!(result.unwrap(), expected_key);
    }
}
