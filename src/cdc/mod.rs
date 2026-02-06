//! Content-Defined Chunking (CDC) implementations.
//!
//! This module contains the core algorithms for identifying chunk boundaries
//! based on content patterns rather than fixed sizes.
//!
//! - [`FastCdc`] - FastCDC rolling hash implementation

mod fastcdc;

pub use fastcdc::FastCdc;
