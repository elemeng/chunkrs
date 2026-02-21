//! Cryptographic hash representation for chunk identity.
//!
//! This module defines [`ChunkHash`], a wrapper around a 32-byte BLAKE3 hash
//! that provides methods for serialization, display, and comparison.

use std::fmt;
use std::hash::{Hash as StdHash, Hasher};

/// A fixed-size cryptographic hash representing chunk content.
///
/// `ChunkHash` is a newtype wrapper around a 32-byte array containing a
/// BLAKE3 hash. It provides:
///
/// - Type safety to distinguish hashes from arbitrary byte arrays
/// - Hex encoding/decoding for serialization
/// - Display formatting for debugging and logging
/// - Standard trait implementations for use in collections
///
/// # Example
///
/// ```
/// use chunkrs::ChunkHash;
///
/// // Create from byte array
/// let hash = ChunkHash::new([0u8; 32]);
///
/// // Convert to hex string
/// let hex = hash.to_hex();
/// assert_eq!(hex.len(), 64);
///
/// // Parse from hex string
/// let parsed = ChunkHash::from_hex(&hex).unwrap();
/// assert_eq!(hash, parsed);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ChunkHash([u8; 32]);

impl ChunkHash {
    /// The size of the hash in bytes (256 bits).
    pub const SIZE: usize = 32;

    /// Creates a new chunk hash from a byte array.
    ///
    /// # Arguments
    ///
    /// * `bytes` - A 32-byte array containing the hash value
    ///
    /// # Example
    ///
    /// ```
    /// use chunkrs::ChunkHash;
    ///
    /// let bytes = [0u8; 32];
    /// let hash = ChunkHash::new(bytes);
    /// ```
    pub const fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Creates a new chunk hash from a slice.
    ///
    /// Returns `None` if the slice is not exactly 32 bytes.
    ///
    /// # Arguments
    ///
    /// * `slice` - A slice containing the hash bytes
    ///
    /// # Example
    ///
    /// ```
    /// use chunkrs::ChunkHash;
    ///
    /// let bytes = vec![0u8; 32];
    /// let hash = ChunkHash::from_slice(&bytes).unwrap();
    ///
    /// // Wrong size returns None
    /// assert!(ChunkHash::from_slice(&[0u8; 31]).is_none());
    /// ```
    pub fn from_slice(slice: &[u8]) -> Option<Self> {
        if slice.len() != 32 {
            return None;
        }
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(slice);
        Some(Self(bytes))
    }

    /// Returns the hash as a byte array reference.
    ///
    /// # Example
    ///
    /// ```
    /// use chunkrs::ChunkHash;
    ///
    /// let bytes = [0u8; 32];
    /// let hash = ChunkHash::new(bytes);
    /// assert_eq!(hash.as_bytes(), &bytes);
    /// ```
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Returns the hash as a hexadecimal string.
    ///
    /// The output uses lowercase hex digits and is always 64 characters long.
    ///
    /// # Example
    ///
    /// ```
    /// use chunkrs::ChunkHash;
    ///
    /// let bytes = [0xABu8; 32];
    /// let hash = ChunkHash::new(bytes);
    /// let hex = hash.to_hex();
    /// assert_eq!(hex.len(), 64);
    /// assert!(hex.chars().all(|c| c == 'a' || c == 'b'));
    /// ```
    pub fn to_hex(&self) -> String {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut result = String::with_capacity(64);
        for byte in &self.0 {
            result.push(HEX[(byte >> 4) as usize] as char);
            result.push(HEX[(byte & 0xf) as usize] as char);
        }
        result
    }

    /// Creates a hash from a hexadecimal string.
    ///
    /// Returns `None` if the string is not valid hexadecimal or not exactly
    /// 64 characters long.
    ///
    /// # Arguments
    ///
    /// * `hex_str` - A 64-character hex string
    ///
    /// # Example
    ///
    /// ```
    /// use chunkrs::ChunkHash;
    ///
    /// let hash = ChunkHash::new([0u8; 32]);
    /// let hex = hash.to_hex();
    ///
    /// let parsed = ChunkHash::from_hex(&hex).unwrap();
    /// assert_eq!(hash, parsed);
    ///
    /// // Invalid input returns None
    /// assert!(ChunkHash::from_hex("not hex").is_none());
    /// ```
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
    fn test_chunk_hash_creation() {
        let bytes = [0x42u8; 32];
        let hash = ChunkHash::new(bytes);
        
        assert_eq!(hash.as_bytes(), &bytes);
    }

    #[test]
    fn test_chunk_hash_from_slice_valid() {
        let bytes = vec![0x33u8; 32];
        let hash = ChunkHash::from_slice(&bytes).unwrap();
        
        assert_eq!(hash.as_bytes().as_ref(), bytes.as_slice());
    }

    #[test]
    fn test_chunk_hash_from_slice_invalid() {
        // Too short
        assert!(ChunkHash::from_slice(&[0u8; 31]).is_none());
        
        // Too long
        assert!(ChunkHash::from_slice(&[0u8; 33]).is_none());
    }

    #[test]
    fn test_chunk_hash_to_hex() {
        let bytes = [0xABu8; 32];
        let hash = ChunkHash::new(bytes);
        let hex = hash.to_hex();
        
        assert_eq!(hex.len(), 64);
        assert!(hex.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_chunk_hash_display() {
        let bytes = [0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 
                      0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                      0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                      0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let hash = ChunkHash::new(bytes);
        let s = format!("{}", hash);
        
        assert!(s.starts_with("0123456789abcdef"));
        assert_eq!(s.len(), 64);
    }

    #[test]
    fn test_chunk_hash_from_hex_roundtrip() {
        let bytes = [0xFFu8; 32];
        let hash1 = ChunkHash::new(bytes);
        let hex = hash1.to_hex();
        let hash2 = ChunkHash::from_hex(&hex).unwrap();
        
        assert_eq!(hash1, hash2, "Hex roundtrip must preserve hash");
    }

    #[test]
    fn test_chunk_hash_from_hex_invalid() {
        // Wrong length
        assert!(ChunkHash::from_hex("1234").is_none());
        
        // Invalid hex
        assert!(ChunkHash::from_hex(&"g".repeat(64)).is_none());
    }

    #[test]
    fn test_chunk_hash_equality() {
        let bytes = [0x78u8; 32];
        let hash1 = ChunkHash::new(bytes);
        let hash2 = ChunkHash::new(bytes);
        let hash3 = ChunkHash::new([0x00; 32]);
        
        assert_eq!(hash1, hash2, "Same bytes must be equal");
        assert_ne!(hash1, hash3, "Different bytes must not be equal");
    }

    #[test]
    fn test_chunk_hash_ord() {
        let hash1 = ChunkHash::new([0x00; 32]);
        let hash2 = ChunkHash::new([0xFF; 32]);
        
        assert!(hash1 < hash2, "Hash ordering must match byte ordering");
    }
}
