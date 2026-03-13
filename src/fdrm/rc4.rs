/// RC4 stream cipher.
///
/// Used in PDF Standard Security Handler revisions 2, 3, and 4.
/// Key length: 5-16 bytes (40-128 bit).
pub struct Rc4 {
    s: [u8; 256],
    i: u8,
    j: u8,
}

impl Rc4 {
    /// Initialize RC4 with the given key (KSA phase).
    pub fn new(key: &[u8]) -> Self {
        let mut s = [0u8; 256];
        for (i, v) in s.iter_mut().enumerate() {
            *v = i as u8;
        }
        let mut j: u8 = 0;
        let n = key.len();
        for i in 0..256 {
            j = j.wrapping_add(s[i]).wrapping_add(key[i % n]);
            s.swap(i, j as usize);
        }
        Rc4 { s, i: 0, j: 0 }
    }

    /// Encrypt or decrypt data in-place (RC4 is symmetric).
    pub fn apply_keystream(&mut self, data: &mut [u8]) {
        for byte in data.iter_mut() {
            self.i = self.i.wrapping_add(1);
            self.j = self.j.wrapping_add(self.s[self.i as usize]);
            self.s.swap(self.i as usize, self.j as usize);
            let k =
                self.s[(self.s[self.i as usize].wrapping_add(self.s[self.j as usize])) as usize];
            *byte ^= k;
        }
    }
}

/// Convenience: encrypt/decrypt a slice with RC4 using the given key.
pub fn crypt(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut out = data.to_vec();
    Rc4::new(key).apply_keystream(&mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // RC4 keystream for key = [0x01, 0x02, 0x03, 0x04, 0x05].
    // Verified with multiple independent implementations (Python, Rust).
    // Note: RFC 6229 draft value for this key was incorrect in the original test.
    // Input:  0x00 * 16
    // Output: b239 6305 f03d c027 ccc3 524a 0a11 18a8

    #[test]
    fn rc4_encrypt_rfc6229_vector1() {
        let key = [0x01u8, 0x02, 0x03, 0x04, 0x05];
        let plaintext = [0x00u8; 16];
        let expected = [
            0xb2u8, 0x39, 0x63, 0x05, 0xf0, 0x3d, 0xc0, 0x27, 0xcc, 0xc3, 0x52, 0x4a, 0x0a, 0x11,
            0x18, 0xa8,
        ];
        let result = crypt(&key, &plaintext);
        assert_eq!(result, expected);
    }

    #[test]
    fn rc4_symmetric() {
        let key = b"secret";
        let plaintext = b"Hello, RC4!";
        let ciphertext = crypt(key, plaintext);
        let recovered = crypt(key, &ciphertext);
        assert_eq!(recovered, plaintext);
    }

    #[test]
    fn rc4_empty_input() {
        let result = crypt(b"key", b"");
        assert!(result.is_empty());
    }

    #[test]
    fn rc4_reference_comparison() {
        // Reference RC4 using usize arithmetic
        let key = [0x01u8, 0x02, 0x03, 0x04, 0x05];
        let n = key.len();
        let mut s: Vec<u8> = (0..=255u8).collect();
        let mut j: usize = 0;
        for i in 0..256usize {
            j = (j + s[i] as usize + key[i % n] as usize) % 256;
            s.swap(i, j);
        }
        let mut i_prga: usize = 0;
        let mut j_prga: usize = 0;
        let mut ref_out = [0u8; 16];
        for byte in ref_out.iter_mut() {
            i_prga = (i_prga + 1) % 256;
            j_prga = (j_prga + s[i_prga] as usize) % 256;
            s.swap(i_prga, j_prga);
            let k = s[(s[i_prga] as usize + s[j_prga] as usize) % 256];
            *byte ^= k;
        }
        eprintln!("reference first byte: 0x{:02x}", ref_out[0]);
        let our_out = crypt(&key, &[0u8; 16]);
        eprintln!("our first byte: 0x{:02x}", our_out[0]);
        assert_eq!(our_out, ref_out.to_vec(), "our RC4 must match reference");
    }

    #[test]
    fn rc4_stateful_streaming() {
        let key = b"key";
        let plaintext = b"Hello, streaming RC4!";
        // Streaming should match single-call crypt
        let mut rc4 = Rc4::new(key);
        let mut out = plaintext.to_vec();
        rc4.apply_keystream(&mut out);
        let expected = crypt(key, plaintext);
        assert_eq!(out, expected);
    }
}
