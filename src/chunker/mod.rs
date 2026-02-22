//! Chunking engine for processing byte streams.
//!
//! - [`Chunker`] - Stateful CDC engine with `push()`/`finish()` API

mod engine;

// Re-export for use within the crate
pub use engine::Chunker;