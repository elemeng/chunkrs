//! Chunk data representation.

use bytes::Bytes;
use std::fmt;

use super::ChunkHash;

/// A content-defined chunk with metadata.
///
/// Contains:
/// - Data ([`Bytes`]) - zero-copy reference
/// - Offset ([`Option<u64>`]) - position in stream
/// - Hash ([`Option<ChunkHash>`]) - BLAKE3 hash if enabled
///
/// # Example
///
/// ```
/// use chunkrs::Chunk;
///
/// let chunk = Chunk::new(&b"hello world"[..]);
/// assert_eq!(chunk.len(), 11);
/// ```
///
/// # Builder Pattern
///
/// ```
/// use chunkrs::{Chunk, ChunkHash};
///
/// let hash = ChunkHash::new([0u8; 32]);
/// let chunk = Chunk::new(&b"test data"[..])
///     .set_offset(100)
///     .set_hash(hash);
/// ```
#[derive(Debug, Clone)]
pub struct Chunk {
    /// The chunk data.
    pub data: Bytes,

    /// The offset in the original stream.
    pub offset: Option<u64>,

    /// The content hash of this chunk.
    pub hash: Option<ChunkHash>,
}

impl Chunk {
    /// Creates a new chunk with the given data.
    pub fn new(data: impl Into<Bytes>) -> Self {
        Self {
            data: data.into(),
            offset: None,
            hash: None,
        }
    }

    /// Creates a new chunk with an offset.
    pub fn with_offset(data: impl Into<Bytes>, offset: u64) -> Self {
        Self {
            data: data.into(),
            offset: Some(offset),
            hash: None,
        }
    }

    /// Creates a new chunk with a hash.
    pub fn with_hash(data: impl Into<Bytes>, hash: ChunkHash) -> Self {
        Self {
            data: data.into(),
            offset: None,
            hash: Some(hash),
        }
    }

    /// Sets the offset for this chunk.
    pub fn set_offset(mut self, offset: u64) -> Self {
        self.offset = Some(offset);
        self
    }

    /// Sets the hash for this chunk.
    pub fn set_hash(mut self, hash: ChunkHash) -> Self {
        self.hash = Some(hash);
        self
    }

    /// Returns the length of the chunk data.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns `true` if the chunk contains no data.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Returns a reference to the chunk data.
    pub fn data(&self) -> &Bytes {
        &self.data
    }

    /// Returns the offset in the original stream, if set.
    pub fn offset(&self) -> Option<u64> {
        self.offset
    }

    /// Returns the hash of the chunk data, if computed.
    pub fn hash(&self) -> Option<ChunkHash> {
        self.hash
    }

    /// Returns the start offset of the chunk.
    pub fn start(&self) -> u64 {
        self.offset.unwrap_or(0)
    }

    /// Returns the end offset of the chunk (exclusive).
    pub fn end(&self) -> u64 {
        self.start() + self.data.len() as u64
    }

    /// Returns the chunk as a range `[start, end)`.
    pub fn range(&self) -> std::ops::Range<u64> {
        self.start()..self.end()
    }

    /// Consumes the chunk and returns the underlying data.
    pub fn into_data(self) -> Bytes {
        self.data
    }

    /// Splits the chunk into its constituent parts.
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