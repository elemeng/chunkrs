//! Async streaming support for chunking.
//!
//! This module provides asynchronous chunking using the `futures-io::AsyncRead`
//! trait, making it runtime-agnostic and compatible with tokio, async-std,
//! smol, and other async runtimes.
//!
//! - [`chunk_async`] - Creates an async stream of chunks from an async reader
//!
//! This module requires the `async-io` feature to be enabled.

mod stream;

pub use stream::chunk_async;
