//! Chunking engine for processing byte streams.
//!
//! This module provides the streaming chunking API:
//!
//! - [`Chunker`] - Stateful CDC engine with `push()`/`finish()` API
//!
//! The chunker uses the FastCDC algorithm to identify content-defined
//! boundaries in a streaming fashion, ensuring deterministic results
//! regardless of input batch sizes.

mod engine;

// Re-export for use within the crate
// Since chunker module is private, pub is crate-local
pub use engine::Chunker;
