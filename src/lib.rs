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
//! - Manage files or file paths (user provides any [`std::io::Read`] source)
//! - Manage concurrency (user controls threading/async execution)
//! - Persist chunks (user decides storage backend)
//! - Assume storage devices (user manages I/O)
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
//! - **Feature: `async-io`** - Enables async streaming via `futures-io::AsyncRead`
//!
//! # Examples
//!
//! ## Synchronous API
//!
//! ```
//! use std::io::Cursor;
//! use chunkrs::{Chunker, ChunkConfig, ChunkError};
//!
//! fn main() -> Result<(), ChunkError> {
//!     let data = vec![0u8; 1024];
//!     let cursor = Cursor::new(data);
//!     let chunker = Chunker::new(ChunkConfig::default());
//!
//!     let mut chunk_count = 0;
//!     let mut total_bytes = 0;
//!     for chunk_result in chunker.chunk(cursor) {
//!         let chunk = chunk_result?;
//!         chunk_count += 1;
//!         total_bytes += chunk.len();
//!     }
//!     assert!(chunk_count > 0, "Should produce at least one chunk");
//!     assert_eq!(total_bytes, 1024, "All bytes should be chunked");
//!     Ok(())
//! }
//! ```
//!
//! ## Asynchronous API
//!
//! Requires the `async-io` feature:
//!
//! ```ignore
//! use futures_util::StreamExt;
//! use chunkrs::{chunk_async, ChunkConfig};
//! use futures_io::AsyncRead;
//!
//! async fn demo<R: AsyncRead + Unpin>(reader: R) -> Result<(), chunkrs::ChunkError> {
//!     let mut stream = chunk_async(reader, ChunkConfig::default());
//!
//!     let mut chunk_count = 0;
//!     while let Some(chunk) = stream.next().await {
//!         let chunk = chunk?;
//!         chunk_count += 1;
//!         println!("Chunk {}: {} bytes", chunk_count, chunk.len());
//!     }
//!     Ok(())
//! }
//! ```
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
mod buffer; // Thread-local buffer reuse for performance
mod cdc; // FastCDC rolling hash implementation
mod hash; // BLAKE3 hasher wrapper

// Async streaming support (feature-gated)
#[cfg(feature = "async-io")]
mod async_stream;

//
// Public API surface
//
// The public API is intentionally minimal. Only essential types are exported
// to keep the surface area small and the API stable.
//

/// Chunk types and related utilities.
pub use chunk::{Chunk, ChunkHash};

/// Chunking engine for processing byte streams.
pub use chunker::{ChunkIter, Chunker};

/// Configuration options for chunking behavior.
pub use config::{ChunkConfig, HashConfig};

/// Error types for chunking operations.
pub use error::ChunkError;

/// Async chunking support (requires `async-io` feature).
#[cfg(feature = "async-io")]
pub use async_stream::chunk_async;
