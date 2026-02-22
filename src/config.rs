//! Configuration for chunking behavior.
//!
//! - [`ChunkConfig`] - Chunk size boundaries and hashing
//! - [`HashConfig`] - Hash computation control

use crate::error::ChunkError;

/// Default minimum chunk size (4 KiB).
pub const DEFAULT_MIN_CHUNK_SIZE: usize = 4 * 1024;

/// Default average chunk size (16 KiB).
pub const DEFAULT_AVG_CHUNK_SIZE: usize = 16 * 1024;

/// Default maximum chunk size (64 KiB).
pub const DEFAULT_MAX_CHUNK_SIZE: usize = 64 * 1024;

/// Default normalization level (masks differ by ±2 bits).
pub const DEFAULT_NORMALIZATION_LEVEL: u8 = 2;

/// Configuration for content-defined chunking.
///
/// Size constraints: `min_size <= avg_size <= max_size`, all powers of 2.
///
/// Normalization level controls chunk size distribution:
/// - Level 0: Single mask
/// - Level 1: Masks differ by ±1 bit
/// - Level N: Masks differ by ±N bits
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChunkConfig {
    min_size: usize,
    avg_size: usize,
    max_size: usize,
    normalization_level: u8,
    hash_config: HashConfig,
    #[cfg(feature = "keyed-cdc")]
    key: Option<[u8; 32]>,
}

impl ChunkConfig {
    /// Creates a new configuration.
    ///
    /// Returns error if sizes are zero, not powers of 2, or out of order.
    pub fn new(min_size: usize, avg_size: usize, max_size: usize) -> Result<Self, ChunkError> {
        if min_size == 0 || avg_size == 0 || max_size == 0 {
            return Err(ChunkError::InvalidConfig {
                message: "chunk sizes must be non-zero",
            });
        }

        if min_size > avg_size {
            return Err(ChunkError::InvalidConfig {
                message: "min_size cannot be greater than avg_size",
            });
        }

        if avg_size > max_size {
            return Err(ChunkError::InvalidConfig {
                message: "avg_size cannot be greater than max_size",
            });
        }

        if !min_size.is_power_of_two() || !avg_size.is_power_of_two() || !max_size.is_power_of_two()
        {
            return Err(ChunkError::InvalidConfig {
                message: "chunk sizes should be powers of 2",
            });
        }

        let avg_bits = avg_size.trailing_zeros() as u8;
        let effective_level = DEFAULT_NORMALIZATION_LEVEL.min(avg_bits.saturating_sub(2));

        Ok(Self {
            min_size,
            avg_size,
            max_size,
            normalization_level: effective_level,
            hash_config: HashConfig::default(),
            #[cfg(feature = "keyed-cdc")]
            key: None,
        })
    }

    /// Sets the minimum chunk size.
    pub fn with_min_size(mut self, size: usize) -> Self {
        self.min_size = size;
        self
    }

    /// Sets the average chunk size.
    pub fn with_avg_size(mut self, size: usize) -> Self {
        self.avg_size = size;
        self
    }

    /// Sets the maximum chunk size.
    pub fn with_max_size(mut self, size: usize) -> Self {
        self.max_size = size;
        self
    }

    /// Sets the normalization level (0-31).
    pub fn with_normalization_level(mut self, level: u8) -> Self {
        self.normalization_level = level;
        self
    }

    /// Sets the hash configuration.
    pub fn with_hash_config(mut self, config: HashConfig) -> Self {
        self.hash_config = config;
        self
    }

    /// Sets the key for keyed gear table (requires `keyed-cdc` feature).
    #[cfg(feature = "keyed-cdc")]
    pub fn with_keyed_gear_table(mut self, key: Option<[u8; 32]>) -> Self {
        self.key = key;
        self
    }

    /// Returns the minimum chunk size.
    pub fn min_size(&self) -> usize {
        self.min_size
    }

    /// Returns the average chunk size.
    pub fn avg_size(&self) -> usize {
        self.avg_size
    }

    /// Returns the maximum chunk size.
    pub fn max_size(&self) -> usize {
        self.max_size
    }

    /// Returns the normalization level.
    pub fn normalization_level(&self) -> u8 {
        self.normalization_level
    }

    /// Returns the hash configuration.
    pub fn hash_config(&self) -> &HashConfig {
        &self.hash_config
    }

    /// Returns the keyed gear table key, if set (requires `keyed-cdc` feature).
    #[cfg(feature = "keyed-cdc")]
    pub fn keyed_gear_table_key(&self) -> Option<[u8; 32]> {
        self.key
    }

    /// Validates the current configuration.
    pub fn validate(&self) -> Result<(), ChunkError> {
        Self::new(self.min_size, self.avg_size, self.max_size).map(|_| ())
    }
}

impl Default for ChunkConfig {
    fn default() -> Self {
        Self {
            min_size: DEFAULT_MIN_CHUNK_SIZE,
            avg_size: DEFAULT_AVG_CHUNK_SIZE,
            max_size: DEFAULT_MAX_CHUNK_SIZE,
            normalization_level: DEFAULT_NORMALIZATION_LEVEL,
            hash_config: HashConfig::default(),
            #[cfg(feature = "keyed-cdc")]
            key: None,
        }
    }
}

/// Configuration for chunk hashing.
///
/// Controls whether BLAKE3 cryptographic hashes are computed for each chunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HashConfig {
    /// Whether to compute BLAKE3 hashes.
    pub enabled: bool,
}

impl HashConfig {
    /// Creates a new hash configuration.
    pub const fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    /// Enables hashing.
    pub const fn enabled() -> Self {
        Self { enabled: true }
    }

    /// Disables hashing.
    pub const fn disabled() -> Self {
        Self { enabled: false }
    }
}

impl Default for HashConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_config_default() {
        let config = ChunkConfig::default();
        assert_eq!(config.min_size(), 4 * 1024);
        assert_eq!(config.avg_size(), 16 * 1024);
        assert_eq!(config.max_size(), 64 * 1024);
    }

    #[test]
    fn test_chunk_config_builder() {
        let config = ChunkConfig::default()
            .with_min_size(8192)
            .with_avg_size(32768)
            .with_max_size(131072);
        assert_eq!(config.min_size(), 8192);
        assert_eq!(config.avg_size(), 32768);
        assert_eq!(config.max_size(), 131072);
    }

    #[test]
    fn test_chunk_config_valid() {
        let config = ChunkConfig::new(4096, 16384, 65536).unwrap();
        assert_eq!(config.min_size(), 4096);
        assert_eq!(config.avg_size(), 16384);
        assert_eq!(config.max_size(), 65536);
    }

    #[test]
    fn test_chunk_config_invalid_zero() {
        assert!(ChunkConfig::new(0, 16384, 65536).is_err());
        assert!(ChunkConfig::new(4096, 0, 65536).is_err());
        assert!(ChunkConfig::new(4096, 16384, 0).is_err());
    }

    #[test]
    fn test_chunk_config_invalid_ordering() {
        assert!(ChunkConfig::new(32768, 16384, 65536).is_err());
        assert!(ChunkConfig::new(4096, 65536, 16384).is_err());
    }

    #[test]
    fn test_chunk_config_invalid_non_power_of_two() {
        assert!(ChunkConfig::new(5, 16, 64).is_err());
        assert!(ChunkConfig::new(4, 17, 64).is_err());
        assert!(ChunkConfig::new(4, 16, 65).is_err());
    }

    #[test]
    fn test_hash_config_default() {
        assert!(HashConfig::default().enabled);
    }

    #[test]
    fn test_hash_config_enabled() {
        assert!(HashConfig::enabled().enabled);
    }

    #[test]
    fn test_hash_config_disabled() {
        assert!(!HashConfig::disabled().enabled);
    }

    #[test]
    fn test_hash_config_new() {
        assert!(HashConfig::new(true).enabled);
        assert!(!HashConfig::new(false).enabled);
    }

    #[test]
    fn test_chunk_config_with_hash_config() {
        let hash_cfg = HashConfig::disabled();
        let chunk_cfg = ChunkConfig::default().with_hash_config(hash_cfg);
        assert!(!chunk_cfg.hash_config().enabled);
    }

    #[test]
    fn test_chunk_config_validate() {
        let config = ChunkConfig::default().with_min_size(0);
        assert!(config.validate().is_err());
    }
}