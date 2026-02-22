//! Internal utility functions and helpers.
//!
//! This module contains small helper functions used throughout the crate.
//! It is an implementation detail and not part of the public API.

use bytes::Bytes;

/// Combines two byte slices into a new Bytes object.
///
/// This is used when pending bytes need to be combined with new data
/// to form a complete chunk.
pub(crate) fn combine_bytes(a: &Bytes, b: &[u8]) -> Bytes {
    let mut combined = Vec::with_capacity(a.len() + b.len());
    combined.extend_from_slice(a);
    combined.extend_from_slice(b);
    Bytes::from(combined)
}
