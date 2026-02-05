//! BLAKE3-based chunk hashing implementation.

use crate::chunk::ChunkHash;

/// A hasher that computes BLAKE3 hashes.
#[derive(Debug, Clone)]
pub struct Blake3Hasher {
    state: blake3::Hasher,
}

impl Blake3Hasher {
    /// Creates a new hasher.
    pub fn new() -> Self {
        Self {
            state: blake3::Hasher::new(),
        }
    }

    /// Creates a new hasher with a key for keyed hashing.
    #[allow(dead_code)]
    pub(crate) fn new_keyed(key: &[u8; 32]) -> Self {
        Self {
            state: blake3::Hasher::new_keyed(key),
        }
    }

    /// Updates the hasher with more data.
    pub fn update(&mut self, data: &[u8]) {
        self.state.update(data);
    }

    /// Finalizes and returns the hash.
    pub fn finalize(&self) -> ChunkHash {
        ChunkHash::new(self.state.finalize().into())
    }

    /// Resets the hasher to its initial state.
    pub fn reset(&mut self) {
        self.state.reset();
    }

    /// Convenience method to hash data in one shot.
    #[allow(dead_code)]
    pub(crate) fn hash(data: &[u8]) -> ChunkHash {
        ChunkHash::new(blake3::hash(data).into())
    }
}

impl Default for Blake3Hasher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash() {
        let hash = Blake3Hasher::hash(b"hello world");
        assert_eq!(hash.as_bytes().len(), 32);

        // Hash should be deterministic
        let hash2 = Blake3Hasher::hash(b"hello world");
        assert_eq!(hash, hash2);

        // Different data should give different hash
        let hash3 = Blake3Hasher::hash(b"hello world!");
        assert_ne!(hash, hash3);
    }

    #[test]
    fn test_incremental_hashing() {
        let mut hasher = Blake3Hasher::new();
        hasher.update(b"hello ");
        hasher.update(b"world");
        let hash = hasher.finalize();

        // Should match one-shot hashing
        let expected = Blake3Hasher::hash(b"hello world");
        assert_eq!(hash, expected);
    }

    #[test]
    fn test_reset() {
        let mut hasher = Blake3Hasher::new();
        hasher.update(b"some data");

        hasher.reset();
        hasher.update(b"hello world");
        let hash = hasher.finalize();

        let expected = Blake3Hasher::hash(b"hello world");
        assert_eq!(hash, expected);
    }
}
