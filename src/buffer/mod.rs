//! Internal buffer management for zero-copy optimization.
//!
//! This module provides a thread-local buffer pool to minimize allocations
//! during chunking operations. It is an implementation detail and not part
//! of the public API.

mod pool;

pub(crate) use pool::Buffer;
