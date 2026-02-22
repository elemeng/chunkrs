//! Content-Defined Chunking (CDC) algorithms.
//!
//! This module implements the FastCDC algorithm for detecting content-defined
//! chunk boundaries.

mod fastcdc;
mod ultracdc;

// Re-export for use within the crate
pub(crate) use fastcdc::FastCdc;
