//! Error types for chunkrs.

use std::fmt;

/// Errors that can occur during chunking operations.
#[derive(Debug)]
pub enum ChunkError {
    /// An I/O error occurred while reading input data.
    Io(std::io::Error),

    /// The chunk size exceeded the maximum allowed limit.
    ChunkTooLarge {
        /// The actual size that was attempted.
        actual: usize,
        /// The maximum allowed size.
        max: usize,
    },

    /// Invalid configuration parameter.
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
