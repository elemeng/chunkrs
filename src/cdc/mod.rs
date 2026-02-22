//! Content-Defined Chunking (CDC) algorithms.
//!
//! This module is private to the crate and not exposed in the public API.

mod fastcdc;
mod tables;

// Re-export for use within the crate (cdc module is private, so pub is crate-local)
pub use fastcdc::FastCdc;
