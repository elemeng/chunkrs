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
//! ```no_run
//! use std::fs::File;
//! use chunkrs::{Chunker, ChunkConfig, ChunkError};
//!
//! fn main() -> Result<(), ChunkError> {
//!     let file = File::open("data.bin")?;
//!     let chunker = Chunker::new(ChunkConfig::default());
//!
//!     for chunk in chunker.chunk(file) {
//!         let chunk = chunk?;
//!         println!("chunk {} bytes", chunk.data.len());
//!     }
//!     Ok(())
//! }
//! ```
//!
//! # Async (feature = "async-io")
//!
//! ```ignore
//! use futures_util::StreamExt;
//! use chunkrs::{chunk_async, ChunkConfig};
//! use tokio::io::AsyncRead;
//!
//! async fn demo<R: AsyncRead + Unpin>(reader: R) -> Result<(), chunkrs::ChunkError> {
//!     let mut stream = chunk_async(reader, ChunkConfig::default());
//!
//!     while let Some(chunk) = stream.next().await {
//!         let chunk = chunk?;
//!         println!("chunk {}", chunk.data.len());
//!     }
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
