//! chunkrs
//!
//! Streaming Content-Defined Chunking (CDC) for Rust.
//!
//! `chunkrs` transforms a byte stream into content-defined chunks with optional
//! strong hashes. It is designed as a small, composable primitive for:
//!
//! - delta synchronization
//! - deduplication
//! - backup systems
//! - content-addressable storage
//!
//! The crate intentionally:
//! - does NOT manage files or paths
//! - does NOT manage concurrency
//! - does NOT persist chunks
//! - does NOT assume storage devices
//!
//! It only does one thing: **Read bytes → yield chunks**
//!
//! # Sync
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
//!     for chunk in chunker.chunk(cursor) {
//!         let chunk = chunk?;
//!         chunk_count += 1;
//!     }
//!     assert!(chunk_count > 0, "Should produce at least one chunk");
//!     Ok(())
//! }
//! ```
//!
//! # Async (feature = "async-io")
//!
//! ```ignore
//! use futures_util::StreamExt;
//! use chunkrs::{chunk_async, ChunkConfig};
//! use futures_io::AsyncRead;
//!
//! async fn demo<R: AsyncRead + Unpin>(reader: R) -> Result<(), chunkrs::ChunkError> {
//!     let mut stream = chunk_async(reader, ChunkConfig::default());
//!     let mut chunk_count = 0;
//!
//!     while let Some(chunk) = stream.next().await {
//!         let chunk = chunk?;
//!         chunk_count += 1;
//!     }
//!     assert!(chunk_count > 0, "Should produce at least one chunk");
//!     Ok(())
//! }
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod chunk;
mod chunker;
mod config;
mod error;

mod buffer; // internal (thread-local reuse)
mod cdc; // internal fastcdc impl
mod hash; // internal blake3 impl

#[cfg(feature = "async-io")]
mod async_stream;

//
// Public surface (intentionally tiny)
//

pub use chunk::{Chunk, ChunkHash};
pub use chunker::{ChunkIter, Chunker};
pub use config::{ChunkConfig, HashConfig};
pub use error::ChunkError;

#[cfg(feature = "async-io")]
pub use async_stream::chunk_async;
