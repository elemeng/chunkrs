//! Chunk types.
//!
//! - [`Chunk`] - Content-defined chunk with data, offset, hash
//! - [`ChunkHash`] - 32-byte cryptographic hash

mod data;
mod hash;

pub use data::Chunk;
pub use hash::ChunkHash;
