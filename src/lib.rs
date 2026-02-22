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
//! # Quick Start
//!
//! ```
//! use chunkrs::{Chunker, ChunkConfig};
//! use bytes::Bytes;
//!
//! fn main() {
//!     // Create a chunker with default configuration
//!     let mut chunker = Chunker::new(ChunkConfig::default());
//!
//!     // Process data in streaming fashion
//!     let data = Bytes::from("hello world, this is some data to chunk");
//!     let (chunks, _pending) = chunker.push(data);
//!
//!     // Get any final incomplete chunk
//!     if let Some(final_chunk) = chunker.finish() {
//!         println!("Final chunk: {} bytes", final_chunk.len());
//!     }
//!
//!     // Process all chunks
//!     for chunk in chunks {
//!         println!("Chunk: offset={:?}, len={}, hash={:?}",
//!                  chunk.offset, chunk.len(), chunk.hash);
//!     }
//! }
//! ```
//!
//! # Streaming API
//!
//! The streaming API is designed for processing data of arbitrary size without
//! loading everything into memory at once:
//!
//! ```
//! use chunkrs::{Chunker, ChunkConfig};
//! use bytes::Bytes;
//!
//! fn main() {
//!     let mut chunker = Chunker::new(ChunkConfig::default());
//!     let mut pending = Bytes::new();
//!
//!     // Feed data in batches of any size
//!     let data = vec![
//!         Bytes::from(&b"first part"[..]),
//!         Bytes::from(&b" second part"[..]),
//!         Bytes::from(&b" final part"[..]),
//!     ];
//!
//!     for batch in data {
//!         // Combine pending bytes from previous call with new batch
//!         let input = if pending.is_empty() {
//!             batch
//!         } else {
//!             let mut combined = Vec::with_capacity(pending.len() + batch.len());
//!             combined.extend_from_slice(&pending);
//!             combined.extend_from_slice(&batch);
//!             Bytes::from(combined)
//!         };
//!
//!         let (chunks, leftover) = chunker.push(input);
//!
//!         // Process complete chunks
//!         for chunk in chunks {
//!             println!("Chunk: {} bytes", chunk.len());
//!         }
//!
//!         // Keep leftover for next iteration
//!         pending = leftover;
//!     }
//!
//!     // Finalize stream to get any remaining data
//!     if let Some(final_chunk) = chunker.finish() {
//!         println!("Final chunk: {} bytes", final_chunk.len());
//!     }
//! }
//! ```
//!
//! ## Determinism
//!
//! Identical byte streams produce identical chunk boundaries, regardless of:
//! - How many bytes are pushed at once (1 byte vs 1MB)
//! - Call timing
//! - Number of `push()` calls
//!
//! This makes chunk boundaries stable across different execution strategies.
//!
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
//!
//! // Builder pattern for incremental configuration
//! let config = ChunkConfig::default()
//!     .with_min_size(8192)
//!     .with_avg_size(32768)
//!     .with_max_size(131072)
//!     .with_hash_config(chunkrs::HashConfig::enabled());
//! # Ok::<(), ChunkError>(())
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

// Internal modules (implementation details)
// These are not exposed in the public API
mod cdc; // FastCDC rolling hash implementation
mod chunk;
mod chunker;
mod config;
mod error;
#[cfg(feature = "hash-blake3")]
mod hash; // BLAKE3 hasher wrapper
mod util; // Internal utility functions

//
// Public API surface
//
// The public API is intentionally minimal. Only essential types are exported
// to keep the surface area small and the API stable.
// Using flat API design: users access all types directly from crate root
//

/// Chunk type returned by the chunker.
///
/// Users receive `Chunk` objects from [`Chunker::push()`] and [`Chunker::finish()`]
/// and can access the chunk data, offset, and optional hash.
pub use chunk::{Chunk, ChunkHash};

/// Chunking engine for processing byte streams.
pub use chunker::Chunker;

/// Configuration options for chunking behavior.
pub use config::{ChunkConfig, HashConfig};

/// Error types for chunking operations.
pub use error::ChunkError;
