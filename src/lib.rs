//! # chunkrs
//!
//! Streaming Content-Defined Chunking (CDC) for Rust.
//!
//! `chunkrs` transforms a byte stream into content-defined chunks with optional
//! strong cryptographic hashes. It is designed as a small, composable primitive
//! for building higher-level systems:
//!
//! - **Delta synchronization** - Identify changed data regions efficiently
//! - **Deduplication** - Find and eliminate duplicate data across storage
//! - **Backup systems** - Optimize storage with content-defined boundaries
//! - **Content-addressable storage** - Store data by content hash for retrieval
//!
//! ## Design Philosophy
//!
//! This crate intentionally maintains a narrow scope and focuses on doing one thing well:
//! **transform byte streams into chunks**. It deliberately does not:
//!
//! - Manage files or file paths
//! - Manage concurrency or I/O
//! - Persist chunks
//! - Assume storage devices
//!
//! This design makes `chunkrs` a flexible building block that can be integrated
//! into any system architecture.
//!
//! ## Algorithm
//!
//! Uses the FastCDC algorithm for boundary detection:
//!
//! - **Deterministic**: Same input + same config → identical chunk boundaries
//! - **Adaptive**: Adjusts chunk sizes based on content patterns
//! - **Efficient**: Single-pass streaming with O(1) memory per chunk
//! - **Hashing**: Optional BLAKE3 hashes for content identity
//!
//! ## Features
//!
//! - **Feature: `hash-blake3`** (default) - Enables BLAKE3 cryptographic hashing
//!
//! # Examples
//!
//! ## Streaming API
//!
//! ```
//! use chunkrs::{Chunker, ChunkConfig};
//! use bytes::Bytes;
//!
//! fn main() {
//!     let mut chunker = Chunker::new(ChunkConfig::default());
//!     let mut pending = Bytes::new();
//!
//!     // Feed data
//!     for chunk in &[Bytes::from(&b"first"[..]), Bytes::from(&b"second"[..])] {
//!         let (chunks, leftover) = chunker.push(chunk);
//!         // Process chunks...
//!         pending = leftover;
//!     }
//!
//!     // Finalize stream
//!     if let Some(final_chunk) = chunker.finish() {
//!         // Process final chunk...
//!     }
//! }
//! ```
//! ## Configuration
//!
//! Customize chunk sizes to match your use case:
//!
//! ```
//! use chunkrs::{ChunkConfig, ChunkError};
//!
//! // Custom sizes: min 4KB, avg 16KB, max 64KB
//! let config = ChunkConfig::new(4096, 16384, 65536)?;
//! assert_eq!(config.min_size(), 4096);
//! # Ok::<(), ChunkError>(())
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

// Public modules
mod chunk;
mod chunker;
mod config;
mod error;

// Internal modules (implementation details)
mod cdc; // FastCDC rolling hash implementation
mod hash; // BLAKE3 hasher wrapper

//
// Public API surface
//
// The public API is intentionally minimal. Only essential types are exported
// to keep the surface area small and the API stable.
//

/// Chunk types and related utilities.
pub use chunk::{Chunk, ChunkHash};

/// Chunking engine for processing byte streams.
pub use chunker::Chunker;

/// Configuration options for chunking behavior.
pub use config::{ChunkConfig, HashConfig};

/// Error types for chunking operations.
pub use error::ChunkError;
