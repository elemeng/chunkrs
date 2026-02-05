//! Chunk hash types.

use std::fmt;
use std::hash::{Hash as StdHash, Hasher};

/// A fixed-size hash value representing chunk content.
///
/// This is a thin wrapper around a 32-byte array (BLAKE3 hash).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ChunkHash([u8; 32]);

impl ChunkHash {
    /// The size of the hash in bytes.
    pub const SIZE: usize = 32;

    /// Creates a new chunk hash from a byte array.
    pub const fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Creates a new chunk hash from a slice.
    ///
    /// Returns `None` if the slice is not exactly 32 bytes.
    pub fn from_slice(slice: &[u8]) -> Option<Self> {
        if slice.len() != 32 {
            return None;
        }
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(slice);
        Some(Self(bytes))
    }

    /// Returns the hash as a byte slice.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Returns the hash as a hex string.
    pub fn to_hex(&self) -> String {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut result = String::with_capacity(64);
        for byte in &self.0 {
            result.push(HEX[(byte >> 4) as usize] as char);
            result.push(HEX[(byte & 0xf) as usize] as char);
        }
        result
    }

    /// Creates a hash from a hex string.
    ///
    /// Returns `None` if the string is not valid hex or not exactly 64 characters.
    pub fn from_hex(hex_str: &str) -> Option<Self> {
        if hex_str.len() != 64 {
            return None;
        }
        let mut bytes = [0u8; 32];
        for i in 0..32 {
            let byte_str = &hex_str[i * 2..i * 2 + 2];
            bytes[i] = u8::from_str_radix(byte_str, 16).ok()?;
        }
        Some(Self(bytes))
    }
}

impl AsRef<[u8]> for ChunkHash {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl StdHash for ChunkHash {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write(&self.0);
    }
}

impl fmt::Display for ChunkHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in &self.0 {
            write!(f, "{:02x}", byte)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let bytes = [0u8; 32];
        let hash = ChunkHash::new(bytes);
        assert_eq!(hash.as_bytes(), &bytes);
    }

    #[test]
    fn test_from_slice() {
        let bytes = vec![0u8; 32];
        let hash = ChunkHash::from_slice(&bytes).unwrap();
        assert_eq!(hash.as_bytes().as_ref(), bytes.as_slice());

        // Wrong size
        assert!(ChunkHash::from_slice(&[0u8; 31]).is_none());
        assert!(ChunkHash::from_slice(&[0u8; 33]).is_none());
    }

    #[test]
    fn test_to_hex() {
        let bytes = [0xABu8; 32];
        let hash = ChunkHash::new(bytes);
        let hex = hash.to_hex();
        assert_eq!(hex.len(), 64);
        assert!(hex.chars().all(|c| c == 'a' || c == 'b'));
    }

    #[test]
    fn test_display() {
        let bytes = [0x01u8, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF];
        let mut full_bytes = [0u8; 32];
        full_bytes[..8].copy_from_slice(&bytes);
        let hash = ChunkHash::new(full_bytes);
        let s = hash.to_string();
        assert!(s.starts_with("0123456789abcdef"));
    }
}
