//! Error types for chunkrs.
//!
//! This module defines the error type used throughout the crate.
//!
//! - [`ChunkError`] - Represents all possible errors during chunking

use std::fmt;

/// Errors that can occur during chunking operations.
///
/// `ChunkError` represents all possible error conditions that may occur while
/// chunking data, including I/O errors and configuration errors.
///
/// # Variants
///
/// - [`ChunkError::Io`] - An I/O error occurred while reading input data
/// - [`ChunkError::ChunkTooLarge`] - The chunk size exceeded the maximum allowed limit
/// - [`ChunkError::InvalidConfig`] - Invalid configuration parameter
///
/// # Example
///
/// ```
/// use chunkrs::ChunkError;
/// use std::io;
///
/// fn handle_error(err: ChunkError) {
///     match err {
///         ChunkError::Io(io_err) => eprintln!("I/O error: {}", io_err),
///         ChunkError::InvalidConfig { message } => eprintln!("Config error: {}", message),
///         _ => eprintln!("Other error"),
///     }
/// }
/// ```
#[derive(Debug)]
pub enum ChunkError {
    /// An I/O error occurred while reading input data.
    Io(std::io::Error),

    /// The chunk size exceeded the maximum allowed limit.
    ///
    /// This error is raised if a chunk would exceed the configured maximum size.
    /// This should not normally occur as the chunker enforces max_size constraints.
    ChunkTooLarge {
        /// The actual size that was attempted.
        actual: usize,
        /// The maximum allowed size.
        max: usize,
    },

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
            ChunkError::Io(e) => write!(f, "io error: {}", e),
            ChunkError::ChunkTooLarge { actual, max } => {
                write!(f, "chunk too large: {} bytes (max {})", actual, max)
            }
            ChunkError::InvalidConfig { message } => {
                write!(f, "invalid config: {}", message)
            }
        }
    }
}

impl std::error::Error for ChunkError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ChunkError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for ChunkError {
    fn from(e: std::io::Error) -> Self {
        ChunkError::Io(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "test");
        let err: ChunkError = io_err.into();
        matches!(err, ChunkError::Io(_));
    }

    #[test]
    fn test_display() {
        let err = ChunkError::ChunkTooLarge {
            actual: 100,
            max: 50,
        };
        assert!(err.to_string().contains("chunk too large"));
    }
}
