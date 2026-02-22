//! Cryptographic hash for chunk identity.

use std::fmt;
use std::hash::{Hash as StdHash, Hasher};

/// A fixed-size cryptographic hash representing chunk content.
///
/// 32-byte BLAKE3 hash wrapper with:
/// - Type safety
/// - Hex encoding/decoding
/// - Display formatting
/// - Standard trait implementations
///
/// # Example
///
/// ```
/// use chunkrs::ChunkHash;
///
/// let hash = ChunkHash::new([0u8; 32]);
/// let hex = hash.to_hex();
/// assert_eq!(hex.len(), 64);
///
/// let parsed = ChunkHash::from_hex(&hex).unwrap();
/// assert_eq!(hash, parsed);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ChunkHash([u8; 32]);

impl ChunkHash {
    /// The size of the hash in bytes (256 bits).
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

    /// Returns the hash as a byte array reference.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Converts the hash to a hexadecimal string.
    pub fn to_hex(&self) -> String {
        let mut hex = String::with_capacity(64);
        for byte in &self.0 {
            hex.push_str(&format!("{:02x}", byte));
        }
        hex
    }

    /// Parses a hash from a hexadecimal string.
    ///
    /// Returns `None` if the string is not exactly 64 hex characters.
    pub fn from_hex(hex: &str) -> Option<Self> {
        if hex.len() != 64 {
            return None;
        }

        let mut bytes = [0u8; 32];
        for (i, chunk) in hex.as_bytes().chunks(2).enumerate() {
            let byte_str = std::str::from_utf8(chunk).ok()?;
            bytes[i] = u8::from_str_radix(byte_str, 16).ok()?;
        }

        Some(Self(bytes))
    }

    /// Checks if this hash is all zeros.
    pub fn is_zero(&self) -> bool {
        self.0.iter().all(|&b| b == 0)
    }
}

impl AsRef<[u8]> for ChunkHash {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl StdHash for ChunkHash {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl fmt::Display for ChunkHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_hash_new() {
        let bytes = [0u8; 32];
        let hash = ChunkHash::new(bytes);
        assert_eq!(hash.as_bytes(), &bytes);
    }

    #[test]
    fn test_chunk_hash_from_slice() {
        let bytes = vec![0u8; 32];
        let hash = ChunkHash::from_slice(&bytes).unwrap();
        assert_eq!(hash.as_bytes().len(), 32);
    }

    #[test]
    fn test_chunk_hash_from_slice_invalid() {
        assert!(ChunkHash::from_slice(&[0u8; 31]).is_none());
        assert!(ChunkHash::from_slice(&[0u8; 33]).is_none());
    }

    #[test]
    fn test_chunk_hash_to_hex() {
        let hash = ChunkHash::new([0xAB; 32]);
        let hex = hash.to_hex();
        assert_eq!(hex.len(), 64);
        assert_eq!(&hex[..2], "ab");
    }

    #[test]
    fn test_chunk_hash_from_hex() {
        let hash = ChunkHash::new([0xCD; 32]);
        let hex = hash.to_hex();
        let parsed = ChunkHash::from_hex(&hex).unwrap();
        assert_eq!(hash, parsed);
    }

    #[test]
    fn test_chunk_hash_from_hex_invalid() {
        assert!(ChunkHash::from_hex("").is_none());
        assert!(ChunkHash::from_hex("ab").is_none());
        assert!(ChunkHash::from_hex(&"ab".repeat(33)).is_none());
    }

    #[test]
    fn test_chunk_hash_is_zero() {
        assert!(ChunkHash::new([0u8; 32]).is_zero());
        assert!(!ChunkHash::new([1u8; 32]).is_zero());
    }

    #[test]
    fn test_chunk_hash_display() {
        let hash = ChunkHash::new([
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D,
            0x0E, 0x0F, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x1B,
            0x1C, 0x1D, 0x1E, 0x1F,
        ]);
        let s = format!("{}", hash);
        assert_eq!(s.len(), 64);
    }

    #[test]
    fn test_chunk_hash_equality() {
        let bytes = [0u8; 32];
        let hash1 = ChunkHash::new(bytes);
        let hash2 = ChunkHash::new(bytes);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_chunk_hash_ordering() {
        let hash1 = ChunkHash::new([0x00; 32]);
        let hash2 = ChunkHash::new([0xFF; 32]);
        assert!(hash1 < hash2);
    }

    #[test]
    fn test_chunk_hash_as_ref() {
        let bytes = [0xAB; 32];
        let hash = ChunkHash::new(bytes);
        let slice: &[u8] = hash.as_ref();
        assert_eq!(slice, &bytes[..]);
    }
}
