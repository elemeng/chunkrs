//! Strong hash implementations for chunk identity.

#[cfg(feature = "hash-blake3")]
mod blake3;

#[cfg(feature = "hash-blake3")]
pub use blake3::Blake3Hasher;
