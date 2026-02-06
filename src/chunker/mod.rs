//! Chunker implementation for processing byte streams.
//!
//! This module provides the synchronous chunking API:
//!
//! - [`Chunker`] - High-level API for chunking byte streams
//! - [`ChunkIter`] - Iterator that yields chunks from a reader
//!
//! The chunker uses the FastCDC algorithm to identify content-defined
//! boundaries in a streaming fashion.

mod iter;

pub use iter::{ChunkIter, Chunker};
