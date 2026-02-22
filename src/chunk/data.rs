//! Chunk data representation.
//!
//! This module defines the [`Chunk`] type, which represents a single content-defined
//! chunk with its data, optional offset in the original stream, and optional
//! cryptographic hash.

use bytes::Bytes;
use std::fmt;

use super::ChunkHash;

/// A content-defined chunk with metadata.
///
/// A `Chunk` represents a contiguous region of data from a byte stream that was
/// identified as a chunk boundary by the FastCDC algorithm. Each chunk contains:
///
/// - The actual data ([`Bytes`]) - zero-copy reference to chunk content
/// - Optional offset ([`Option<u64>`]) - position in the original stream
/// - Optional hash ([`Option<ChunkHash>`]) - BLAKE3 content hash if enabled
///
/// # Example
///
/// ```
/// use chunkrs::Chunk;
///
/// // Create a simple chunk
/// let chunk = Chunk::new(&b"hello world"[..]);
/// assert_eq!(chunk.len(), 11);
/// assert_eq!(chunk.start(), 0);
/// assert_eq!(chunk.end(), 11);
/// ```
///
/// # Builder Pattern
///
/// Use the builder methods to add metadata:
///
/// ```
/// use chunkrs::{Chunk, ChunkHash};
///
/// let hash = ChunkHash::new([0u8; 32]);
/// let chunk = Chunk::new(&b"test data"[..])
///     .set_offset(100)
///     .set_hash(hash);
///
/// assert_eq!(chunk.offset(), Some(100));
/// assert!(chunk.hash().is_some());
/// ```
///
/// # Zero-Copy Semantics
///
/// The `data` field uses [`Bytes`] from the `bytes` crate, which provides
/// zero-copy slicing and shared ownership. This is efficient for:
///
/// - Slicing chunks without copying data
/// - Sharing chunk data across multiple consumers
/// - Avoiding unnecessary allocations
#[derive(Debug, Clone)]
pub struct Chunk {
    /// The chunk data (may be owned or borrowed via Bytes).
    pub data: Bytes,

    /// The offset in the original stream (if available).
    ///
    /// This represents the byte position where this chunk starts in the
    /// original input stream. Set to `None` if the source doesn't track offsets.
    pub offset: Option<u64>,

    /// The content hash of this chunk (if computed).
    ///
    /// Contains the BLAKE3 hash of the chunk data when hashing is enabled
    /// via [`ChunkConfig`]. Set to `None` if hashing is disabled.
    pub hash: Option<ChunkHash>,
}

impl Chunk {
    /// Creates a new chunk with the given data.
    ///
    /// # Arguments
    ///
    /// * `data` - The chunk data. Accepts any type that can be converted to [`Bytes`],
    ///   including `&[u8]`, `Vec<u8>`, `String`, etc.
    ///
    /// # Example
    ///
    /// ```
    /// use chunkrs::Chunk;
    ///
    /// let chunk = Chunk::new(&b"hello"[..]);
    /// assert_eq!(chunk.len(), 5);
    /// ```
    pub fn new(data: impl Into<Bytes>) -> Self {
        Self {
            data: data.into(),
            offset: None,
            hash: None,
        }
    }

    /// Creates a new chunk with an offset.
    ///
    /// # Arguments
    ///
    /// * `data` - The chunk data
    /// * `offset` - The starting byte position in the original stream
    ///
    /// # Example
    ///
    /// ```
    /// use chunkrs::Chunk;
    ///
    /// let chunk = Chunk::with_offset(&b"hello"[..], 100);
    /// assert_eq!(chunk.offset(), Some(100));
    /// ```
    pub fn with_offset(data: impl Into<Bytes>, offset: u64) -> Self {
        Self {
            data: data.into(),
            offset: Some(offset),
            hash: None,
        }
    }

    /// Creates a new chunk with a hash.
    ///
    /// # Arguments
    ///
    /// * `data` - The chunk data
    /// * `hash` - The BLAKE3 hash of the chunk data
    ///
    /// # Example
    ///
    /// ```
    /// use chunkrs::{Chunk, ChunkHash};
    ///
    /// let hash = ChunkHash::new([0u8; 32]);
    /// let chunk = Chunk::with_hash(&b"hello"[..], hash);
    /// assert!(chunk.hash().is_some());
    /// ```
    pub fn with_hash(data: impl Into<Bytes>, hash: ChunkHash) -> Self {
        Self {
            data: data.into(),
            offset: None,
            hash: Some(hash),
        }
    }

    /// Sets the offset for this chunk.
    ///
    /// This is part of the builder pattern and returns `self` for chaining.
    ///
    /// # Example
    ///
    /// ```
    /// use chunkrs::Chunk;
    ///
    /// let chunk = Chunk::new(&b"test"[..]).set_offset(50);
    /// assert_eq!(chunk.offset(), Some(50));
    /// ```
    pub fn set_offset(mut self, offset: u64) -> Self {
        self.offset = Some(offset);
        self
    }

    /// Sets the hash for this chunk.
    ///
    /// This is part of the builder pattern and returns `self` for chaining.
    ///
    /// # Example
    ///
    /// ```
    /// use chunkrs::{Chunk, ChunkHash};
    ///
    /// let hash = ChunkHash::new([0u8; 32]);
    /// let chunk = Chunk::new(&b"test"[..]).set_hash(hash);
    /// assert!(chunk.hash().is_some());
    /// ```
    pub fn set_hash(mut self, hash: ChunkHash) -> Self {
        self.hash = Some(hash);
        self
    }

    /// Returns the length of the chunk data in bytes.
    ///
    /// # Example
    ///
    /// ```
    /// use chunkrs::Chunk;
    ///
    /// let chunk = Chunk::new(&b"hello world"[..]);
    /// assert_eq!(chunk.len(), 11);
    /// ```
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns `true` if the chunk contains no data.
    ///
    /// # Example
    ///
    /// ```
    /// use chunkrs::Chunk;
    ///
    /// let chunk = Chunk::new(&b""[..]);
    /// assert!(chunk.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Returns a reference to the chunk data.
    ///
    /// # Example
    ///
    /// ```
    /// use chunkrs::Chunk;
    ///
    /// let chunk = Chunk::new(&b"hello"[..]);
    /// assert_eq!(chunk.data().as_ref(), b"hello");
    /// ```
    pub fn data(&self) -> &Bytes {
        &self.data
    }

    /// Returns the offset in the original stream, if set.
    ///
    /// Returns `None` if the chunk was created without tracking its position.
    ///
    /// # Example
    ///
    /// ```
    /// use chunkrs::Chunk;
    ///
    /// let chunk = Chunk::with_offset(&b"test"[..], 100);
    /// assert_eq!(chunk.offset(), Some(100));
    /// ```
    pub fn offset(&self) -> Option<u64> {
        self.offset
    }

    /// Returns the hash of the chunk data, if computed.
    ///
    /// Returns `None` if hashing was disabled during chunking.
    ///
    /// # Example
    ///
    /// ```
    /// use chunkrs::{Chunk, ChunkHash};
    ///
    /// let hash = ChunkHash::new([0u8; 32]);
    /// let chunk = Chunk::new(&b"test"[..]).set_hash(hash);
    /// assert!(chunk.hash().is_some());
    /// ```
    pub fn hash(&self) -> Option<ChunkHash> {
        self.hash
    }

    /// Returns the start offset of the chunk.
    ///
    /// Returns the offset if set, otherwise returns `0`.
    ///
    /// # Example
    ///
    /// ```
    /// use chunkrs::Chunk;
    ///
    /// let chunk = Chunk::with_offset(&b"hello"[..], 100);
    /// assert_eq!(chunk.start(), 100);
    ///
    /// let chunk2 = Chunk::new(&b"test"[..]);
    /// assert_eq!(chunk2.start(), 0);
    /// ```
    pub fn start(&self) -> u64 {
        self.offset.unwrap_or(0)
    }

    /// Returns the end offset of the chunk (exclusive).
    ///
    /// This is calculated as `start() + len()`.
    ///
    /// # Example
    ///
    /// ```
    /// use chunkrs::Chunk;
    ///
    /// let chunk = Chunk::with_offset(&b"hello"[..], 100);
    /// assert_eq!(chunk.end(), 105); // 100 + 5
    /// ```
    pub fn end(&self) -> u64 {
        self.start() + self.data.len() as u64
    }

    /// Returns the chunk as a range `[start, end)`.
    ///
    /// Useful for indexing or slicing operations.
    ///
    /// # Example
    ///
    /// ```
    /// use chunkrs::Chunk;
    ///
    /// let chunk = Chunk::with_offset(&b"hello"[..], 100);
    /// assert_eq!(chunk.range(), 100..105);
    /// ```
    pub fn range(&self) -> std::ops::Range<u64> {
        self.start()..self.end()
    }

    /// Consumes the chunk and returns the underlying data.
    ///
    /// This is useful when you only need the data and want to avoid the overhead
    /// of extracting it from the `Bytes` field.
    ///
    /// # Example
    ///
    /// ```
    /// use chunkrs::Chunk;
    ///
    /// let chunk = Chunk::new(&b"hello"[..]);
    /// let data = chunk.into_data();
    /// assert_eq!(data.as_ref(), b"hello");
    /// ```
    pub fn into_data(self) -> Bytes {
        self.data
    }

    /// Splits the chunk into its constituent parts.
    ///
    /// Returns a tuple of `(data, hash)`.
    ///
    /// # Example
    ///
    /// ```
    /// use chunkrs::{Chunk, ChunkHash};
    ///
    /// let hash = ChunkHash::new([0u8; 32]);
    /// let chunk = Chunk::new(&b"test"[..]).set_hash(hash);
    ///
    /// let (data, chunk_hash) = chunk.into_parts();
    /// assert_eq!(data.as_ref(), b"test");
    /// assert!(chunk_hash.is_some());
    /// ```
    pub fn into_parts(self) -> (Bytes, Option<ChunkHash>) {
        (self.data, self.hash)
    }
}

impl From<Bytes> for Chunk {
    fn from(data: Bytes) -> Self {
        Self::new(data)
    }
}

impl From<Vec<u8>> for Chunk {
    fn from(data: Vec<u8>) -> Self {
        Self::new(data)
    }
}

impl fmt::Display for Chunk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Chunk({} bytes", self.len())?;
        if let Some(offset) = self.offset {
            write!(f, " @ {}", offset)?;
        }
        if let Some(hash) = self.hash {
            write!(f, ", hash={}", hash)?;
        }
        write!(f, ")")
    }
}

#[cfg(test)]
mod tests {
    use super::super::ChunkHash;
    use super::*;

    #[test]
    fn test_chunk_creation() {
        let chunk = Chunk::new(&b"hello"[..]);

        assert_eq!(chunk.len(), 5);
        assert!(!chunk.is_empty());
        assert_eq!(chunk.offset(), None);
        assert!(chunk.hash().is_none());
    }

    #[test]
    fn test_chunk_empty() {
        let chunk = Chunk::new(&b""[..]);

        assert!(chunk.is_empty());
        assert_eq!(chunk.len(), 0);
    }

    #[test]
    fn test_chunk_with_offset() {
        let chunk = Chunk::with_offset(&b"data"[..], 100);

        assert_eq!(chunk.offset(), Some(100));
        assert_eq!(chunk.start(), 100);
        assert_eq!(chunk.end(), 104);
    }

    #[test]
    fn test_chunk_with_hash() {
        let hash = ChunkHash::new([0x12; 32]);
        let chunk = Chunk::with_hash(&b"data"[..], hash);

        assert_eq!(chunk.hash(), Some(hash));
    }

    #[test]
    fn test_chunk_builder_pattern() {
        let hash = ChunkHash::new([0xAB; 32]);
        let chunk = Chunk::new(&b"test"[..]).set_offset(50).set_hash(hash);

        assert_eq!(chunk.len(), 4);
        assert_eq!(chunk.offset(), Some(50));
        assert_eq!(chunk.hash(), Some(hash));
    }

    #[test]
    fn test_chunk_range() {
        let chunk = Chunk::with_offset(&b"hello world"[..], 10);

        assert_eq!(chunk.range(), 10..21);
    }

    #[test]
    fn test_chunk_from_bytes() {
        let bytes = Bytes::from_static(b"test");
        let chunk: Chunk = bytes.into();

        assert_eq!(chunk.len(), 4);
    }

    #[test]
    fn test_chunk_display() {
        let chunk = Chunk::with_offset(&b"data"[..], 100);
        let s = format!("{}", chunk);

        assert!(s.contains("4 bytes"));
        assert!(s.contains("@ 100"));
    }

    #[test]
    fn test_chunk_into_data() {
        let original = Bytes::from(&b"test data"[..]);
        let chunk = Chunk::new(original.clone());
        let extracted = chunk.into_data();

        assert_eq!(extracted, original);
    }

    #[test]
    fn test_chunk_into_parts() {
        let hash = ChunkHash::new([0x99; 32]);
        let chunk = Chunk::new(&b"data"[..]).set_hash(hash);

        let (data, extracted_hash) = chunk.into_parts();

        assert_eq!(data.as_ref(), b"data");
        assert_eq!(extracted_hash, Some(hash));
    }
}
