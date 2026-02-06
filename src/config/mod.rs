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

/// Configuration for content-defined chunking behavior.
///
/// `ChunkConfig` controls the size constraints and hashing behavior for the
/// chunking process. It uses the FastCDC algorithm which requires:
///
/// - Minimum chunk size (`min_size`) - No chunk will be smaller than this
/// - Average chunk size (`avg_size`) - Target size for most chunks
/// - Maximum chunk size (`max_size`) - No chunk will exceed this
///
/// # Size Constraints
///
/// All sizes must be:
/// - Non-zero
/// - Powers of 2 (for optimal performance)
/// - Ordered: `min_size <= avg_size <= max_size`
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
///     .with_max_size(131072);
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

    /// Configuration for hashing behavior.
    hash_config: HashConfig,
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

        Ok(Self {
            min_size,
            avg_size,
            max_size,
            hash_config: HashConfig::default(),
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

    /// Returns the hash configuration.
    pub fn hash_config(&self) -> &HashConfig {
        &self.hash_config
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
            hash_config: HashConfig::default(),
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
    fn test_default_config() {
        let config = ChunkConfig::default();
        assert_eq!(config.min_size(), DEFAULT_MIN_CHUNK_SIZE);
        assert_eq!(config.avg_size(), DEFAULT_AVG_CHUNK_SIZE);
        assert_eq!(config.max_size(), DEFAULT_MAX_CHUNK_SIZE);
    }

    #[test]
    fn test_builder_pattern() {
        let config = ChunkConfig::default()
            .with_min_size(8192)
            .with_avg_size(32768)
            .with_max_size(131072);

        assert_eq!(config.min_size(), 8192);
        assert_eq!(config.avg_size(), 32768);
        assert_eq!(config.max_size(), 131072);
    }

    #[test]
    fn test_invalid_config_zero_size() {
        let result = ChunkConfig::new(0, 16384, 65536);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_config_min_gt_avg() {
        let result = ChunkConfig::new(32768, 16384, 65536);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_config_avg_gt_max() {
        let result = ChunkConfig::new(4096, 65536, 16384);
        assert!(result.is_err());
    }

    #[test]
    fn test_hash_config() {
        let config = HashConfig::default();
        assert!(config.enabled);

        let config = HashConfig::disabled();
        assert!(!config.enabled);
    }
}
