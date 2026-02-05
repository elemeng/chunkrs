//! Internal buffer management for zero-copy optimization.

mod pool;

pub(crate) use pool::Buffer;
