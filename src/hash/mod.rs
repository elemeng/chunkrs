//! Strong hash implementations for chunk identity.
//!
//! This module provides cryptographic hashing functionality for computing
//! content hashes of chunks. Currently supports BLAKE3 via the `hash-blake3`
//! feature.
//!
//! - [`Blake3Hasher`] - BLAKE3 hash implementation (requires `hash-blake3` feature)

#[cfg(feature = "hash-blake3")]
mod blake3;

// Re-export for use within the crate
#[cfg(feature = "hash-blake3")]
pub(crate) use blake3::Blake3Hasher;
