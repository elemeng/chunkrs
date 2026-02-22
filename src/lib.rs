//! # chunkrs
//!
//! Streaming Content-Defined Chunking (CDC) for Rust.
//!
//! Transforms byte streams into content-defined chunks with optional BLAKE3 hashes.
//! Designed as a small, composable primitive for:
//!
//! - Delta synchronization
//! - Deduplication
//! - Backup systems
//! - Content-addressable storage
//!
//! ## Design Philosophy
//!
//! Narrow scope: **transform byte streams into chunks**. Does not manage:
//! - Files or file paths
//! - Concurrency or I/O
//! - Chunk persistence
//! - Storage devices
//!
//! ## Algorithm
//!
//! FastCDC: deterministic, adaptive, single-pass streaming with O(1) memory per chunk.
//!
//! ## Features
//!
//! - `hash-blake3` (default) - BLAKE3 cryptographic hashing
//!
//! # Quick Start
//!
//! ```
//! use chunkrs::{Chunker, ChunkConfig};
//! use bytes::Bytes;
//!
//! let mut chunker = Chunker::new(ChunkConfig::default());
//! let (chunks, _pending) = chunker.push(Bytes::from("hello world"));
//!
//! if let Some(final_chunk) = chunker.finish() {
//!     println!("Final chunk: {} bytes", final_chunk.len());
//! }
//!
//! for chunk in chunks {
//!     println!("Chunk: offset={:?}, len={}, hash={:?}",
//!              chunk.offset, chunk.len(), chunk.hash);
//! }
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

// Internal modules
mod cdc;
mod chunk;
mod chunker;
mod config;
mod error;
#[cfg(feature = "hash-blake3")]
mod hash;
mod util;

// Public API (flat design)
pub use chunk::{Chunk, ChunkHash};
pub use chunker::Chunker;
pub use config::{ChunkConfig, HashConfig};
pub use error::ChunkError;
