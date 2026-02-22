//! Configuration for chunking behavior.
//!
//! This module provides types to configure how chunking is performed:
//!
//! - [`ChunkConfig`] - Controls chunk size boundaries and hashing behavior
//! - [`HashConfig`] - Specifies whether to compute cryptographic hashes
//!
//! # Example
//!
//! ```
//! use chunkrs::{ChunkConfig, HashConfig};
//!
//! // Custom chunk sizes
//! let config = ChunkConfig::new(4096, 16384, 65536)?;
//!
//! // Enable hashing
//! let config = ChunkConfig::default()
//!     .with_hash_config(HashConfig::enabled());
//!
//! # Ok::<(), chunkrs::ChunkError>(())
//! ```

use crate::error::ChunkError;

/// Default minimum chunk size (4 KiB).
pub const DEFAULT_MIN_CHUNK_SIZE: usize = 4 * 1024;

/// Default average/target chunk size (16 KiB).
pub const DEFAULT_AVG_CHUNK_SIZE: usize = 16 * 1024;

/// Default maximum chunk size (64 KiB).
pub const DEFAULT_MAX_CHUNK_SIZE: usize = 64 * 1024;

/// Default normalization level for FastCDC mask generation.
///
/// Controls how aggressively chunk sizes are distributed around the average.
/// Level 2 means masks differ by ±2 bits from the base mask at avg_size.
/// This matches the Go FastCDC implementation's default.
pub const DEFAULT_NORMALIZATION_LEVEL: u8 = 2;

/// Configuration for content-defined chunking behavior.
///
/// `ChunkConfig` controls the size constraints and hashing behavior for the
/// chunking process. It uses the FastCDC algorithm which requires:
///
/// - Minimum chunk size (`min_size`) - No chunk will be smaller than this
/// - Average chunk size (`avg_size`) - Target size for most chunks
/// - Maximum chunk size (`max_size`) - No chunk will exceed this
/// - Normalization level (`normalization_level`) - Controls chunk size distribution
///
/// # Size Constraints
///
/// All sizes must be:
/// - Non-zero
/// - Powers of 2 (for optimal performance)
/// - Ordered: `min_size <= avg_size <= max_size`
///
/// # Normalization Level
///
/// The normalization level controls how aggressively chunk sizes are distributed
/// around the average:
///
/// - **Level 0**: No normalization - single mask throughout
/// - **Level 1** (default): Masks differ by ±1 bit - balanced distribution
/// - **Level 2+**: Masks differ by ±N bits - tighter distribution
///
/// Higher levels produce more predictable chunk sizes but may reduce deduplication
/// ratio for heterogeneous data.
///
/// # Example
///
/// ```
/// use chunkrs::ChunkConfig;
///
/// // Use default configuration
/// let config = ChunkConfig::default();
///
/// // Custom configuration
/// let config = ChunkConfig::new(4096, 16384, 65536)?;
///
/// // Builder pattern
/// let config = ChunkConfig::default()
///     .with_min_size(8192)
///     .with_avg_size(32768)
///     .with_max_size(131072)
///     .with_normalization_level(2);
/// # Ok::<(), chunkrs::ChunkError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChunkConfig {
    /// Minimum chunk size in bytes.
    min_size: usize,

    /// Average/target chunk size in bytes.
    avg_size: usize,

    /// Maximum chunk size in bytes.
    max_size: usize,

    /// Normalization level for mask generation (0-31).
    normalization_level: u8,

    /// Configuration for hashing behavior.
    hash_config: HashConfig,

    /// Optional key for keyed gear table (security feature).
    ///
    /// When set, the gear table is hashed with this key using BLAKE3,
    /// preventing adversarial chunk boundary manipulation attacks.
    /// This requires the `keyed-cdc` feature flag.
    #[cfg(feature = "keyed-cdc")]
    key: Option<[u8; 32]>,
}

impl ChunkConfig {
    /// Creates a new configuration with the specified size bounds.
    ///
    /// # Arguments
    ///
    /// * `min_size` - Minimum chunk size in bytes (must be power of 2)
    /// * `avg_size` - Average/target chunk size in bytes (must be power of 2)
    /// * `max_size` - Maximum chunk size in bytes (must be power of 2)
    ///
    /// # Errors
    ///
    /// Returns [`ChunkError::InvalidConfig`] if:
    /// - Any size is zero
    /// - `min_size > avg_size` or `avg_size > max_size`
    /// - Sizes are not powers of 2
    ///
    /// # Example
    ///
    /// ```
    /// use chunkrs::ChunkConfig;
    ///
    /// let config = ChunkConfig::new(4096, 16384, 65536)?;
    /// assert_eq!(config.min_size(), 4096);
    /// # Ok::<(), chunkrs::ChunkError>(())
    /// ```
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

        // FastCDC works best with power-of-2 sizes
        if !min_size.is_power_of_two() || !avg_size.is_power_of_two() || !max_size.is_power_of_two()
        {
            return Err(ChunkError::InvalidConfig {
                message: "chunk sizes should be powers of 2 for optimal performance",
            });
        }

        // Validate normalization level doesn't exceed available bits
        let avg_bits = avg_size.trailing_zeros() as u8;
        // Use the smaller of default level and available bits
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
    ///
    /// Note: This does not validate the configuration. Use [`ChunkConfig::validate`]
    /// to check if the configuration is valid.
    ///
    /// # Example
    ///
    /// ```
    /// use chunkrs::ChunkConfig;
    ///
    /// let config = ChunkConfig::default().with_min_size(8192);
    /// assert_eq!(config.min_size(), 8192);
    /// ```
    pub fn with_min_size(mut self, size: usize) -> Self {
        self.min_size = size;
        self
    }

    /// Sets the average/target chunk size.
    ///
    /// Note: This does not validate the configuration. Use [`ChunkConfig::validate`]
    /// to check if the configuration is valid.
    ///
    /// # Example
    ///
    /// ```
    /// use chunkrs::ChunkConfig;
    ///
    /// let config = ChunkConfig::default().with_avg_size(32768);
    /// assert_eq!(config.avg_size(), 32768);
    /// ```
    pub fn with_avg_size(mut self, size: usize) -> Self {
        self.avg_size = size;
        self
    }

    /// Sets the maximum chunk size.
    ///
    /// Note: This does not validate the configuration. Use [`ChunkConfig::validate`]
    /// to check if the configuration is valid.
    ///
    /// # Example
    ///
    /// ```
    /// use chunkrs::ChunkConfig;
    ///
    /// let config = ChunkConfig::default().with_max_size(131072);
    /// assert_eq!(config.max_size(), 131072);
    /// ```
    pub fn with_max_size(mut self, size: usize) -> Self {
        self.max_size = size;
        self
    }

    /// Sets the normalization level for mask generation.
    ///
    /// Higher levels produce more predictable chunk sizes by making the mask
    /// transition more aggressive. Valid range is 0-31.
    ///
    /// Note: This does not validate the configuration. Use [`ChunkConfig::validate`]
    /// to check if the configuration is valid.
    ///
    /// # Example
    ///
    /// ```
    /// use chunkrs::ChunkConfig;
    ///
    /// let config = ChunkConfig::default().with_normalization_level(2);
    /// assert_eq!(config.normalization_level(), 2);
    /// ```
    pub fn with_normalization_level(mut self, level: u8) -> Self {
        self.normalization_level = level;
        self
    }

    /// Sets the hash configuration.
    ///
    /// # Example
    ///
    /// ```
    /// use chunkrs::{ChunkConfig, HashConfig};
    ///
    /// let config = ChunkConfig::default()
    ///     .with_hash_config(HashConfig::enabled());
    /// ```
    pub fn with_hash_config(mut self, config: HashConfig) -> Self {
        self.hash_config = config;
        self
    }

    /// Sets the key for keyed gear table generation.
    ///
    /// When a key is set, the gear table is hashed with this key using BLAKE3,
    /// preventing adversarial chunk boundary manipulation attacks. This is useful
    /// for public-facing deduplication services.
    ///
    /// This requires the `keyed-cdc` feature flag.
    ///
    /// # Arguments
    ///
    /// * `key` - A 32-byte key, or None to disable keyed mode
    ///
    /// # Example
    ///
    /// ```ignore
    /// use chunkrs::ChunkConfig;
    ///
    /// let key = [0u8; 32];
    /// let config = ChunkConfig::default().with_keyed_gear_table(Some(key));
    /// ```
    #[cfg(feature = "keyed-cdc")]
    pub fn with_keyed_gear_table(mut self, key: Option<[u8; 32]>) -> Self {
        self.key = key;
        self
    }

    /// Returns the minimum chunk size.
    pub fn min_size(&self) -> usize {
        self.min_size
    }

    /// Returns the average/target chunk size.
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

    /// Returns the key for keyed gear table, if set.
    ///
    /// This requires the `keyed-cdc` feature flag.
    #[cfg(feature = "keyed-cdc")]
    pub fn keyed_gear_table_key(&self) -> Option<[u8; 32]> {
        self.key
    }

    /// Validates the current configuration.
    ///
    /// Returns an error if the configuration is invalid.
    ///
    /// # Example
    ///
    /// ```
    /// use chunkrs::ChunkConfig;
    ///
    /// let config = ChunkConfig::default().with_min_size(0);
    /// assert!(config.validate().is_err());
    /// ```
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

/// Configuration for chunk hashing behavior.
///
/// `HashConfig` controls whether BLAKE3 cryptographic hashes are computed
/// for each chunk. Hashing is enabled by default.
///
/// # Example
///
/// ```
/// use chunkrs::HashConfig;
///
/// // Enable hashing
/// let config = HashConfig::enabled();
///
/// // Disable hashing
/// let config = HashConfig::disabled();
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HashConfig {
    /// Whether to compute BLAKE3 hashes for chunks.
    pub enabled: bool,
}

impl HashConfig {
    /// Creates a new hash configuration.
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to enable hashing
    pub const fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    /// Enables hashing.
    ///
    /// # Example
    ///
    /// ```
    /// use chunkrs::HashConfig;
    ///
    /// let config = HashConfig::enabled();
    /// assert!(config.enabled);
    /// ```
    pub const fn enabled() -> Self {
        Self { enabled: true }
    }

    /// Disables hashing.
    ///
    /// # Example
    ///
    /// ```
    /// use chunkrs::HashConfig;
    ///
    /// let config = HashConfig::disabled();
    /// assert!(!config.enabled);
    /// ```
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
        assert!(
            ChunkConfig::new(32768, 16384, 65536).is_err(),
            "min > avg should fail"
        );
        assert!(
            ChunkConfig::new(4096, 65536, 16384).is_err(),
            "avg > max should fail"
        );
    }

    #[test]
    fn test_chunk_config_invalid_non_power_of_two() {
        assert!(
            ChunkConfig::new(5, 16, 64).is_err(),
            "Non-power-of-2 min_size should fail"
        );
        assert!(
            ChunkConfig::new(4, 17, 64).is_err(),
            "Non-power-of-2 avg_size should fail"
        );
        assert!(
            ChunkConfig::new(4, 16, 65).is_err(),
            "Non-power-of-2 max_size should fail"
        );
    }

    #[test]
    fn test_hash_config_default() {
        let config = HashConfig::default();
        assert!(config.enabled, "Hashing should be enabled by default");
    }

    #[test]
    fn test_hash_config_enabled() {
        let config = HashConfig::enabled();
        assert!(config.enabled);
    }

    #[test]
    fn test_hash_config_disabled() {
        let config = HashConfig::disabled();
        assert!(!config.enabled);
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
        assert!(
            config.validate().is_err(),
            "Validation should catch invalid config"
        );
    }
}
