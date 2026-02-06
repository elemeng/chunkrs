//! Chunk types and related utilities.
//!
//! This module provides the core types for representing chunks of data:
//!
//! - [`Chunk`] - A content-defined chunk with data, optional offset, and optional hash
//! - [`ChunkHash`] - A 32-byte cryptographic hash for chunk identity
//!
//! Chunks are the primary output of the chunking process and contain all
//! metadata needed for downstream processing.

mod data;
mod hash;

pub use data::Chunk;
pub use hash::ChunkHash;
