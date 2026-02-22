//! BLAKE3-based chunk hashing implementation.
//!
//! This module provides a wrapper around the BLAKE3 hash function for computing
//! cryptographic hashes of chunk data.
//!
//! # Features
//!
//! - **Fast**: BLAKE3 is optimized for performance on modern CPUs
//! - **Secure**: Cryptographically strong hash function
//! - **Deterministic**: Same input always produces the same hash
//! - **Incremental**: Supports streaming updates for large data

#[cfg(feature = "hash-blake3")]
use crate::chunk::ChunkHash;

/// A hasher that computes BLAKE3 hashes.
///
/// `Blake3Hasher` wraps the `blake3` crate's hasher and provides a convenient
/// API for computing hashes incrementally or in one shot.
///
/// # Example
///
/// ```ignore
/// use chunkrs::hash::Blake3Hasher;
/// use chunkrs::ChunkHash;
///
/// // Incremental hashing
/// let mut hasher = Blake3Hasher::new();
/// hasher.update(b"hello ");
/// hasher.update(b"world");
/// let hash = hasher.finalize();
///
/// // One-shot hashing
/// let hash2 = Blake3Hasher::hash(b"hello world");
/// assert_eq!(hash, hash2);
/// ```
#[derive(Debug, Clone)]
pub struct Blake3Hasher {
    state: blake3::Hasher,
}

impl Blake3Hasher {
    /// Creates a new hasher.
    ///
    /// The hasher is initialized with default BLAKE3 parameters.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use chunkrs::hash::Blake3Hasher;
    ///
    /// let hasher = Blake3Hasher::new();
    /// ```
    pub fn new() -> Self {
        Self {
            state: blake3::Hasher::new(),
        }
    }

    /// Creates a new hasher with a key for keyed hashing.
    ///
    /// Keyed hashing uses a 32-byte key to ensure that only those with the key
    /// can verify or forge hashes. This is useful for HMAC-like applications.
    ///
    /// # Arguments
    ///
    /// * `key` - A 32-byte key for the keyed hash
    #[allow(dead_code)]
    pub fn new_keyed(key: &[u8; 32]) -> Self {
        Self {
            state: blake3::Hasher::new_keyed(key),
        }
    }

    /// Updates the hasher with more data.
    ///
    /// This can be called multiple times to incrementally hash large amounts
    /// of data without loading it all into memory at once.
    ///
    /// # Arguments
    ///
    /// * `data` - The data to add to the hash
    ///
    /// # Example
    ///
    /// ```ignore
    /// use chunkrs::hash::Blake3Hasher;
    ///
    /// let mut hasher = Blake3Hasher::new();
    /// hasher.update(b"hello ");
    /// hasher.update(b"world");
    /// ```
    #[allow(dead_code)]
    pub fn update(&mut self, data: &[u8]) {
        self.state.update(data);
    }

    /// Finalizes and returns the hash.
    ///
    /// Consumes the hasher and returns the computed hash. The hasher can be
    /// reused by calling [`Blake3Hasher::reset`] after finalizing.
    ///
    /// # Returns
    ///
    /// A [`ChunkHash`] containing the 32-byte BLAKE3 hash
    ///
    /// # Example
    ///
    /// ```ignore
    /// use chunkrs::hash::Blake3Hasher;
    ///
    /// let mut hasher = Blake3Hasher::new();
    /// hasher.update(b"hello world");
    /// let hash = hasher.finalize();
    /// ```
    #[allow(dead_code)]
    pub fn finalize(&self) -> ChunkHash {
        ChunkHash::new(self.state.finalize().into())
    }

    /// Resets the hasher to its initial state.
    ///
    /// Allows the hasher to be reused for computing new hashes without
    /// allocating a new one.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use chunkrs::hash::Blake3Hasher;
    ///
    /// let mut hasher = Blake3Hasher::new();
    /// hasher.update(b"hello");
    /// let hash1 = hasher.finalize();
    ///
    /// hasher.reset();
    /// hasher.update(b"world");
    /// let hash2 = hasher.finalize();
    ///
    /// assert_ne!(hash1, hash2);
    /// ```
    #[allow(dead_code)]
    pub fn reset(&mut self) {
        self.state.reset();
    }

    /// Convenience method to hash data in one shot.
    ///
    /// This is equivalent to creating a hasher, updating it with the data,
    /// and finalizing it.
    ///
    /// # Arguments
    ///
    /// * `data` - The data to hash
    ///
    /// # Returns
    ///
    /// A [`ChunkHash`] containing the 32-byte BLAKE3 hash
    ///
    /// # Example
    ///
    /// ```ignore
    /// use chunkrs::hash::Blake3Hasher;
    ///
    /// let hash = Blake3Hasher::hash(b"hello world");
    /// ```
    pub fn hash(data: &[u8]) -> ChunkHash {
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
    fn test_hash_determinism() {
        let hash1 = Blake3Hasher::hash(b"hello world");
        let hash2 = Blake3Hasher::hash(b"hello world");

        assert_eq!(hash1, hash2, "Same input must produce same hash");
        assert_eq!(hash1.as_bytes().len(), 32, "Hash must be 32 bytes");
    }

    #[test]
    fn test_hash_uniqueness() {
        let hash1 = Blake3Hasher::hash(b"hello world");
        let hash2 = Blake3Hasher::hash(b"hello world!");

        assert_ne!(
            hash1, hash2,
            "Different inputs must produce different hashes"
        );
    }

    #[test]
    fn test_incremental_hashing() {
        let mut hasher = Blake3Hasher::new();
        hasher.update(b"hello ");
        hasher.update(b"world");
        let incremental_hash = hasher.finalize();

        let one_shot_hash = Blake3Hasher::hash(b"hello world");

        assert_eq!(
            incremental_hash, one_shot_hash,
            "Incremental hashing must match one-shot hashing"
        );
    }

    #[test]
    fn test_hasher_reset() {
        let mut hasher = Blake3Hasher::new();
        hasher.update(b"first data");
        hasher.reset();
        hasher.update(b"second data");
        let hash2 = hasher.finalize();

        let expected = Blake3Hasher::hash(b"second data");
        assert_eq!(hash2, expected, "Reset must clear previous state");
    }

    #[test]
    fn test_hasher_multiple_updates() {
        let mut hasher = Blake3Hasher::new();

        // Multiple small updates
        hasher.update(b"a");
        hasher.update(b"b");
        hasher.update(b"c");
        let hash1 = hasher.finalize();

        let hash2 = Blake3Hasher::hash(b"abc");
        assert_eq!(hash1, hash2, "Multiple updates must produce correct hash");
    }
}
