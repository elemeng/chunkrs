//! Error types for chunkrs.
//!
//! This module defines the error type used throughout the crate.
//!
//! - [`ChunkError`] - Represents all possible errors during chunking

use std::fmt;

/// Errors that can occur during chunking operations.
///
/// `ChunkError` represents all possible error conditions that may occur while
/// chunking data.
///
/// # Variants
///
/// - [`ChunkError::InvalidConfig`] - Invalid configuration parameter
///
/// # Example
///
/// ```
/// use chunkrs::ChunkError;
///
/// fn handle_error(err: ChunkError) {
///     match err {
///         ChunkError::InvalidConfig { message } => eprintln!("Config error: {}", message),
///     }
/// }
/// ```
#[derive(Debug)]
pub enum ChunkError {
    /// Invalid configuration parameter.
    ///
    /// This error is raised when the chunking configuration is invalid, such as:
    /// - Zero or negative chunk sizes
    /// - Minimum size greater than average or average greater than maximum
    /// - Non-power-of-2 sizes (FastCDC requirement)
    InvalidConfig {
        /// Description of what was invalid.
        message: &'static str,
    },
}

impl fmt::Display for ChunkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChunkError::InvalidConfig { message } => {
                write!(f, "invalid config: {}", message)
            }
        }
    }
}

impl std::error::Error for ChunkError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display() {
        let err = ChunkError::InvalidConfig {
            message: "test error",
        };
        assert!(err.to_string().contains("invalid config"));
    }
}
